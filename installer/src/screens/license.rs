//! The license screen: the GPL text (embedded, so it works offline) scrolled
//! inside the panel, the allowUnfree note, the Acepto checkbox and the
//! Atrás / Aceptar buttons.

use crate::app::{Action, App, Click};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

/// Embedded so the installer works without network or the repo.
const LICENSE: &str = include_str!("../../../LICENSE");

/// Height of the scrolled text area.
const ROWS: usize = 20;

pub fn height(_app: &App) -> u16 {
    // text + footer + blank + checkbox
    (ROWS + 3) as u16
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // Leave the last column for the scrollbar.
    let width = (area.width as usize).saturating_sub(1).max(8);
    let (lines, sources, source_total, note_start) = wrap_license(width);

    let total = lines.len();
    let visible = ROWS;
    let max = total.saturating_sub(visible);
    app.license_max_scroll = max;
    if app.license_scroll > max {
        app.license_scroll = max;
    }
    let start = app.license_scroll;

    let text_area = Rect::new(area.x, area.y, area.width, visible as u16);
    let mut rendered: Vec<Line> = Vec::new();
    for i in 0..visible {
        let idx = start + i;
        if idx >= total {
            rendered.push(Line::from(""));
            continue;
        }
        let style = if idx >= note_start {
            Style::default().fg(app.palette.warning)
        } else {
            Style::default().fg(app.palette.text)
        };
        rendered.push(Line::from(Span::styled(lines[idx].clone(), style)));
    }
    f.render_widget(Paragraph::new(rendered), text_area);

    // Scrollbar: thumb in primary, track in lbg.
    let mut state = ScrollbarState::new(total)
        .position(start)
        .viewport_content_length(visible);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .style(Style::default().fg(app.palette.row_bg))
        .thumb_style(Style::default().fg(app.palette.primary));
    f.render_stateful_widget(scrollbar, text_area, &mut state);

    // Footer: hint on the left, position on the right.
    let footer = Rect::new(area.x, area.y + visible as u16, area.width, 1);
    let src_line = sources
        .get(start)
        .copied()
        .unwrap_or(source_total)
        .min(source_total);
    let left = "↓ desplaza para leer";
    let right = format!("línea {src_line} de {source_total}");
    let pad = (area.width as usize)
        .saturating_sub(left.chars().count())
        .saturating_sub(right.chars().count());
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(left, Style::default().fg(app.palette.muted)),
            Span::raw(" ".repeat(pad)),
            Span::styled(right, Style::default().fg(app.palette.muted)),
        ])),
        footer,
    );

    // Clickable regions: the text (focus it) and the checkbox (toggle it).
    app.targets.borrow_mut().push((text_area, Click::LicenseText));
    let checkbox = Rect::new(area.x, area.y + visible as u16 + 2, area.width, 1);
    app.targets.borrow_mut().push((checkbox, Click::LicenseCheckbox));
    f.render_widget(
        Paragraph::new(widgets::checkbox(
            &app.palette,
            app.license_accepted,
            "Acepto",
            app.license_focus == 1 && !app.focus_buttons,
        )),
        checkbox,
    );
}

/// Returns the wrapped lines, the 1-based source line of each, the total
/// source lines and the index where the note begins.
fn wrap_license(width: usize) -> (Vec<String>, Vec<usize>, usize, usize) {
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
    let note_start = lines.len();
    lines.push(String::new());
    sources.push(source_total);
    lines.push("Nota de nixdots:".to_string());
    sources.push(source_total);
    for chunk in wrap(
        "esta configuración incluye paquetes no libres (allowUnfree), cada uno con su propia licencia.",
        width,
    ) {
        lines.push(chunk);
        sources.push(source_total);
    }
    (lines, sources, source_total, note_start)
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
    let page = ROWS;
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
        // Space toggles when the checkbox has the focus, otherwise scrolls.
        KeyCode::Char(' ') if app.license_focus == 1 => {
            app.license_accepted = !app.license_accepted;
            true
        }
        KeyCode::Char(' ') => {
            app.license_scroll = (app.license_scroll + page).min(app.license_max_scroll);
            true
        }
        KeyCode::Char('x') if app.license_focus == 1 => {
            app.license_accepted = !app.license_accepted;
            true
        }
        // Tab moves from the text to the checkbox; from there the app moves to
        // the buttons.
        KeyCode::Tab if app.license_focus == 0 => {
            app.license_focus = 1;
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Aceptar ›"), Action::Next),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
