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
    let user_check_y = lines.len() as u16;
    lines.push(widgets::checkbox(
        &app.palette,
        app.user.custom,
        "Usar un nombre de usuario diferente",
        has_focus && focus == 1,
    ));
    lines.push(Line::from(""));
    let mut user_field_y = 0;
    if app.user.custom {
        let note = Line::from(Span::styled(
            app.username_error().unwrap_or("solo a-z, 0-9, - y _"),
            Style::default().fg(if app.username_error().is_some() {
                app.palette.error
            } else {
                app.palette.muted
            }),
        ));
        user_field_y = lines.len() as u16;
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
    lines.push(Line::from(""));

    // Network name: the generated one (read-only) or a hand-written one.
    let generated = app.final_host_name();
    if !app.user.host_custom {
        let tail = " · generado del nombre";
        let used =
            "Nombre en la red".chars().count() + generated.chars().count() + tail.chars().count();
        let pad = (w as usize).saturating_sub(used);
        lines.push(Line::from(vec![
            Span::styled("Nombre en la red", Style::default().fg(app.palette.muted)),
            Span::raw(" ".repeat(pad)),
            Span::styled(
                generated.clone(),
                Style::default().fg(app.palette.highlight),
            ),
            Span::styled(tail, Style::default().fg(app.palette.muted)),
        ]));
    }
    let host_check_y = lines.len() as u16;
    lines.push(widgets::checkbox(
        &app.palette,
        app.user.host_custom,
        "Usar un nombre de red diferente",
        has_focus && focus == 3,
    ));
    lines.push(Line::from(""));
    let mut host_field_y = 0;
    if app.user.host_custom {
        let note = Line::from(Span::styled(
            app.host_name_error().unwrap_or("letras, números y -"),
            Style::default().fg(if app.host_name_error().is_some() {
                app.palette.error
            } else {
                app.palette.muted
            }),
        ));
        host_field_y = lines.len() as u16;
        lines.extend(widgets::text_field(
            &app.palette,
            "Nombre en la red",
            app.user.host_name.value(),
            app.user.host_name.visual_cursor(),
            has_focus && focus == 4,
            blink,
            false,
            &generated,
            w,
            Some(note),
            app.host_name_error().is_some(),
        ));
    }

    let focus_y = match focus {
        1 => user_check_y,
        2 if app.user.custom => user_field_y,
        3 => host_check_y,
        4 if app.user.host_custom => host_field_y,
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
            KeyCode::Char(' ') if app.user.focus == 3 => {
                app.user.host_custom = !app.user.host_custom;
                if app.user.host_custom {
                    app.user.focus = 4;
                } else {
                    app.user.host_name = Input::default();
                }
                return true;
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::Enter => {
                focus_next(app);
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                focus_prev(app);
                return true;
            }
            _ => {}
        }
    }
    if super::is_edit_key(&key) {
        match app.user.focus {
            0 => {
                app.user.name.handle_event(&Event::Key(key));
            }
            2 => edit_username(app, key),
            4 => edit_host_name(app, key),
            _ => {}
        }
        return true;
    }
    false
}

/// Moves the focus to the next visible field: 0 nombre, 1 casilla de usuario,
/// 2 usuario manual (si `custom`), 3 casilla de red, 4 nombre de red manual
/// (si `host_custom`); past the last one, to the buttons.
fn focus_next(app: &mut App) {
    app.user.focus = match app.user.focus {
        0 => 1,
        1 if app.user.custom => 2,
        1 => 3,
        2 => 3,
        3 if app.user.host_custom => 4,
        _ => {
            app.focus_buttons_on_primary();
            return;
        }
    };
}

/// The inverse of [`focus_next`], skipping the hidden fields.
fn focus_prev(app: &mut App) {
    app.user.focus = match app.user.focus {
        4 => 3,
        3 if app.user.custom => 2,
        3 => 1,
        2 => 1,
        1 => 0,
        _ => return,
    };
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

/// The hand-written network name only accepts ASCII letters, digits and `-`;
/// other characters are dropped as they are typed (validation flags the rest).
fn edit_host_name(app: &mut App, key: KeyEvent) {
    if let KeyCode::Char(c) = key.code {
        if !(c.is_ascii_alphanumeric() || c == '-') {
            return;
        }
    }
    app.user.host_name.handle_event(&Event::Key(key));
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
