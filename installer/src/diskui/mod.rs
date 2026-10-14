//! `nixdots-disk` — a TUI imitation of macOS Disk Utility.
//!
//! Toolbar of two-line action blocks, a 30-column sidebar tree, a detail pane
//! and modal "sheets" that drop below the toolbar. Runs in dry-run with sample
//! data by default; `--real` executes the destructive commands.

mod data;
mod sheets;

use data::{Loaded, Node};

use crate::theme::{self, Palette};
use crate::widgets;
use color_eyre::eyre::Result;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

const SIDEBAR_WIDTH: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Volume,
    FirstAid,
    Partition,
    Erase,
    Restore,
    Unmount,
    Info,
}

impl Action {
    const ALL: [Action; 7] = [
        Action::Volume,
        Action::FirstAid,
        Action::Partition,
        Action::Erase,
        Action::Restore,
        Action::Unmount,
        Action::Info,
    ];

    fn icon(self) -> &'static str {
        match self {
            Action::Volume => "+ −",
            Action::FirstAid => "✚",
            Action::Partition => "◔",
            Action::Erase => "⌫",
            Action::Restore => "↺",
            Action::Unmount => "⏏",
            Action::Info => "ⓘ",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::Volume => "Volumen",
            Action::FirstAid => "Auxilios",
            Action::Partition => "Particionar",
            Action::Erase => "Borrar",
            Action::Restore => "Restaurar",
            Action::Unmount => "Desmontar",
            Action::Info => "Info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Toolbar,
    Sidebar,
}

pub struct DiskApp {
    palette: Palette,
    dry_run: bool,
    loaded: Loaded,
    nodes: Vec<Node>,
    selected: usize,
    focus: Focus,
    action: Action,
    sheet: Option<sheets::Sheet>,
    show_commands: bool,
    status: Option<String>,
    should_quit: bool,
}

impl DiskApp {
    fn new(dry_run: bool) -> Self {
        let palette = if theme::console::is_linux_console() {
            Palette::console()
        } else {
            Palette::truecolor()
        };
        let loaded = data::load(dry_run);
        let mut app = Self {
            palette,
            dry_run,
            loaded,
            nodes: Vec::new(),
            selected: 0,
            focus: Focus::Toolbar,
            action: Action::Info,
            sheet: None,
            show_commands: false,
            status: None,
            should_quit: false,
        };
        app.rebuild();
        app
    }

    fn rebuild(&mut self) {
        self.nodes = data::flatten(&self.loaded);
        if self.selected >= self.nodes.len() {
            self.selected = self.nodes.len().saturating_sub(1);
        }
    }

    fn selected_node(&self) -> Option<&Node> {
        self.nodes.get(self.selected)
    }

    fn action_enabled(&self, action: Action) -> bool {
        let node = self.selected_node();
        match action {
            Action::Partition => node.map(|n| n.is_disk && !n.protected).unwrap_or(false),
            Action::Erase => node.map(|n| !n.is_disk && !n.protected).unwrap_or(false),
            Action::Unmount => node.map(|n| !n.is_disk).unwrap_or(false),
            Action::Volume | Action::FirstAid | Action::Restore | Action::Info => {
                node.map(|n| !n.protected).unwrap_or(false)
            }
        }
    }

    fn commands(&self) -> Vec<String> {
        let node = self.selected_node();
        let dev = node.map(|n| format!("/dev/{}", n.label)).unwrap_or_else(|| "/dev/xxx".into());
        let mut commands = vec![format!(
            "lsblk -J -b -o NAME,SIZE,FSTYPE,LABEL,MOUNTPOINTS,TYPE,MODEL {dev}"
        )];
        if node.map(|n| n.is_disk).unwrap_or(false) {
            commands.push(format!("sgdisk --print {dev}"));
        }
        if let Some(sheet) = &self.sheet {
            commands.extend(sheet.commands());
        }
        commands
    }

