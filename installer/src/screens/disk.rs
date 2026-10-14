use crate::app::{Action, App};
use crate::detect::human_size;
use crate::disk;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(app: &App) -> u16 {
    let n = app.disks.disks.len() as u16;
    let tree: u16 = app
        .disks
        .layout
        .iter()
        .map(|p| {
            if p.partition.kind.as_deref() == Some("btrfs") {
                p.partition.subvolumes.len() as u16
            } else {
                1
            }
        })
        .sum();
    // list + blank + "cómo instalar" + cards(4) + blank + "así quedará"(1) + bar+legend(2)
    // + blank + two columns
    let single = if app.disks.options().len() == 1 { 1 } else { 0 };
    n.max(1) + 1 + 1 + 5 + single + 1 + 1 + 2 + 1 + tree.max(5)
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let width = area.width;
    let mut y = area.y;

    // 1. Target disk list.
    f.render_widget(
        Paragraph::new(Span::styled(
            "Disco de destino:",
            Style::default().fg(app.palette.muted),
        )),
        Rect::new(area.x, y, width, 1),
    );
    y += 1;
    f.render_widget(Paragraph::new(disk_lines(app)), Rect::new(area.x, y, width, app.disks.disks.len().max(1) as u16));
    y += app.disks.disks.len().max(1) as u16;

    y += 1;
    // 2. How to install: two cards.
    f.render_widget(
        Paragraph::new(Span::styled("Cómo instalar:", Style::default().fg(app.palette.muted))),
        Rect::new(area.x, y, width, 1),
    );
    y += 1;
    draw_cards(f, app, Rect::new(area.x, y, width, 5));
    y += 5;
    if app.disks.options().len() == 1 {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No se encontró otro sistema",
                Style::default().fg(app.palette.muted),
            )),
            Rect::new(area.x, y, width, 1),
        );
        y += 1;
    }

    y += 1;
    // 3. Result bar.
    let dev = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/…".into());
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Así quedará {dev}:"),
            Style::default().fg(app.palette.muted),
        )),
        Rect::new(area.x, y, width, 1),
    );
    y += 1;
    draw_result_bar(f, app, Rect::new(area.x, y, width, 2));
    y += 3;

    // 4. Two columns: tree | volume name + option-specific controls.
    let half = width / 2;
    let left = Rect::new(area.x, y, half.saturating_sub(1), area.bottom().saturating_sub(y));
    let right = Rect::new(area.x + half + 1, y, width.saturating_sub(half + 1), area.bottom().saturating_sub(y));
    draw_tree(f, app, left);
    draw_right(f, app, right);
}

fn disk_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for (i, disk) in app.disks.disks.iter().enumerate() {
        let selected = i == app.disks.index && disk.available && app.disk_focus == 0;
        let marker = if selected { "› " } else { "  " };
        let base = if disk.available {
            Style::default().fg(if selected { app.palette.white } else { app.palette.text })
        } else {
            Style::default().fg(app.palette.disabled)
        };
        let style = if selected {
            base.bg(app.palette.row_bg)
        } else {
            base
        };
        let note = if disk.available { disk.note.clone() } else { "no disponible".to_string() };
        let note_color = if !disk.available {
            app.palette.disabled
        } else if disk.is_windows() {
            app.palette.warning
        } else {
            app.palette.muted
        };
        let model = if disk.model.is_empty() {
            String::new()
        } else {
            format!("  {}", disk.model)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, style),
            Span::styled(
                format!("{}  {}  ", disk.path(), disk.size_human()),
                style,
            ),
            Span::styled(model, style),
            Span::styled(format!("  {note}"), Style::default().fg(note_color).bg(if selected { app.palette.row_bg } else { app.palette.reset })),
        ]));
    }
    lines
}

