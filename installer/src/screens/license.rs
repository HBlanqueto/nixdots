//! The license screen: a compact NixOS logo, the GPL text (embedded, so it
//! works offline) in a scrollable box, and the No acepto / Acepto pills.
//! Acepto opens a small confirmation dialog; No acepto returns to Inicio.

use crate::app::{Action, App, Click, ScrollKind};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

/// Embedded so the installer works without network or the repo.
const LICENSE: &str = include_str!("../../../LICENSE");

pub fn height(_app: &App) -> u16 {
    0
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let console = crate::theme::console::is_linux_console();
    // Same full logo as the Inicio screen (never the compact version).
    let logo = crate::logo::nixos_logo(console);
    let logo_h = logo.len() as u16;
    let logo_w = logo
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.chars().count()).sum::<usize>())
        .max()
        .unwrap_or(0) as u16;

    let box_w = 84.min(area.width.saturating_sub(4)).max(24);
    // logo + gap + nixdots + desc + gap + box + gray + gap + buttons
    let overhead = logo_h + 1 + 1 + 1 + 1 + 1 + 1 + 2;
    // The logo always keeps its height; the text box shrinks instead.
    let box_h = area.height.saturating_sub(overhead).clamp(3, 20);
    let total = logo_h + 1 + 1 + 1 + 1 + box_h + 1 + 1 + 1;
    let mut y = area.y + area.height.saturating_sub(total) / 2;

    if logo_h > 0 {
        let x = area.x + area.width.saturating_sub(logo_w) / 2;
        f.render_widget(Paragraph::new(logo), Rect::new(x, y, logo_w, logo_h));
        y += logo_h;
    }
    y += 1;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "nixdots",
            Style::default().fg(app.palette.text).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Para continuar con la instalación, debes aceptar los términos de la licencia.",
            Style::default().fg(app.palette.muted),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 2;

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    draw_box(f, app, Rect::new(box_x, y, box_w, box_h));
    y += box_h;

    let gray = format!(
        "Se guardará una copia en /etc/nixdots/LICENSE · línea {} de {}",
        app.license_line.min(app.license_total).max(1),
        app.license_total
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(gray, Style::default().fg(app.palette.muted))))
            .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 2;

    let buttons: Vec<Btn> = app.buttons().into_iter().map(|(b, _)| b).collect();
    app.button_hitboxes = widgets::render_buttons(
        f,
        Rect::new(area.x, y, area.width, 1),
        &app.palette,
        &buttons,
        app.button_focus,
        app.focus_buttons,
    );

    if app.license_modal {
        draw_modal(f, app, area);
    }
}

fn draw_box(f: &mut Frame, app: &mut App, rect: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.primary));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let width = inner.width.saturating_sub(1).max(8) as usize;
    let (lines, sources, source_total) = wrap_license(width);
    let total = lines.len();
    let visible = inner.height as usize;
    let max = total.saturating_sub(visible);
    app.license_max_scroll = max;
    if app.license_scroll > max {
        app.license_scroll = max;
    }
    let start = app.license_scroll;
    app.license_total = source_total;
    app.license_line = sources.get(start).copied().unwrap_or(source_total);

    let text_area = Rect::new(inner.x, inner.y, inner.width, inner.height);
    app.license_text_area = text_area;
    let mut rendered: Vec<Line> = Vec::new();
    for i in 0..visible {
        let idx = start + i;
        if idx >= total {
            rendered.push(Line::from(""));
        } else {
            rendered.push(Line::from(Span::styled(
                lines[idx].clone(),
                Style::default().fg(app.palette.text),
            )));
        }
    }
    f.render_widget(Paragraph::new(rendered), text_area);

    let mut state = ScrollbarState::new(total)
        .position(start)
        .viewport_content_length(visible);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .style(Style::default().fg(app.palette.row_bg))
        .thumb_style(Style::default().fg(app.palette.primary));
    f.render_stateful_widget(scrollbar, text_area, &mut state);

    app.targets.borrow_mut().push((text_area, Click::LicenseText));
    app.scrollbars.borrow_mut().push((
        Rect::new(text_area.right().saturating_sub(1), text_area.y, 1, text_area.height),
        ScrollKind::License,
    ));
}

