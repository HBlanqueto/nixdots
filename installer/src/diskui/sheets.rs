//! Modal sheets for the disk utility: Partition, Erase and Info.

use crate::runner::{self, Cmd};
use crate::theme::Palette;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

pub struct Outcome {
    pub closed: bool,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct Part {
    name: String,
    format: String,
    size_gb: f64,
    mount: String,
}

pub struct PartitionSheet {
    device: String,
    parts: Vec<Part>,
    selected: usize,
    focus: usize,
    confirm: bool,
}

pub struct EraseSheet {
    device: String,
    formats: Vec<&'static str>,
    index: usize,
    focus: usize,
    confirm: bool,
}

pub struct InfoSheet {
    device: String,
    size: u64,
    kind: String,
    focus: usize,
}

pub enum Sheet {
    Partition(PartitionSheet),
    Erase(EraseSheet),
    Info(InfoSheet),
}

impl Sheet {
    pub fn partition(device: &str, size: u64) -> Self {
        let gb = size as f64 / 1_073_741_824.0;
        let parts = vec![
            Part {
                name: "EFI".into(),
                format: "fat32".into(),
                size_gb: 1.0,
                mount: "/boot".into(),
            },
            Part {
                name: "nixdots".into(),
                format: "btrfs".into(),
                size_gb: (gb - 1.0).max(1.0),
                mount: "/".into(),
            },
        ];
        Sheet::Partition(PartitionSheet {
            device: device.to_string(),
            parts,
            selected: 0,
            focus: 1,
            confirm: false,
        })
    }

    pub fn erase(device: &str, _kind: &str) -> Self {
        Sheet::Erase(EraseSheet {
            device: device.to_string(),
            formats: vec!["btrfs", "ext4", "xfs", "vfat"],
            index: 0,
            focus: 0,
            confirm: false,
        })
    }

    pub fn info(device: &str, size: u64, kind: &str) -> Self {
        Sheet::Info(InfoSheet {
            device: device.to_string(),
            size,
            kind: kind.to_string(),
            focus: 0,
        })
    }

    pub fn commands(&self) -> Vec<String> {
        match self {
            Sheet::Partition(sheet) => {
                let mut commands = vec![format!("sgdisk --zap-all /dev/{}", sheet.device)];
                for (i, part) in sheet.parts.iter().enumerate() {
                    let code = if part.format == "fat32" { "ef00" } else { "8300" };
                    commands.push(format!(
                        "sgdisk -n {}:0:+{:.0}G -t {}:{} -c {}:{} /dev/{}",
                        i + 1,
                        part.size_gb,
                        i + 1,
                        code,
                        i + 1,
                        part.name,
                        sheet.device
                    ));
                }
                commands
            }
            Sheet::Erase(sheet) => {
                let format = sheet.formats[sheet.index];
                vec![
                    format!("umount -R /dev/{} || true", sheet.device),
                    format!("wipefs -a /dev/{}", sheet.device),
                    format!("mkfs.{format} /dev/{}", sheet.device),
                ]
            }
            Sheet::Info(sheet) => vec![format!("lsblk -f /dev/{}", sheet.device)],
        }
    }

    fn title(&self) -> String {
        match self {
            Sheet::Partition(s) => format!("Particionar · {}", s.device),
            Sheet::Erase(s) => format!("Borrar · {}", s.device),
            Sheet::Info(s) => format!("Info · {}", s.device),
        }
    }

    fn destructive(&self) -> bool {
        matches!(self, Sheet::Erase(_))
    }

    fn confirm(&self) -> bool {
        match self {
            Sheet::Partition(s) => s.confirm,
            Sheet::Erase(s) => s.confirm,
            Sheet::Info(_) => false,
        }
    }
}

pub fn draw(f: &mut Frame, sheet: &Sheet, area: Rect, palette: &Palette, dry_run: bool) {
    let border = if sheet.destructive() {
        palette.error
    } else {
        palette.primary
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(format!(" {} ", sheet.title()), Style::default().fg(border)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    match sheet {
        Sheet::Partition(s) => draw_partition(f, s, inner, palette),
        Sheet::Erase(s) => draw_erase(f, s, inner, palette),
        Sheet::Info(s) => draw_info(f, s, inner, palette),
    }

    let buttons = sheet_buttons(sheet);
    let row = Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1);
    let focus = sheet_focus(sheet);
    widgets::render_buttons(f, row, palette, &buttons, focus);

    if sheet.confirm() {
        let row = Rect::new(inner.x, inner.bottom().saturating_sub(3), inner.width, 1);
        let text = if dry_run {
            "simulación: enter muestra los comandos · esc cancela"
        } else {
            "enter confirma y ejecuta · esc cancela"
        };
        f.render_widget(
            Paragraph::new(Span::styled(text, Style::default().fg(palette.warning)))
                .alignment(Alignment::Center),
            row,
        );
    }
}

fn draw_partition(f: &mut Frame, sheet: &PartitionSheet, area: Rect, palette: &Palette) {
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(4),
    ])
    .split(area);