fn draw_cards(f: &mut Frame, app: &App, area: Rect) {
    let options = app.disks.options();
    let dev = app.selected_disk();
    let disk_size = dev.map(|d| d.size).unwrap_or(0);
    let half = area.width / 2;
    for (i, title) in options.iter().enumerate() {
        let x = area.x + i as u16 * half;
        let w = if i + 1 == options.len() { area.width.saturating_sub(x - area.x) } else { half };
        let rect = Rect::new(x, area.y, w, area.height);
        let chosen = i == app.disks.option;
        let border = if chosen { app.palette.primary } else { app.palette.border };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        let inner = block.inner(rect);
        f.render_widget(block, rect);

        let radio = if chosen { "(•) " } else { "( ) " };
        let title_style = if chosen {
            Style::default().fg(app.palette.white).add_modifier(ratatui::style::Modifier::BOLD)
        } else {
            Style::default().fg(app.palette.muted)
        };
        let mut lines = vec![Line::from(vec![
            Span::styled(radio, Style::default().fg(app.palette.primary)),
            Span::styled(title.to_string(), title_style),
        ])];
        lines.push(card_bar(app, i, disk_size, inner.width));
        lines.push(Line::from(Span::styled(card_legend(app, i, disk_size), Style::default().fg(app.palette.muted))));
        f.render_widget(Paragraph::new(lines), inner);
    }
}

/// (size bytes, colour, label) segments for the current option.
fn segments(app: &App, option: usize, disk_size: u64) -> Vec<(u64, Color, String)> {
    let esp = 1_073_741_824u64; // 1 GiB ESP
    let gib = 1_073_741_824u64;
    if option == 1 {
        // Install alongside Windows 11.
        let windows = 155 * gib;
        let system = (app.disks.partition_gb * gib).min(disk_size.saturating_sub(esp + windows));
        vec![
            (windows, app.palette.secondary, "Windows 11".into()),
            (esp, app.palette.warning, "EFI".into()),
            (system, app.palette.primary, app.disks.volume_name.value().to_string()),
        ]
    } else {
        let system = disk_size.saturating_sub(esp);
        vec![
            (esp, app.palette.warning, "EFI".into()),
            (system, app.palette.primary, app.disks.volume_name.value().to_string()),
        ]
    }
}

fn card_bar(app: &App, option: usize, disk_size: u64, width: u16) -> Line<'static> {
    let segs = segments(app, option, disk_size);
    let sizes: Vec<u64> = segs.iter().map(|s| s.0).collect();
    let widths = disk::block_widths(&sizes, width.saturating_sub(0) as usize);
    let mut spans = Vec::new();
    for (i, w) in widths.iter().enumerate() {
        spans.push(Span::styled("█".repeat(*w), Style::default().fg(segs[i].1)));
    }
    Line::from(spans)
}

