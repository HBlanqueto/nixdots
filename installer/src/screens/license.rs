use crate::app::{Action, App};
use crate::widgets::Btn;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(_app: &App) -> u16 {
    4
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("GNU GPL v2.0", Style::default().fg(app.palette.text))),
        Line::from(Span::styled(
            "incluye paquetes no libres (allowUnfree)",
            Style::default().fg(app.palette.muted),
        )),
        Line::from(""),
        crate::widgets::checkbox(&app.palette, app.license_accepted, "Acepto", !app.focus_buttons),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.license_accepted = !app.license_accepted;
            true
        }
        KeyCode::Char('l') => {
            app.open_license();
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::secondary("leer completa"), Action::ViewLicense),
        (Btn::primary("Aceptar ›"), Action::Next),
    ]
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["visor interno: LICENSE (sin salir de la TUI)".into()]
}
