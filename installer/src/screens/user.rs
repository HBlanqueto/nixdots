use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let w = area.width;
    let blink = app.blink();
    let focus = app.user.focus;
    let has_focus = !app.focus_buttons && !app.no_focus();

    let mut lines = widgets::text_field(
        &app.palette,
        "Nombre completo",
        app.user.name.value(),
        app.user.name.visual_cursor(),
        has_focus && focus == 0,
        blink,
        false,
        "Nombre y apellido",
        w,
        None,
        false,
    );
    lines.push(Line::from(""));

    // The checkbox and, when set, the hand-written username.
    lines.push(widgets::checkbox(
        &app.palette,
        app.user.custom,
        "Usar un nombre de usuario diferente",
        has_focus && focus == 1,
    ));
    lines.push(Line::from(""));
    if app.user.custom {
        let note = Line::from(Span::styled(
            app.username_error().unwrap_or("solo a-z, 0-9, - y _"),
            Style::default().fg(if app.username_error().is_some() {
                app.palette.error
            } else {
                app.palette.muted
            }),
        ));
        lines.extend(widgets::text_field(
            &app.palette,
            "Usuario",
            app.user.username.value(),
            app.user.username.visual_cursor(),
            has_focus && focus == 2,
            blink,
            false,
            &app.final_username(),
            w,
            Some(note),
            app.username_error().is_some(),
        ));
    } else {
        // Read-only generated username, not focusable.
        let generated = app.final_username();
        let tail = " · generado del nombre";
        let used = "Usuario".chars().count() + generated.chars().count() + tail.chars().count();
        let pad = (w as usize).saturating_sub(used);
        lines.push(Line::from(vec![
            Span::styled("Usuario", Style::default().fg(app.palette.muted)),
            Span::raw(" ".repeat(pad)),
            Span::styled(generated, Style::default().fg(app.palette.highlight)),
            Span::styled(tail, Style::default().fg(app.palette.muted)),
        ]));
    }
    lines.push(Line::from(""));

    // Separator + the macOS-style visible name.
    lines.push(Line::from(Span::styled(
        "─".repeat(w as usize),
        Style::default().fg(app.palette.rule),
    )));
    let pretty = app.pretty_host_name();
    let pad = (w as usize)
        .saturating_sub("Este equipo se llamará".chars().count() + pretty.chars().count() + 1);
    lines.push(Line::from(vec![
        Span::styled(
            "Este equipo se llamará",
            Style::default().fg(app.palette.muted),
        ),
        Span::raw(" ".repeat(pad)),
        Span::styled(pretty, Style::default().fg(app.palette.highlight)),
    ]));

    let focus_y = match focus {
        1 => 5,
        2 if app.user.custom => 7,
        _ => 0,
    };
    let scroll = widgets::scroll_to_show(focus_y, 4, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if !app.focus_buttons {
        match key.code {
            KeyCode::Char(' ') if app.user.focus == 1 => {
                app.user.custom = !app.user.custom;
                if app.user.custom {
                    app.user.focus = 2;
                } else {
                    app.user.username = Input::default();
                }
                return true;
            }
            KeyCode::Down | KeyCode::Tab => {
                match app.user.focus {
                    0 => app.user.focus = 1,
                    1 if app.user.custom => app.user.focus = 2,
                    1 => app.focus_buttons_on_primary(),
                    _ => app.focus_buttons_on_primary(),
                }
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                match app.user.focus {
                    1 => app.user.focus = 0,
                    2 => app.user.focus = 1,
                    _ => {}
                }
                return true;
            }
            KeyCode::Enter if app.user.focus == 0 => {
                app.user.focus = 1;
                return true;
            }
            KeyCode::Enter if app.user.focus == 1 => {
                if app.user.custom {
                    app.user.focus = 2;
                } else {
                    app.focus_buttons_on_primary();
                }
                return true;
            }
            _ => {}
        }
    }
    if super::is_edit_key(&key) {
        if app.user.focus == 0 {
            app.user.name.handle_event(&Event::Key(key));
        } else if app.user.focus == 2 {
            edit_username(app, key);
        }
        return true;
    }
    false
}

/// The hand-written username only accepts `a-z0-9_-`; other characters are
/// dropped as they are typed (validation flags the rest).
fn edit_username(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        let allowed = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
        if !allowed {
            return;
        }
    }
    app.user.username.handle_event(&Event::Key(key));
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!(
            "settings.nix: userdescription = \"{}\";",
            app.user.name.value()
        ),
        format!("settings.nix: username = \"{}\";", app.final_username()),
    ]
}
