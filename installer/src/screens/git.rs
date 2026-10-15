use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(app: &App) -> u16 {
    if app.git.enabled {
        5
    } else {
        2
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let enabled = app.git.enabled;
    let checkbox = widgets::checkbox(
        &app.palette,
        enabled,
        "Configurar git ahora",
        app.git.focus == 0 && !app.focus_buttons && !app.no_focus(),
    );
    let mut lines = vec![checkbox];
    if enabled {
        lines.extend(widgets::field(
            &app.palette,
            "nombre",
            app.git.name.value(),
            app.git.name.visual_cursor(),
            app.git.focus == 1 && !app.focus_buttons && !app.no_focus(),
            false,
            area.width,
            true,
            "tu nombre",
        ));
        lines.extend(widgets::field(
            &app.palette,
            "email",
            app.git.email.value(),
            app.git.email.visual_cursor(),
            app.git.focus == 2 && !app.no_focus(),
            false,
            area.width,
            true,
            "tu@correo",
        ));
    } else {
        lines.push(Line::from(Span::styled(
            "opcional · se puede configurar después",
            Style::default().fg(app.palette.muted),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.git.enabled = !app.git.enabled;
            if !app.git.enabled {
                app.git.focus = 0;
            }
            true
        }
        KeyCode::Up | KeyCode::Down => {
            let fields = 3;
            app.git.focus = (app.git.focus + if key.code == KeyCode::Down { 1 } else { fields - 1 })
                % fields;
            true
        }
        // Shift+Tab walks the fields backwards.
        KeyCode::BackTab if app.git.enabled && app.git.focus > 0 => {
            app.git.focus -= 1;
            true
        }
        // Enter walks the three fields; from the last the app moves to the
        // button row.
        KeyCode::Enter if app.git.enabled && app.git.focus < 2 => {
            app.git.focus += 1;
            true
        }
        _ if super::is_edit_key(&key) && app.git.enabled && app.git.focus > 0 => {
            let target = if app.git.focus == 1 {
                &mut app.git.name
            } else {
                &mut app.git.email
            };
            target.handle_event(&Event::Key(key));
            true
        }
        _ => false,
    }
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