/// The small confirmation dialog, centered over the (dimmed) screen.
fn draw_modal(f: &mut Frame, app: &mut App, area: Rect) {
    let console = crate::theme::console::is_linux_console();
    let gray = if console { app.palette.row_bg } else { Color::Rgb(0x2f, 0x3a, 0x49) };
    let bg = if console { app.palette.row_bg } else { Color::Rgb(0x17, 0x1d, 0x27) };

    // Dim everything behind the dialog.
    {
        let buf = f.buffer_mut();
        let r = area.intersection(*buf.area());
        for y in r.top()..r.bottom() {
            for x in r.left()..r.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.fg = gray;
                    cell.bg = gray;
                }
            }
        }
    }

    let w = 58.min(area.width);
    let h = 7;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.primary))
        .style(Style::default().bg(bg));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("❄ ", Style::default().fg(app.palette.primary)),
            Span::styled(
                "Leí y acepto los términos de la licencia.",
                Style::default().fg(app.palette.text),
            ),
        ]))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
    );

    let buttons = vec![Btn::secondary("No acepto"), Btn::primary("Acepto")];
    let row = Rect::new(inner.x, inner.bottom().saturating_sub(2), inner.width, 1);
    let hit = widgets::render_buttons(f, row, &app.palette, &buttons, app.license_modal_focus, true);
    for (x0, x1, index) in hit {
        let r = Rect::new(x0, row.y, x1.saturating_sub(x0) + 1, 1);
        let click = if index == 1 {
            Click::LicenseModalAcepto
        } else {
            Click::LicenseModalNo
        };
        app.targets.borrow_mut().push((r, click));
    }
}

/// Keys while the confirmation dialog is open: Esc / No acepto close it,
/// Acepto accepts and moves on. Everything else is swallowed.
pub fn modal_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.license_modal = false,
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
            app.license_modal_focus = 1 - app.license_modal_focus;
        }
        KeyCode::Enter => {
            if app.license_modal_focus == 1 {
                app.license_modal = false;
                app.advance();
            } else {
                app.license_modal = false;
            }
        }
        _ => {}
    }
}

/// Returns the wrapped lines, the 1-based source line of each, and the total.
fn wrap_license(width: usize) -> (Vec<String>, Vec<usize>, usize) {
    let source: Vec<&str> = LICENSE.lines().collect();
    let source_total = source.len();
    let mut lines = Vec::new();
    let mut sources = Vec::new();
    for (i, raw) in source.iter().enumerate() {
        for chunk in wrap(raw, width) {
            lines.push(chunk);
            sources.push(i + 1);
        }
    }
    (lines, sources, source_total)
}

fn wrap(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut count = 0;
    for word in line.split_inclusive(' ') {
        let wlen = word.chars().count();
        if count + wlen > width && count > 0 {
            out.push(std::mem::take(&mut current));
            count = 0;
        }
        if wlen > width {
            for ch in word.chars() {
                if count == width {
                    out.push(std::mem::take(&mut current));
                    count = 0;
                }
                current.push(ch);
                count += 1;
            }
        } else {
            current.push_str(word);
            count += wlen;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    let page = app.license_text_area.height.max(1) as usize;
    match key.code {
        KeyCode::Up => {
            app.license_scroll = app.license_scroll.saturating_sub(1);
            true
        }
        KeyCode::Down => {
            app.license_scroll = (app.license_scroll + 1).min(app.license_max_scroll);
            true
        }
        KeyCode::PageUp => {
            app.license_scroll = app.license_scroll.saturating_sub(page);
            true
        }
        KeyCode::PageDown => {
            app.license_scroll = (app.license_scroll + page).min(app.license_max_scroll);
            true
        }
        KeyCode::Home => {
            app.license_scroll = 0;
            true
        }
        KeyCode::End => {
            app.license_scroll = app.license_max_scroll;
            true
        }
        KeyCode::Char(' ') => {
            app.license_scroll = (app.license_scroll + page).min(app.license_max_scroll);
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("No acepto"), Action::Back),
        (Btn::primary("Acepto ›"), Action::LicenseAccept),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["cp LICENSE /mnt/etc/nixdots/LICENSE".into()]
}
