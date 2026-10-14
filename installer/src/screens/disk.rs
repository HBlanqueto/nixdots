use crate::app::App;
use crate::detect::human_size;
use crate::disk::{self, NamedPartition};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(app: &App) -> u16 {
    let disks = app.disks.len() as u16;
    // Every non-btrfs partition is one leaf line; btrfs contributes one line
    // per subvolume.
    let tree: u16 = app
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
    6 + disks + tree
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let width = area.width as usize;
    let mut lines: Vec<Line> = Vec::new();

    if app.disks.is_empty() {
        lines.push(Line::from(Span::styled(
            "no se encontraron discos",
            Style::default().fg(app.palette.error),
        )));
    }
    for (i, disk) in app.disks.iter().enumerate() {
        let selected = i == app.disk_index;
        let label = format!("{}  ({})", disk.path(), disk.size_human());
        let mut line = app.radio(selected, &label);
        if !disk.model.is_empty() {
            line.spans.push(Span::styled(
                format!("  {}", disk.model),
                Style::default().fg(app.palette.muted),
            ));
        }
        lines.push(line);
    }

    lines.push(Line::from(""));

    // Proportional block bar: ESP in warning, btrfs in primary.
    let total = app.selected_disk().map(|d| d.size).unwrap_or(0);
    let (sizes, width_bar) = partition_sizes(&app.layout, total, width);
    let mut bar: Vec<Span> = Vec::new();
    let mut labels: Vec<Span> = Vec::new();
    for (i, cells) in width_bar.iter().enumerate() {
        let color = if i == 0 {
            app.palette.warning
        } else {
            app.palette.primary
        };
        bar.push(Span::styled("█".repeat(*cells), Style::default().fg(color)));
        let label = format!("{} ", human_size(sizes[i]));
        let pad = cells.saturating_sub(label.chars().count());
        labels.push(Span::styled(
            format!("{label}{}", " ".repeat(pad)),
            Style::default().fg(app.palette.muted),
        ));
    }
    lines.push(Line::from(bar));
    lines.push(Line::from(labels));
    lines.push(Line::from(""));

    // Subvolume tree, straight from the evaluated disko.nix.
    for part in &app.layout {
        if part.partition.kind.as_deref() != Some("btrfs") {
            // Simple partitions (the ESP) are a leaf.
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
            let glyph = if i + 1 == names.len() {
                "└─"
            } else {
                "├─"
            };
            let mount = sub.mountpoint.clone().unwrap_or_default();
            let tag = if sub.is_ephemeral(name) {
                "efímero"
            } else if sub.is_persistent(name) {
                "persistente"
            } else {
                ""
            };
            let tag_color = if tag.is_empty() {
                app.palette.muted
            } else {
                app.palette.secondary
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{glyph} "), Style::default().fg(app.palette.border)),
                Span::styled(format!("{name:<10}"), Style::default().fg(app.palette.text)),
                Span::styled(format!("{mount:<10}"), Style::default().fg(app.palette.muted)),
                Span::styled(tag, Style::default().fg(tag_color)),
            ]));
        }
    }

    lines.push(Line::from(""));
    let target = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/…".into());
    lines.push(app.checkbox(
        app.disk_confirmed,
        &format!("Borrar {target}"),
        app.button_focus == 1,
    ));

    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.cycle_disk(-1),
        KeyCode::Down => app.cycle_disk(1),
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.disk_confirmed = !app.disk_confirmed;
        }
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.go_back();
            } else if app.disk_confirmed {
                app.advance();
            } else {
                app.status = Some("marca la casilla para borrar el disco".into());
            }
        }
        _ => {}
    }
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    vec![
        format!("disko --mode destroy,format,mount disko.nix --argstr device {device}"),
    ]
}

/// Numeric byte size per partition. The `100%` partition absorbs the rest.
fn partition_sizes(layout: &[NamedPartition], total: u64, bar_width: usize) -> (Vec<u64>, Vec<usize>) {
    let mut sizes: Vec<u64> = layout
        .iter()
        .map(|p| {
            let size = p.partition.size.clone().unwrap_or_default();
            if size.contains('%') {
                0
            } else {
                disk::parse_size(&size, total)
            }
        })
        .collect();
    let fixed: u64 = sizes.iter().sum();
    for (i, part) in layout.iter().enumerate() {
        let size = part.partition.size.clone().unwrap_or_default();
        if size.contains('%') {
            sizes[i] = total.saturating_sub(fixed);
        }
    }
    let widths = disk::block_widths(&sizes, bar_width);
    (sizes, widths)
}
