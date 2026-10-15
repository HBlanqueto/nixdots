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
    let blink = app.blink();
    let enabled = app.git.enabled;
    let focus = app.git.focus;
    let has_focus = !app.focus_buttons && !app.no_focus();

    let mut lines = vec![widgets::checkbox(
        &app.palette,
        enabled,
        "Configurar git ahora",
        has_focus && focus == 0,
    )];
    lines.push(Line::from(""));
    if enabled {
        lines.extend(widgets::text_field(
            &app.palette,
            "Nombre",
            app.git.name.value(),
            app.git.name.visual_cursor(),
            has_focus && focus == 1,
            blink,
            false,
            "tu nombre",
            w,
            None,
            false,
        ));
        lines.push(Line::from(""));
        lines.extend(widgets::text_field(
            &app.palette,
            "Correo",
            app.git.email.value(),
            app.git.email.visual_cursor(),
            has_focus && focus == 2,
            blink,
            false,
            "tu@correo",
            w,
            email_note(app),
            !app.git.email.value().is_empty() && !email_valid(app.git.email.value()),
        ));
    } else {
        lines.push(Line::from(Span::styled(
            "opcional · se puede configurar después",
            Style::default().fg(app.palette.muted),
        )));
    }

    let focus_y = match focus {
        1 if enabled => 2,
        2 if enabled => 7,
        _ => 0,
    };
    let scroll = widgets::scroll_to_show(focus_y, 4, area.height);
    f.render_widget(Paragraph::new(lines.clone()).scroll((scroll, 0)), area);
    widgets::draw_scrollbar(f, &app.palette, area, lines.len(), scroll, area.height);
}

/// A basic email check for the note: `a@b.c`, no spaces.
fn email_valid(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(' ')
}

fn email_note(app: &App) -> Option<Line<'static>> {
    if app.git.email.value().is_empty() {
        return None;
    }
    if email_valid(app.git.email.value()) {
        Some(Line::from(Span::styled(
            "✓",
            Style::default().fg(app.palette.success),
        )))
    } else {
        Some(Line::from(Span::styled(
            "correo no válido",
            Style::default().fg(app.palette.error),
        )))
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    let enabled = app.git.enabled;
    if !app.focus_buttons {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Char('x') if app.git.focus == 0 => {
                app.git.enabled = !app.git.enabled;
                if !app.git.enabled {
                    app.git.focus = 0;
                }
                return true;
            }
            KeyCode::Down | KeyCode::Tab => {
                let last = if enabled { 2 } else { 0 };
                if app.git.focus < last {
                    app.git.focus += 1;
                } else {
                    app.focus_buttons_on_primary();
                }
                return true;
            }
            KeyCode::Up | KeyCode::BackTab => {
                if app.git.focus > 0 {
                    app.git.focus -= 1;
                }
                return true;
            }
            KeyCode::Enter if enabled && app.git.focus < 2 => {
                app.git.focus += 1;
                return true;
            }
            _ => {}
        }
    }
    if super::is_edit_key(&key) && enabled && app.git.focus > 0 {
        let target = if app.git.focus == 1 {
            &mut app.git.name
        } else {
            &mut app.git.email
        };
        target.handle_event(&Event::Key(key));
        return true;
    }
    false
}

pub fn buttons(app: &App) -> Vec<(Btn, Action)> {
    let primary = if app.git.enabled {
        Btn::new("Continuar")
    } else {
        Btn::new("Omitir")
    };
    vec![(Btn::new("Atrás"), Action::Back), (primary, Action::Next)]
}

pub fn commands(app: &App) -> Vec<String> {
    if app.git.enabled {
        vec![
            format!("settings.nix: gitName = \"{}\";", app.git.name.value()),
            format!("settings.nix: gitEmail = \"{}\";", app.git.email.value()),
        ]
    } else {
        vec![
            "settings.nix: gitName = null;".into(),
            "settings.nix: gitEmail = null;".into(),
        ]
    }
}
