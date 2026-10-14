use crate::app::App;
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
        Line::from(Span::styled(
            "GNU GPL v2.0",
            Style::default().fg(app.palette.text),
        )),
        Line::from(Span::styled(
            "incluye paquetes no libres (allowUnfree)",
            Style::default().fg(app.palette.muted),
        )),
        Line::from(""),
        app.checkbox(app.license_accepted, "Acepto", app.button_focus == 1),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.license_accepted = !app.license_accepted;
        }
        KeyCode::Char('l') => {
            app.pending_less = Some(app.repo_dir.join("LICENSE"));
        }
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.go_back();
            } else if app.license_accepted {
                app.advance();
            } else {
                app.status = Some("acepta la licencia para continuar".into());
            }
        }
        _ => {}
    }
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["less LICENSE".into()]
}
