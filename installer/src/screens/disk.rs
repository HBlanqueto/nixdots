//! "Disco": a minimalist, macOS-style disk picker. One card per disk, an
//! automatic decision sentence, and — only when Windows must be shrunk — a
//! distribution bar.

use crate::app::{Action, App};
use crate::detect::{human_size, Disk};
use crate::disk::{self as diskplan, DiskDecision};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

/// 3-line ASCII disk icon.
const DISK_ICON: [&str; 3] = [" .------.", "|  ____  |", "|__|  |__|"];

const CARD_W: u16 = 26;
const CARD_GAP: u16 = 2;

pub fn height(_app: &App) -> u16 {
    0
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let shrink_case = !app.disks.erase_all && app.disks.decision.is_shrink();
    let link_case = app.selected_disk().is_some_and(|d| d.windows) && !app.disks.erase_all;
    let details_h = if app.disks.details {
        1 + details(app).len() as u16
    } else {
        0
    };
    // heading + sub+gap + cards+gap + sentence+gap (+ shrink + link + details)
    let total = 1 + 2 + 9 + 1 + shrink_case as u16 * 2 + link_case as u16 * 2 + details_h;
    let top = area.y + 3;
    let bottom = area.bottom().saturating_sub(4);
    let mut y = top + bottom.saturating_sub(top).saturating_sub(total) / 2;

    f.render_widget(
        Paragraph::new(Span::styled(
            "¿Dónde quieres instalar nixdots?",
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;
    f.render_widget(
        Paragraph::new(Span::styled(
            "Elige un disco",
            Style::default().fg(app.palette.muted),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 2;

    // Disk cards, centered as a row.
    let count = app.disks.disks.len() as u16;
    let row_w = count * CARD_W + count.saturating_sub(1) * CARD_GAP;
    let mut x = area.x + area.width.saturating_sub(row_w) / 2;
    for i in 0..count {
        let rect = Rect::new(x, y, CARD_W, 8);
        draw_card(f, app, rect, i as usize);
        x += CARD_W + CARD_GAP;
    }
    y += 8 + 1;

    // Decision sentence.
    let (text, color) = sentence(app);
    f.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(color)))
            .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    // Shrink distribution bar.
    if !app.disks.erase_all {
        if let DiskDecision::Shrink { volume, max_free, .. } = app.disks.decision {
            y += 1;
            draw_shrink(f, app, Rect::new(area.x, y, area.width, 1), volume, max_free);
            y += 1;
        }
    }

    // "o borrar todo el disco" (only with Windows, and not already chosen).
    if app.selected_disk().is_some_and(|d| d.windows) && !app.disks.erase_all {
        y += 1;
        let link = "o borrar todo el disco";
        let pad = (area.width as usize).saturating_sub(link.chars().count()) / 2;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" ".repeat(pad)),
                Span::styled(link, Style::default().fg(app.palette.muted)),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }

    // Details (`d`).
    if app.disks.details {
        y += 1;
        for line in details(app) {
            f.render_widget(
                Paragraph::new(Span::styled(line, Style::default().fg(app.palette.muted)))
                    .alignment(Alignment::Center),
                Rect::new(area.x, y, area.width, 1),
            );
            y += 1;
        }
    }

    app.draw_buttons_at(
        f,
        Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 1),
    );

    if app.disks.erase_modal {
        draw_erase_modal(f, app, area);
    }
}

/// One disk card: icon, short name, real size and state.
fn draw_card(f: &mut Frame, app: &App, rect: Rect, i: usize) {
    let Some(disk) = app.disks.disks.get(i) else {
        return;
    };
    let chosen = i == app.disks.index && disk.available;
    let border = if chosen {
        app.palette.primary
    } else if disk.available {
        app.palette.muted
    } else {
        app.palette.disabled
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(if chosen {
            app.palette.well
        } else {
            app.palette.reset
        }));
    let inner = block.inner(rect);
    f.render_widget(Clear, rect);
    f.render_widget(block, rect);

    let dim = !disk.available;
    let name_color = if dim {
        app.palette.disabled
    } else if chosen {
        app.palette.white
    } else {
        app.palette.text
    };
    let mut lines: Vec<Line> = Vec::new();
    for icon in DISK_ICON {
        lines.push(
            Line::from(Span::styled(icon.to_string(), Style::default().fg(name_color)))
                .alignment(Alignment::Center),
        );
    }
    lines.push(
        Line::from(Span::styled(
            disk.short_name(),
            Style::default().fg(name_color).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled(
            disk.size_human(),
            Style::default().fg(if dim { app.palette.disabled } else { app.palette.text }),
        ))
        .alignment(Alignment::Center),
    );
    lines.push(
        Line::from(Span::styled(
            state(disk),
            Style::default().fg(app.palette.muted),
        ))
        .alignment(Alignment::Center),
    );
    f.render_widget(Paragraph::new(lines), inner);
}

fn state(disk: &Disk) -> String {
    if !disk.available {
        return "no disponible".to_string();
    }
    if disk.windows {
        if disk.free >= diskplan::MIN_GIB * diskplan::GIB {
            format!("Windows 11 · {} libres", gb(disk.free))
        } else {
            "Windows 11".to_string()
        }
    } else {
        disk.note.clone()
    }
}

fn gb(bytes: u64) -> String {
    format!("{:.0} GB", bytes as f64 / 1_000_000_000.0)
}

/// The automatic decision sentence and its color.
fn sentence(app: &App) -> (String, ratatui::style::Color) {
    if app.disks.erase_all {
        return (
            "Se borrará todo el disco y Windows.".to_string(),
            app.palette.warning,
        );
    }
    match &app.disks.decision {
        DiskDecision::WholeDisk => ("Se usará todo el disco.".to_string(), app.palette.warning),
        DiskDecision::FreeSpace { .. } => (
            "Se instalará en el espacio libre. Windows no se toca.".to_string(),
            app.palette.text,
        ),
        DiskDecision::Shrink { .. } => (
            "Se hará espacio reduciendo Windows. Tus archivos de Windows se conservan.".to_string(),
            app.palette.warning,
        ),
        DiskDecision::TooSmall { missing } => (
            format!(
                "No hay espacio suficiente. Se necesitan al menos 40 GB (faltan {}).",
                gb(*missing)
            ),
            app.palette.error,
        ),
        DiskDecision::Blocked(msg) => (msg.clone(), app.palette.error),
    }
}

/// `Windows ████ nixdots ████` with ‹ › steppers.
fn draw_shrink(f: &mut Frame, app: &App, area: Rect, volume: u64, max_free: u64) {
    let freed = app.disks.shrink_bytes;
    let windows = volume.saturating_sub(freed);
    let bar_w = area.width.saturating_sub(20).clamp(10, 60) as usize;
    let total = windows + freed;
    let widths = diskplan::block_widths(&[windows, freed], bar_w);
    let _ = total;
    let mut spans = vec![
        Span::styled("Windows ", Style::default().fg(app.palette.secondary)),
        Span::styled(
            "█".repeat(widths[0]),
            Style::default().fg(app.palette.secondary),
        ),
        Span::raw(" "),
        Span::styled(
            "█".repeat(widths[1]),
            Style::default().fg(app.palette.primary),
        ),
        Span::styled(" nixdots ", Style::default().fg(app.palette.primary)),
    ];
    let _ = max_free;
    f.render_widget(
        Paragraph::new(Line::from(std::mem::take(&mut spans))).alignment(Alignment::Center),
        area,
    );

    // ‹ › steppers at both ends.
    let left = Rect::new(area.x + 2, area.y, 1, 1);
    let right = Rect::new(area.right().saturating_sub(3), area.y, 1, 1);
    f.render_widget(
        Paragraph::new(Span::styled(
            "‹",
            Style::default().fg(app.palette.primary).add_modifier(Modifier::BOLD),
        )),
        left,
    );
    f.render_widget(
        Paragraph::new(        Span::styled(
            "›",
            Style::default().fg(app.palette.primary).add_modifier(Modifier::BOLD),
        )),
        right,
    );
}

/// The `d` details: two gray lines.
fn details(app: &App) -> Vec<String> {
    let size = match app.disks.effective() {
        DiskDecision::Shrink { .. } => app.disks.shrink_bytes,
        DiskDecision::FreeSpace { .. } => app
            .selected_disk()
            .map(|d| d.free)
            .unwrap_or(0),
        _ => app
            .selected_disk()
            .map(|d| d.size.saturating_sub(diskplan::GIB))
            .unwrap_or(0),
    };
    let cifrado = if app.disks.encrypt {
        "cifrado"
    } else {
        "sin cifrar"
    };
    vec![
        format!("EFI 1 GB · System HD {} btrfs · {cifrado}", human_size(size)),
        "@ @home @nix @log @persist · zram".to_string(),
    ]
}

/// The "borrar todo" confirmation, in the license dialog's style.
fn draw_erase_modal(f: &mut Frame, app: &mut App, area: Rect) {
    widgets::dim(f, area, &app.palette);
    let w = 58.min(area.width);
    let h = 7;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.warning));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    f.render_widget(
        Paragraph::new(Span::styled(
            "Se borrará Windows y todos sus archivos.",
            Style::default().fg(app.palette.text),
        ))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );
    let buttons = vec![Btn::secondary("No"), Btn::destructive("Borrar todo")];
    let row = Rect::new(inner.x, inner.bottom().saturating_sub(2), inner.width, 1);
    let _ = widgets::render_buttons(f, row, &app.palette, &buttons, app.disks.erase_focus, true);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.disks.erase_modal {
        match key.code {
            KeyCode::Esc => app.disks.erase_modal = false,
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                app.disks.erase_focus = 1 - app.disks.erase_focus;
            }
            KeyCode::Enter => {
                if app.disks.erase_focus == 1 {
                    app.disks.erase_all = true;
                }
                app.disks.erase_modal = false;
            }
            _ => {}
        }
        return true;
    }
    match key.code {
        KeyCode::Left | KeyCode::Right => {
            if app.disks.bar_focus && app.disks.decision.is_shrink() {
                app.adjust_shrink(if key.code == KeyCode::Left { -10 } else { 10 });
            } else {
                app.cycle_disk(if key.code == KeyCode::Left { -1 } else { 1 });
            }
            true
        }
        KeyCode::Tab if app.disks.decision.is_shrink() => {
            app.disks.bar_focus = !app.disks.bar_focus;
            true
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.disks.details = !app.disks.details;
            true
        }
        KeyCode::Char('b') | KeyCode::Char('B')
            if app.selected_disk().is_some_and(|d| d.windows) && !app.disks.erase_all =>
        {
            app.disks.erase_modal = true;
            app.disks.erase_focus = 0;
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

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
