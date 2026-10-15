use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width;
    let focus = app.user.focus;
    let has_focus = !app.focus_buttons && !app.no_focus();

    let name = widgets::text_field(
        &app.palette,
        "Nombre completo",
        app.user.name.value(),
        app.user.name.visual_cursor(),
        has_focus && focus == 0,
        false,
        "Nombre y apellido",
        w,
        None,
        false,
    );
    let username_note = username_note(app);
    let username = widgets::text_field(
        &app.palette,
        "Usuario",
        app.user.username.value(),
        app.user.username.visual_cursor(),
        has_focus && focus == 1,
        false,
        "se genera del nombre",
        w,
        username_note,
        !app.user.username.value().is_empty() && !app.username_valid(),
    );

    let mut lines = name;
    lines.push(Line::from(""));
    lines.extend(username);
    lines.push(Line::from(""));
    // Separator + the macOS-style visible name.
    lines.push(Line::from(Span::styled(
        "─".repeat(w as usize),
        Style::default().fg(app.palette.rule),
    )));
    let pretty = app.pretty_host_name();
    let pad = (w as usize).saturating_sub("Este equipo se llamará".chars().count() + pretty.chars().count() + 1);
    lines.push(Line::from(vec![
        Span::styled("Este equipo se llamará", Style::default().fg(app.palette.muted)),
        Span::raw(" ".repeat(pad)),
        Span::styled(pretty, Style::default().fg(app.palette.highlight)),
    ]));

    let focus_y = if focus == 0 { 0 } else { 5 };
    let scroll = widgets::scroll_to_show(focus_y, 4, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

/// The `Usuario` field's note: generation state or a validation error.
fn username_note(app: &App) -> Option<Line<'static>> {
    let value = app.user.username.value();
    if value.is_empty() {
        return None;
    }
    if !app.username_valid() {
        return Some(Line::from(Span::styled(
            "✗ inválido",
            Style::default().fg(app.palette.error),
        )));
    }
    if app.user.edited {
        Some(Line::from(Span::styled(
            "editado",
            Style::default().fg(app.palette.muted),
        )))
    } else {
        Some(Line::from(Span::styled(
            "✓ generado",
            Style::default().fg(app.palette.success),
        )))
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if !app.focus_buttons {
        match key.code {
            // Down / Tab move to the next field, or to Continuar past the last.
            KeyCode::Down | KeyCode::Tab => {
                if app.user.focus == 0 {
                    focus_username(app);
                } else {
                    app.focus_buttons_on_primary();
                }
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                if app.user.focus == 1 {
                    app.user.focus = 0;
                }
                return true;
            }
            // Enter advances nombre -> usuario; from usuario the app moves to
            // Continuar.
            KeyCode::Enter if app.user.focus == 0 => {
                focus_username(app);
                return true;
            }
            _ => {}
        }
    }
    if super::is_edit_key(&key) {
        if app.user.focus == 0 {
            app.user.name.handle_event(&Event::Key(key));
            app.regenerate_username();
        } else {
            edit_username(app, key);
        }
        return true;
    }
    false
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
