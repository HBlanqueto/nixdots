use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    // two fields + separator + the visible-name line
    6
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    // Nombre first, usuario second.
    let name = widgets::field(
        &app.palette,
        "nombre completo",
        app.user.name.value(),
        app.user.name.visual_cursor(),
        app.user.focus == 0 && !app.focus_buttons,
        false,
        area.width,
        true,
        "Nombre y apellido",
    );
    let mut username = widgets::field(
        &app.palette,
        "usuario · se genera del nombre",
        app.user.username.value(),
        app.user.username.visual_cursor(),
        app.user.focus == 1 && !app.focus_buttons,
        false,
        area.width,
        true,
        "se genera del nombre",
    );
    if !app.user.username.value().is_empty() {
        let (mark, color) = if app.username_valid() {
            ("✓", app.palette.success)
        } else {
            ("✗", app.palette.error)
        };
        username[0]
            .spans
            .push(Span::styled(format!("  {mark}"), Style::default().fg(color)));
    }
    let mut lines = name;
    lines.extend(username);
    // Separator + the macOS-style visible name, updated as the name is typed.
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(app.palette.rule),
    )));
    lines.push(Line::from(vec![
        Span::styled("este equipo se llamará ", Style::default().fg(app.palette.muted)),
        Span::styled(
            app.pretty_host_name(),
            Style::default().fg(app.palette.highlight),
        ),
    ]));
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Down / Tab from the name lock the username (it becomes editable).
        KeyCode::Down if app.user.focus == 0 => {
            focus_username(app);
            true
        }
        KeyCode::Up if app.user.focus == 1 => {
            app.user.focus = 0;
            true
        }
        KeyCode::Tab if app.user.focus == 0 => {
            focus_username(app);
            true
        }
        // Shift+Tab goes back usuario -> nombre.
        KeyCode::BackTab if app.user.focus == 1 => {
            app.user.focus = 0;
            true
        }
        // Enter advances nombre -> usuario; from usuario the app takes over
        // and moves the focus to Continuar.
        KeyCode::Enter if app.user.focus == 0 => {
            focus_username(app);
            true
        }
        _ if super::is_edit_key(&key) => {
            if app.user.focus == 0 {
                app.user.name.handle_event(&Event::Key(key));
                app.regenerate_username();
            } else {
                edit_username(app, key);
            }
            true
        }
        _ => false,
    }
}

fn focus_username(app: &mut App) {
    app.user.focus = 1;
    // Editing by hand from now on (unless it is emptied, which re-enables
    // generation).
    if !app.user.username.value().is_empty() {
        app.user.edited = true;
    }
}

/// The hand-edited username only accepts `a-z0-9_-` and must start with a
/// letter or `_`; other characters are dropped as they are typed.
fn edit_username(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        let allowed = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
        let first_ok = !app.user.username.value().is_empty()
            || c.is_ascii_lowercase()
            || c == '_';
        if !allowed || !first_ok {
            return;
        }
    }
    app.user.username.handle_event(&Event::Key(key));
    app.user.edited = !app.user.username.value().is_empty();
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("settings.nix: userdescription = \"{}\";", app.user.name.value()),
        format!("settings.nix: username = \"{}\";", app.user.username.value()),
    ]
}
