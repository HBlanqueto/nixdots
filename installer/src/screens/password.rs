use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    5
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let first = widgets::field(
        &app.palette,
        &format!("contraseña ({})", app.settings.username),
        app.password.p1.value(),
        app.password.p1.visual_cursor(),
        app.password.focus == 0 && !app.focus_buttons && !app.no_focus(),
        true,
        area.width,
        true,
        "",
    );
    let second = widgets::field(
        &app.palette,
        "confirmar",
        app.password.p2.value(),
        app.password.p2.visual_cursor(),
        app.password.focus == 1 && !app.focus_buttons && !app.no_focus(),
        true,
        area.width,
        true,
        "",
    );
    let (filled, label, color) = strength(app.password.p1.value(), &app.palette);
    let bar = Line::from(vec![
        Span::styled("seguridad  ", Style::default().fg(app.palette.muted)),
        Span::styled("━".repeat(filled), Style::default().fg(color)),
        Span::styled(
            "─".repeat(12 - filled),
            Style::default().fg(app.palette.rule),
        ),
        Span::styled(
            format!("  {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        if app.passwords_match() {
            Span::styled("  ✓ coinciden", Style::default().fg(app.palette.success))
        } else {
            Span::raw("")
        },
    ]);
    let mut lines = first;
    lines.extend(second);
    lines.push(bar);
    f.render_widget(Paragraph::new(lines), area);
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
    match key.code {
        KeyCode::Up | KeyCode::Down => {
            app.password.focus = if app.password.focus == 0 { 1 } else { 0 };
            true
        }
        // Shift+Tab goes back confirmar -> nueva.
        KeyCode::BackTab if app.password.focus == 1 => {
            app.password.focus = 0;
            true
        }
        // Enter goes nueva -> confirmar; from confirmar the app moves to
        // Continuar.
        KeyCode::Enter if app.password.focus == 0 => {
            app.password.focus = 1;
            true
        }
        _ if super::is_edit_key(&key) => {
            let target = if app.password.focus == 0 {
                &mut app.password.p1
            } else {
                &mut app.password.p2
            };
            target.handle_event(&Event::Key(key));
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

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "mkpasswd -m yescrypt --stdin".into(),
        format!(
            "→ /persist/secrets/{}.hash (0600)",
            app.settings.username
        ),
    ]
}
