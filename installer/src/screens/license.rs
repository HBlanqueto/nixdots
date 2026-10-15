//! The license screen: a compact NixOS logo, the GPL text (embedded, so it
//! works offline) in a scrollable box, and the No acepto / Acepto pills.
//! Acepto opens a small confirmation dialog; No acepto returns to Inicio.

use crate::app::{Action, App};
use crate::widgets::{self, Btn, ButtonRow};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

/// Embedded so the installer works without network or the repo.
const LICENSE: &str = include_str!("../../../LICENSE");

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
    // logo + 3-line gap (like Inicio) + nixdots + desc + gap + box + gray + buttons
    let overhead = logo_h + 3 + 1 + 1 + 1 + 1 + 1 + 2;
    // The logo always keeps its height; the text box shrinks instead.
    let box_h = area.height.saturating_sub(overhead).clamp(3, 20);
    let total = logo_h + 3 + 1 + 1 + 1 + 1 + box_h + 1 + 1 + 1;
    let mut y = area.y + area.height.saturating_sub(total) / 2;

    if logo_h > 0 {
        let x = area.x + area.width.saturating_sub(logo_w) / 2;
        f.render_widget(Paragraph::new(logo), Rect::new(x, y, logo_w, logo_h));
        y += logo_h;
    }
    y += 3;
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
    let box_rect = Rect::new(box_x, y, box_w, box_h);
    draw_box(f, app, box_rect);
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

    app.draw_buttons(f, Rect::new(area.x, y, area.width, 1));

    if app.license_modal {
        draw_modal(f, app, box_rect);
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
}

/// The small confirmation dialog, centered over the license text box.
fn draw_modal(f: &mut Frame, app: &mut App, over: Rect) {
    let palette = app.palette;
    let buttons = vec![Btn::new("No acepto"), Btn::new("Acepto")];
    let focus = app.license_modal_focus;
    let message = Line::from(vec![
        Span::styled("❄  ", Style::default().fg(palette.primary)),
        Span::styled(
            "Leí y acepto los términos de la licencia.",
            Style::default().fg(palette.text),
        ),
    ]);
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
        (Btn::new("No acepto"), Action::Back),
        (Btn::new("Acepto"), Action::LicenseAccept),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["cp LICENSE /mnt/etc/nixdots/LICENSE".into()]
}