    fn run(&mut self, terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.draw(f))?;
            if ratatui::crossterm::event::poll(std::time::Duration::from_millis(120))? {
                if let Event::Key(key) = ratatui::crossterm::event::read()? {
                    if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                        self.handle(key);
                    }
                }
            }
        }
        Ok(())
    }

    fn handle(&mut self, key: KeyEvent) {
        if self.sheet.is_some() {
            let result = sheets::key(self.sheet.as_mut().unwrap(), key, self.dry_run);
            if result.closed {
                self.sheet = None;
                self.status = result.message;
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('v') => self.show_commands = !self.show_commands,
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Toolbar => Focus::Sidebar,
                    Focus::Sidebar => Focus::Toolbar,
                };
            }
            KeyCode::Left | KeyCode::Right if self.focus == Focus::Toolbar => {
                let idx = Action::ALL.iter().position(|a| *a == self.action).unwrap_or(0);
                let next = if key.code == KeyCode::Right {
                    (idx + 1) % Action::ALL.len()
                } else {
                    (idx + Action::ALL.len() - 1) % Action::ALL.len()
                };
                self.action = Action::ALL[next];
            }
            KeyCode::Up if self.focus == Focus::Sidebar => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Down if self.focus == Focus::Sidebar => {
                if self.selected + 1 < self.nodes.len() {
                    self.selected += 1;
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if self.focus == Focus::Toolbar && self.action_enabled(self.action) {
                    self.open_sheet();
                } else if self.focus == Focus::Sidebar {
                    self.focus = Focus::Toolbar;
                }
            }
            _ => {}
        }
    }

    fn open_sheet(&mut self) {
        let Some(node) = self.selected_node() else { return };
        match self.action {
            Action::Partition if node.is_disk => {
                self.sheet = Some(sheets::Sheet::partition(&node.label, node.size));
            }
            Action::Erase if !node.is_disk => {
                self.sheet = Some(sheets::Sheet::erase(&node.label, &node.fstype));
            }
            Action::Info => {
                self.sheet = Some(sheets::Sheet::info(&node.label, node.size, &node.fstype));
            }
            Action::Unmount => {
                self.status = Some(format!("desmontar /dev/{}", node.label));
            }
            _ => {
                self.status = Some(format!("{} no disponible en simulación", self.action.label()));
            }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(Block::default().style(Style::default().bg(self.palette.reset)), area);

        let toolbar_h = 3;
        let rows = Layout::vertical([Constraint::Length(toolbar_h), Constraint::Min(0)]).split(area);
        self.draw_toolbar(f, rows[0]);

        let cols = Layout::horizontal([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)]).split(rows[1]);
        self.draw_sidebar(f, cols[0]);
        self.draw_detail(f, cols[1]);

        if self.show_commands {
            let cmd_area = Rect::new(area.x + 1, area.bottom().saturating_sub(6), area.width.saturating_sub(2), 6);
            self.draw_commands(f, cmd_area);
        }

        if let Some(sheet) = &self.sheet {
            let backdrop = Rect::new(area.x, area.y + toolbar_h, area.width, area.height.saturating_sub(toolbar_h));
            widgets::dim(f, backdrop, &self.palette);
            let sheet_area = Rect::new(area.x + 4, area.y + toolbar_h + 1, area.width.saturating_sub(8), area.height.saturating_sub(toolbar_h + 3));
            sheets::draw(f, sheet, sheet_area, &self.palette, self.dry_run);
        }

        if let Some(status) = &self.status {
            let row = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
            f.render_widget(
                Paragraph::new(Span::styled(status.clone(), Style::default().fg(self.palette.warning)))
                    .alignment(Alignment::Center),
                row,
            );
        }
    }

    fn draw_toolbar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(self.palette.border));
        let inner = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), area.height);
        f.render_widget(block, area);

        let mut x = inner.x;
        for action in Action::ALL {
            let enabled = self.action_enabled(action);
            let active = self.focus == Focus::Toolbar && action == self.action && enabled;
            let width = 12u16;
            let rect = Rect::new(x, inner.y, width.min(inner.right().saturating_sub(x)), 2);
            if rect.width == 0 {
                break;
            }
            let style = if active {
                Style::default().fg(self.palette.white).bg(self.palette.primary).add_modifier(Modifier::BOLD)
            } else if enabled {
                Style::default().fg(self.palette.text)
            } else {
                Style::default().fg(self.palette.muted)
            };
            let text = vec![
                Line::from(Span::styled(action.icon().to_string(), style)).alignment(Alignment::Center),
                Line::from(Span::styled(action.label().to_string(), style)).alignment(Alignment::Center),
            ];
            f.render_widget(Paragraph::new(text).style(style), rect);
            x += width;
        }
    }

    fn draw_sidebar(&self, f: &mut Frame, area: Rect) {
        f.render_widget(Block::default().style(Style::default().bg(self.palette.well)), area);
        let mut lines: Vec<Line> = Vec::new();
        let mut last_section: Option<bool> = None;
        for (i, node) in self.nodes.iter().enumerate() {
            if node.depth == 0 {
                let section = node.removable;
                if last_section != Some(section) {
                    last_section = Some(section);
                    lines.push(Line::from(Span::styled(
                        format!(" {}", if section { "Externo" } else { "Interno" }),
                        Style::default().fg(self.palette.muted).add_modifier(Modifier::BOLD),
                    )));
                }
            }
            let selected = i == self.selected && self.focus == Focus::Sidebar;
            let indent = "  ".repeat(node.depth);
            let marker = if node.depth == 0 && node.removable { "⏏" } else { "" };
            let text = format!("{indent}{} {}{marker}", node.icon, node.label);
            lines.push(widgets::list_row(&self.palette, selected, text, None));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn draw_detail(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(self.palette.border));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let Some(node) = self.selected_node() else {
            return;
        };

        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(inner);

        // 1. Icon + name + type.
        let icon = ["     ", " ▄▄▄ ", "▐███▌", "▐███▌", " ▀▀▀ "];
        let icon_lines: Vec<Line> = icon
            .iter()
            .map(|l| Line::from(Span::styled(*l, Style::default().fg(self.palette.primary))))
            .collect();
        let icon_area = Layout::horizontal([Constraint::Length(7), Constraint::Min(0)]).split(chunks[0]);
        f.render_widget(Paragraph::new(icon_lines), icon_area[0]);
        let header = vec![
            Line::from(Span::styled(node.label.clone(), Style::default().fg(self.palette.text).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled(format!("{} · {}", node.fstype, data::human(node.size)), Style::default().fg(self.palette.muted))),
        ];
        f.render_widget(Paragraph::new(header), icon_area[1]);

        // 2. Capacity note.
        let note = if node.fstype == "btrfs" {
            "COMPARTIDO POR VARIOS SUBVOLÚMENES".to_string()
        } else {
            "CAPACIDAD TOTAL".to_string()
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(note, Style::default().fg(self.palette.secondary)))),
            chunks[1],
        );

        // 3. Usage bar + legend.
        let (used, total) = self.loaded.usage_for(&node.path);
        let width = chunks[2].width.saturating_sub(2) as usize;
        let filled = if total == 0 { 0 } else { ((used as f64 / total as f64) * width as f64).round() as usize };
        let bar = Line::from(vec![
            Span::styled("█".repeat(filled), Style::default().fg(self.palette.primary)),
            Span::styled("█".repeat(width.saturating_sub(filled)), Style::default().fg(self.palette.row_bg)),
        ]);
        let legend = Line::from(vec![
            Span::styled("● Usado ", Style::default().fg(self.palette.primary)),
            Span::styled(data::human(used), Style::default().fg(self.palette.muted)),
            Span::raw("   "),
            Span::styled("● Snapshots ", Style::default().fg(self.palette.snapshot)),
            Span::styled("0 B", Style::default().fg(self.palette.muted)),
            Span::raw("   "),
            Span::styled("● Libre ", Style::default().fg(self.palette.muted)),
            Span::styled(data::human(total.saturating_sub(used)), Style::default().fg(self.palette.muted)),
        ]);
        f.render_widget(Paragraph::new(vec![bar, legend]), chunks[2]);

        // 4. Two-column info table.
        self.draw_table(f, chunks[3], node);
    }

    fn draw_table(&self, f: &mut Frame, area: Rect, node: &Node) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.border));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mid = inner.x + inner.width / 2;
        f.render_widget(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(self.palette.border)),
            Rect::new(mid, inner.y, 1, inner.height),
        );

        let info = self.loaded.info_for(&node.path);
        let left = info
            .iter()
            .take((inner.height as usize).div_ceil(2))
            .map(|(k, v)| Self::kv(&self.palette, k, v))
            .collect::<Vec<_>>();
        let right = info
            .iter()
            .skip((inner.height as usize).div_ceil(2))
            .map(|(k, v)| Self::kv(&self.palette, k, v))
            .collect::<Vec<_>>();
        f.render_widget(
            Paragraph::new(left),
            Rect::new(inner.x + 1, inner.y, inner.width / 2 - 1, inner.height),
        );
        f.render_widget(
            Paragraph::new(right),
            Rect::new(mid + 1, inner.y, inner.width / 2 - 1, inner.height),
        );
    }

    fn kv(palette: &Palette, key: &str, value: &str) -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{key}: "), Style::default().fg(palette.muted)),
            Span::styled(value.to_string(), Style::default().fg(palette.text)),
        ])
    }

    fn draw_commands(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.palette.border))
            .title(Span::styled(" comandos ", Style::default().fg(self.palette.muted)));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let lines: Vec<Line> = self
            .commands()
            .iter()
            .map(|c| Line::from(Span::styled(format!("$ {c}"), Style::default().fg(self.palette.muted))))
            .collect();
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
    }
}

/// Entry point used by the `nixdots-disk` binary and the installer's Disk step.
pub fn run(dry_run: bool) -> Result<()> {
    use ratatui::crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::crossterm::execute;
    use ratatui::backend::CrosstermBackend;

    let _console = theme::console::Guard::activate();
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout))?;

    let mut app = DiskApp::new(dry_run);
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}