    // Horizontal bar per partition.
    let total: f64 = sheet.parts.iter().map(|p| p.size_gb).sum();
    let width = rows[0].width as usize;
    let mut spans = Vec::new();
    for (i, part) in sheet.parts.iter().enumerate() {
        let cells = ((part.size_gb / total) * width as f64).round() as usize;
        let color = if i == sheet.selected {
            palette.focus
        } else {
            palette.primary
        };
        spans.push(Span::styled("█".repeat(cells), Style::default().fg(color)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), rows[0]);

    // Partition list.
    let list: Vec<Line> = sheet
        .parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let text = format!("{:<10} {:<6} {:>7.1} GB  {}", part.name, part.format, part.size_gb, part.mount);
            widgets::list_row(palette, i == sheet.selected, text, None)
        })
        .collect();
    f.render_widget(Paragraph::new(list), rows[1]);

    // Right-hand fields for the selected partition.
    if let Some(part) = sheet.parts.get(sheet.selected) {
        let fields = vec![
            Line::from(vec![
                Span::styled("nombre   ", Style::default().fg(palette.muted)),
                Span::styled(part.name.clone(), Style::default().fg(palette.text)),
            ]),
            Line::from(vec![
                Span::styled("formato  ", Style::default().fg(palette.muted)),
                Span::styled(format!("{} ▾", part.format), Style::default().fg(palette.text)),
            ]),
            Line::from(vec![
                Span::styled("tamaño   ", Style::default().fg(palette.muted)),
                Span::styled(
                    format!("◀ {:.1} GB ▶", part.size_gb),
                    Style::default().fg(palette.highlight).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("montaje  ", Style::default().fg(palette.muted)),
                Span::styled(part.mount.clone(), Style::default().fg(palette.text)),
            ]),
        ];
        f.render_widget(Paragraph::new(fields), rows[2]);
    }
}

