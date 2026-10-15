use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    3
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = widgets::field(
        &app.palette,
        "nombre del equipo",
        app.host.input.value(),
        app.host.input.visual_cursor(),
        !app.focus_buttons,
        false,
        area.width,
        true,
        "",
    );
    if !app.host.input.value().is_empty() {
        let (mark, color) = if app.hostname_valid() {
            ("✓", app.palette.success)
        } else {
            ("✗", app.palette.error)
        };
        lines[0]
            .spans
            .push(Span::styled(format!("  {mark}"), Style::default().fg(color)));
    }
    lines.push(Line::from(Span::styled(
        "letras, números y guiones",
        Style::default().fg(app.palette.muted),
    )));
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if super::is_edit_key(&key) {
        app.host.input.handle_event(&Event::Key(key));
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
    vec![format!(
        "settings.nix: hostName = \"{}\";",
        app.host.input.value()
    )]
}