fn card_legend(app: &App, option: usize, disk_size: u64) -> String {
    segments(app, option, disk_size)
        .iter()
        .map(|s| format!("{} {}", s.2, human_size(s.0)))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn draw_result_bar(f: &mut Frame, app: &App, area: Rect) {
    let disk_size = app.selected_disk().map(|d| d.size).unwrap_or(0);
    let segs = segments(app, app.disks.option, disk_size);
    let sizes: Vec<u64> = segs.iter().map(|s| s.0).collect();
    let widths = disk::block_widths(&sizes, area.width as usize);
    let bar: Vec<Span> = widths
        .iter()
        .zip(segs.iter())
        .map(|(w, s)| Span::styled("█".repeat(*w), Style::default().fg(s.1)))
        .collect();
    let legend: Vec<Span> = segs
        .iter()
        .flat_map(|s| {
            vec![
                Span::styled("● ", Style::default().fg(s.1)),
                Span::styled(format!("{} {}   ", s.2, human_size(s.0)), Style::default().fg(app.palette.muted)),
            ]
        })
        .collect();
    f.render_widget(Paragraph::new(Line::from(bar)), Rect::new(area.x, area.y, area.width, 1));
    f.render_widget(Paragraph::new(Line::from(legend)), Rect::new(area.x, area.y + 1, area.width, 1));
}

fn draw_tree(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for part in &app.disks.layout {
        if part.partition.kind.as_deref() != Some("btrfs") {
            let mount = part.partition.mountpoint.clone().unwrap_or_default();
            let fs = part.partition.format.clone().unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled("└─ ", Style::default().fg(app.palette.border)),
                Span::styled(format!("{:<10}", part.name), Style::default().fg(app.palette.text)),
                Span::styled(format!("{mount:<10}"), Style::default().fg(app.palette.muted)),
                Span::styled(fs, Style::default().fg(app.palette.secondary)),
            ]));
            continue;
        }
        let names: Vec<&String> = part.partition.subvolumes.keys().collect();
        for (i, name) in names.iter().enumerate() {
            let sub = &part.partition.subvolumes[*name];
            let glyph = if i + 1 == names.len() { "└─" } else { "├─" };
            let mount = sub.mountpoint.clone().unwrap_or_default();
            let tag = if sub.is_ephemeral(name) {
                "efímero"
            } else if sub.is_persistent(name) {
                "persiste"
            } else {
                ""
            };
            let tag_color = if tag.is_empty() { app.palette.muted } else { app.palette.secondary };
            lines.push(Line::from(vec![
                Span::styled(format!("{glyph} "), Style::default().fg(app.palette.border)),
                Span::styled(format!("{name:<10}"), Style::default().fg(app.palette.text)),
                Span::styled(format!("{mount:<9}"), Style::default().fg(app.palette.muted)),
                Span::styled(tag, Style::default().fg(tag_color)),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_right(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    // Volume name (editable).
    lines.push(widgets::input_line(
        &app.palette,
        "nombre",
        app.disks.volume_name.value(),
        app.disks.volume_name.visual_cursor(),
        app.disk_focus == 1 && !app.focus_buttons,
        false,
        area.width,
        true,
        "System HD",
    ));
    lines.push(Line::from(""));

    if app.disks.option == 1 {
        // Alongside Windows: size stepper + warning.
        lines.push(Line::from(vec![
            Span::styled("tamaño  ", Style::default().fg(app.palette.muted)),
            Span::styled(
                format!("◀ {} GB ▶", app.disks.partition_gb),
                Style::default().fg(app.palette.highlight).add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "desactiva BitLocker y el inicio rápido antes de continuar",
            Style::default().fg(app.palette.warning),
        )));
    } else {
        // Erase everything: red confirmation checkbox.
        let dev = app
            .selected_disk()
            .map(|d| d.path())
            .unwrap_or_else(|| "/dev/…".into());
        let mark = if app.disks.confirmed { "[x]" } else { "[ ]" };
        let color = app.palette.error;
        lines.push(Line::from(vec![
            Span::styled(format!("{mark} "), Style::default().fg(color)),
            Span::styled(
                format!("Borrar todo {dev} (incluye Windows)"),
                Style::default().fg(color),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "no se puede deshacer",
            Style::default().fg(app.palette.muted),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            app.cycle_disk(-1);
            true
        }
        KeyCode::Down if app.disk_focus == 0 => {
            app.cycle_disk(1);
            true
        }
        KeyCode::Left | KeyCode::Right => {
            let n = app.disks.options().len();
            if n > 1 {
                let delta = if key.code == KeyCode::Right { 1 } else { n - 1 };
                app.disks.option = (app.disks.option + delta) % n;
                app.disks.confirmed = false;
            } else if app.disks.option == 1 {
                // Adjust the partition size when installing alongside.
                if key.code == KeyCode::Right {
                    app.disks.partition_gb = (app.disks.partition_gb + 10).min(2000);
                } else {
                    app.disks.partition_gb = app.disks.partition_gb.saturating_sub(10).max(10);
                }
            }
            true
        }
        KeyCode::Tab if app.disk_focus == 0 => {
            app.disk_focus = 1;
            true
        }
        KeyCode::Char(' ') if app.disks.option == 0 && app.disk_focus == 0 => {
            app.disks.confirmed = !app.disks.confirmed;
            true
        }
        _ if app.disk_focus == 1 && super::is_edit_key(&key) => {
            app.disks.volume_name.handle_event(&Event::Key(key));
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Continuar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    let mut commands = vec![format!(
        "disko --mode destroy,format,mount disko.nix --argstr device {device}"
    )];
    if app.disks.windows {
        commands.push("os-prober   (para la entrada de Windows)".into());
    }
    commands
}
