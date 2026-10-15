use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width;
    let focus = app.password.focus;
    let has_focus = !app.focus_buttons && !app.no_focus();

    let first = widgets::text_field(
        &app.palette,
        "Contraseña",
        app.password.p1.value(),
        app.password.p1.visual_cursor(),
        has_focus && focus == 0,
        true,
        "",
        w,
        if app.password.p1.value().is_empty() {
            None
        } else {
            Some(strength_note(app))
        },
        false,
    );
    let confirm_note = if app.password.p2.value().is_empty() {
        None
    } else if app.passwords_match() {
        Some(Line::from(Span::styled(
            "✓ coinciden",
            Style::default().fg(app.palette.success),
        )))
    } else {
        Some(Line::from(Span::styled(
            "no coinciden",
            Style::default().fg(app.palette.error),
        )))
    };
    let second = widgets::text_field(
        &app.palette,
        "Confirmar",
        app.password.p2.value(),
        app.password.p2.visual_cursor(),
        has_focus && focus == 1,
        true,
        "",
        w,
        confirm_note,
        !app.password.p2.value().is_empty() && !app.passwords_match(),
    );

    let mut lines = first;
    lines.push(Line::from(""));
    lines.extend(second);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─".repeat(w as usize),
        Style::default().fg(app.palette.rule),
    )));
    lines.push(Line::from(Span::styled(
        "Se usa para iniciar sesión y para sudo.",
        Style::default().fg(app.palette.muted),
    )));

    let focus_y = if focus == 0 { 0 } else { 5 };
    let scroll = widgets::scroll_to_show(focus_y, 4, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

/// The strength bar note: `████░░ buena`.
fn strength_note(app: &App) -> Line<'static> {
    let (filled, label, color) = strength(app.password.p1.value(), &app.palette);
    Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(12 - filled), Style::default().fg(app.palette.rule)),
        Span::styled(
            format!(" {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

/// (filled cells out of 12, label, colour) for the strength bar.
fn strength(pass: &str, palette: &crate::theme::Palette) -> (usize, &'static str, ratatui::style::Color) {
    let n = pass.chars().count();
    let mut classes = 0;
    if pass.chars().any(|c| c.is_ascii_lowercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_uppercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_digit()) {
        classes += 1;
    }
    if pass.chars().any(|c| !c.is_ascii_alphanumeric()) {
        classes += 1;
    }
    let score = ((n.min(20) as f32) / 20.0) * 0.6 + (classes as f32 / 4.0) * 0.4;
    let filled = ((score * 12.0).round() as usize).clamp(1, 12);
    if n < 8 || score < 0.45 {
        (filled, "débil", palette.error)
    } else if score < 0.72 {
        (filled, "aceptable", palette.warning)
    } else {
        (filled, "buena", palette.success)
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if !app.focus_buttons {
        match key.code {
            KeyCode::Down | KeyCode::Tab => {
                if app.password.focus == 0 {
                    app.password.focus = 1;
                } else {
                    app.focus_buttons_on_primary();
                }
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                if app.password.focus == 1 {
                    app.password.focus = 0;
                }
                return true;
            }
            KeyCode::Enter if app.password.focus == 0 => {
                app.password.focus = 1;
                return true;
            }
            _ => {}
        }
    }
    if super::is_edit_key(&key) {
        let target = if app.password.focus == 0 {
            &mut app.password.p1
        } else {
            &mut app.password.p2
        };
        target.handle_event(&Event::Key(key));
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "mkpasswd -m yescrypt --stdin".into(),
        format!(
            "→ /persist/secrets/{}.hash (0600)",
            app.settings.username
        ),
    ]
}
