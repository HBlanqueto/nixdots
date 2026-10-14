use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    2
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mut username = widgets::input_line(
        &app.palette,
        "usuario",
        app.user.username.value(),
        app.user.username.visual_cursor(),
        app.user.focus == 0,
        false,
        area.width,
        true,
    );
    if !app.user.username.value().is_empty() {
        let (mark, color) = if app.username_valid() {
            ("✓", app.palette.success)
        } else {
            ("✗", app.palette.error)
        };
        username.spans.push(Span::styled(format!("  {mark}"), Style::default().fg(color)));
    }
    let name = widgets::input_line(
        &app.palette,
        "nombre",
        app.user.name.value(),
        app.user.name.visual_cursor(),
        app.user.focus == 1,
        false,
        area.width,
        true,
    );
    f.render_widget(Paragraph::new(vec![username, name]), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up | KeyCode::Down => {
            app.user.focus = if app.user.focus == 0 { 1 } else { 0 };
            true
        }
        _ if super::is_edit_key(&key) => {
            let target = if app.user.focus == 0 {
                &mut app.user.username
            } else {
                &mut app.user.name
            };
            target.handle_event(&Event::Key(key));
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

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("settings.nix: username = \"{}\";", app.user.username.value()),
        format!(
            "settings.nix: userdescription = \"{}\";",
            app.user.name.value()
        ),
    ]
}