fn draw_erase(f: &mut Frame, sheet: &EraseSheet, area: Rect, palette: &Palette) {
    let lines = vec![
        Line::from(vec![
            Span::styled("nombre   ", Style::default().fg(palette.muted)),
            Span::styled(sheet.device.clone(), Style::default().fg(palette.text)),
        ]),
        Line::from(vec![
            Span::styled("formato  ", Style::default().fg(palette.muted)),
            Span::styled(
                format!("{} ▾", sheet.formats[sheet.index]),
                Style::default().fg(palette.highlight),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "btrfs recomendado para nixdots",
            Style::default().fg(palette.muted),
        )),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_info(f: &mut Frame, sheet: &InfoSheet, area: Rect, palette: &Palette) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Dispositivo  ", Style::default().fg(palette.muted)),
            Span::styled(format!("/dev/{}", sheet.device), Style::default().fg(palette.text)),
        ]),
        Line::from(vec![
            Span::styled("Tipo         ", Style::default().fg(palette.muted)),
            Span::styled(sheet.kind.clone(), Style::default().fg(palette.text)),
        ]),
        Line::from(vec![
            Span::styled("Capacidad    ", Style::default().fg(palette.muted)),
            Span::styled(super::data::human(sheet.size), Style::default().fg(palette.text)),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn sheet_buttons(sheet: &Sheet) -> Vec<Btn> {
    match sheet {
        Sheet::Info(_) => vec![Btn::primary("Cerrar")],
        Sheet::Erase(_) => vec![Btn::secondary("Cancelar"), Btn::destructive("Borrar")],
        Sheet::Partition(_) => vec![Btn::secondary("Cancelar"), Btn::primary("Aplicar")],
    }
}

fn sheet_focus(sheet: &Sheet) -> usize {
    match sheet {
        Sheet::Partition(s) => s.focus,
        Sheet::Erase(s) => s.focus,
        Sheet::Info(s) => s.focus,
    }
}

pub fn key(sheet: &mut Sheet, key: KeyEvent, dry_run: bool) -> Outcome {
    match sheet {
        Sheet::Info(s) => match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char(' ') => Outcome {
                closed: true,
                message: None,
            },
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                s.focus = 0;
                Outcome { closed: false, message: None }
            }
            _ => Outcome { closed: false, message: None },
        },
        Sheet::Erase(s) => {
            if s.confirm {
                return match key.code {
                    KeyCode::Enter => Outcome {
                        closed: true,
                        message: Some(execute(dry_run, &Sheet::Erase(EraseSheet {
                            device: s.device.clone(),
                            formats: s.formats.clone(),
                            index: s.index,
                            focus: s.focus,
                            confirm: false,
                        }).commands())),
                    },
                    _ => {
                        s.confirm = false;
                        Outcome { closed: false, message: None }
                    }
                };
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Outcome { closed: true, message: None },
                KeyCode::Tab | KeyCode::BackTab => {
                    s.focus = (s.focus + 1) % 2;
                    Outcome { closed: false, message: None }
                }
                KeyCode::Left | KeyCode::Right => {
                    let n = s.formats.len();
                    s.index = if key.code == KeyCode::Right {
                        (s.index + 1) % n
                    } else {
                        (s.index + n - 1) % n
                    };
                    Outcome { closed: false, message: None }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if s.focus == 1 {
                        s.confirm = true;
                    } else {
                        return Outcome { closed: true, message: None };
                    }
                    Outcome { closed: false, message: None }
                }
                _ => Outcome { closed: false, message: None },
            }
        }
        Sheet::Partition(s) => {
            if s.confirm {
                return match key.code {
                    KeyCode::Enter => Outcome {
                        closed: true,
                        message: Some(execute(dry_run, &Sheet::Partition(PartitionSheet {
                            device: s.device.clone(),
                            parts: s.parts.clone(),
                            selected: s.selected,
                            focus: s.focus,
                            confirm: false,
                        }).commands())),
                    },
                    _ => {
                        s.confirm = false;
                        Outcome { closed: false, message: None }
                    }
                };
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Outcome { closed: true, message: None },
                KeyCode::Tab | KeyCode::BackTab => {
                    s.focus = (s.focus + 1) % 2;
                    Outcome { closed: false, message: None }
                }
                KeyCode::Up => {
                    s.selected = s.selected.saturating_sub(1);
                    Outcome { closed: false, message: None }
                }
                KeyCode::Down => {
                    if s.selected + 1 < s.parts.len() {
                        s.selected += 1;
                    }
                    Outcome { closed: false, message: None }
                }
                KeyCode::Left | KeyCode::Right => {
                    if let Some(part) = s.parts.get_mut(s.selected) {
                        let delta = if key.code == KeyCode::Right { 1.0 } else { -1.0 };
                        part.size_gb = (part.size_gb + delta).max(1.0);
                    }
                    Outcome { closed: false, message: None }
                }
                KeyCode::Char('f') => {
                    if let Some(part) = s.parts.get_mut(s.selected) {
                        part.format = match part.format.as_str() {
                            "fat32" => "btrfs".into(),
                            "btrfs" => "ext4".into(),
                            "ext4" => "xfs".into(),
                            _ => "fat32".into(),
                        };
                    }
                    Outcome { closed: false, message: None }
                }
                KeyCode::Char('a') => {
                    s.parts.push(Part {
                        name: format!("part{}", s.parts.len() + 1),
                        format: "btrfs".into(),
                        size_gb: 10.0,
                        mount: String::new(),
                    });
                    Outcome { closed: false, message: None }
                }
                KeyCode::Char('d') => {
                    if s.parts.len() > 1 && s.selected < s.parts.len() {
                        s.parts.remove(s.selected);
                        s.selected = s.selected.min(s.parts.len() - 1);
                    }
                    Outcome { closed: false, message: None }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if s.focus == 1 {
                        s.confirm = true;
                    } else {
                        return Outcome { closed: true, message: None };
                    }
                    Outcome { closed: false, message: None }
                }
                _ => Outcome { closed: false, message: None },
            }
        }
    }
}

/// Runs the sheet's commands (or just reports them in the dry-run).
fn execute(dry_run: bool, commands: &[String]) -> String {
    if dry_run {
        return format!("(simulación) {}", commands.join(" · "));
    }
    let mut last = String::new();
    for line in commands {
        let mut parts = line.split_whitespace();
        let Some(program) = parts.next() else { continue };
        let cmd = Cmd::new(program).args(parts.map(String::from));
        let code = runner::run(&cmd, &mut |_| {});
        if code != 0 {
            return format!("error ejecutando: {line}");
        }
        last = line.clone();
    }
    format!("hecho: {last}")
}
