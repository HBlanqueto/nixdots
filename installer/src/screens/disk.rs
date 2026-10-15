//! "Disco": the disk picker. Not inside the shared box: the progress bar, a
//! title, a row of 26-column disk cards, the automatic-decision sentence and
//! the buttons.

use crate::app::{Action, App};
use crate::detect::{human_size, Disk};
use crate::disk::{self as diskplan, DiskDecision};
use crate::widgets::{self, Btn, ButtonRow};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

/// The disk icon: a small rounded box with bars and a light.
const ICON: [&str; 3] = ["╭────────────╮", "│ ▂▂▂▂▂▂▂  ● │", "╰────────────╯"];
/// Fallback for the Linux console, which has no block/braille glyphs.
const ICON_ASCII: [&str; 3] = ["+------------+", "| =======  o |", "+------------+"];

const CARD_W: u16 = 26;
const CARD_GAP: u16 = 2;
const CARD_H: u16 = 8;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let shrink_case = !app.disks.erase_all && app.disks.decision.is_shrink();
    let link_case = app.selected_disk().is_some_and(|d| d.windows) && !app.disks.erase_all;
    let details_h = if app.disks.details {
        1 + details(app).len() as u16
    } else {
        0
    };
    // title + sub + gap + cards + gap + sentence (+ shrink + link + details)
    let total = 1 + 1 + 1 + CARD_H + 1 + 1 + shrink_case as u16 + link_case as u16 + details_h;
    let mut y = area.y + area.height.saturating_sub(total) / 2;

    f.render_widget(
        Paragraph::new(Span::styled(
            "¿Dónde quieres instalar nixdots?",
            Style::default()
                .fg(app.palette.white)
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

    // Cards, centered as a row.
    let count = app.disks.disks.len() as u16;
    if count > 0 {
        let row_w = count * CARD_W + count.saturating_sub(1) * CARD_GAP;
        let mut x = area.x + area.width.saturating_sub(row_w) / 2;
        for i in 0..count {
            draw_card(f, app, Rect::new(x, y, CARD_W, CARD_H), i as usize);
            x += CARD_W + CARD_GAP;
        }
    }
    y += CARD_H + 1;

    // Decision sentence.
    let (text, color) = sentence(app);
    f.render_widget(
        Paragraph::new(Span::styled(text, Style::default().fg(color)))
            .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    // Shrink distribution bar.
    if shrink_case {
        if let DiskDecision::Shrink { volume, max_free, .. } = app.disks.decision {
            draw_shrink(f, app, Rect::new(area.x, y, area.width, 1), volume, max_free);
            y += 1;
        }
    }

    // "o borrar todo el disco" (only with Windows, and not already chosen).
    if link_case {
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
        for line in details(app) {
            f.render_widget(
                Paragraph::new(Span::styled(line, Style::default().fg(app.palette.muted)))
                    .alignment(Alignment::Center),
                Rect::new(area.x, y, area.width, 1),
            );
            y += 1;
        }
    }

    if app.disks.erase_modal {
        draw_erase_modal(f, app, area);
    }
}

/// One disk card: the icon, the short name, the size and the state. Unusable
/// disks are dimmed and their state names the reason.
fn draw_card(f: &mut Frame, app: &App, rect: Rect, i: usize) {
    let Some(disk) = app.disks.disks.get(i) else {
        return;
    };
    let usable = disk.unusable_reason().is_none();
    let chosen = i == app.disks.index && usable;
    let border = if chosen {
        app.palette.primary
    } else if usable {
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

    let name_color = if !usable {
        app.palette.disabled
    } else if chosen {
        app.palette.white
    } else {
        app.palette.text
    };
    let icon = if crate::theme::console::is_linux_console() {
        ICON_ASCII
    } else {
        ICON
    };
    let mut lines: Vec<Line> = icon
        .iter()
        .map(|line| {
            Line::from(Span::styled(line.to_string(), Style::default().fg(name_color)))
                .alignment(Alignment::Center)
        })
        .collect();
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
            Style::default().fg(if usable {
                app.palette.text
            } else {
                app.palette.disabled
            }),
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
    if let Some(reason) = disk.unusable_reason() {
        return reason.to_string();
    }
    let base = if disk.windows {
        if disk.free >= diskplan::MIN_GIB * diskplan::GIB {
            format!("Windows 11 · {} libres", gb(disk.free))
        } else {
            "Windows 11".to_string()
        }
    } else {
        disk.note.clone()
    };
    if disk.test {
        format!("{base} · prueba")
    } else {
        base
    }
}

fn gb(bytes: u64) -> String {
    format!("{:.0} GB", bytes as f64 / 1_000_000_000.0)
}

/// The automatic decision sentence and its color.
fn sentence(app: &App) -> (String, ratatui::style::Color) {
    if !has_usable_disk(app) {
        return (
            "No hay ningún disco donde instalar.".to_string(),
            app.palette.error,
        );
    }
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
        DiskDecision::TooSmall { .. } => (
            "Demasiado pequeño (mín. 40 GB).".to_string(),
            app.palette.error,
        ),
        DiskDecision::Blocked(msg) => (msg.clone(), app.palette.error),
    }
}

/// Whether at least one disk can be installed to.
fn has_usable_disk(app: &App) -> bool {
    app.disks.disks.iter().any(|d| d.unusable_reason().is_none())
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

/// The "borrar todo" confirmation, over the dimmed disk picker. `area` is the
/// box content, so the dialog is centered over the box itself.
fn draw_erase_modal(f: &mut Frame, app: &mut App, area: Rect) {
    let palette = app.palette;
    let buttons = vec![Btn::new("No"), Btn::danger("Borrar todo")];
    let focus = app.disks.erase_focus;
    let over = Rect::new(
        area.x.saturating_sub(1),
        area.y.saturating_sub(1),
        area.width + 2,
        area.height + 2,
    );
    let message = Line::from(Span::styled(
        "Se borrará Windows y todos sus archivos.",
        Style::default().fg(palette.text),
    ));
    widgets::draw_dialog(f, over, &palette, message, |f, row| {
        let widget = ButtonRow {
            buttons: &buttons,
            focus,
            focused: true,
            palette: &palette,
        };
        f.render_widget(widget, row);
    });
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
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
