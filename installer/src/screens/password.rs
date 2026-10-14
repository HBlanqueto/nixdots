use crate::app::App;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(_app: &App) -> u16 {
    3
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let width = area.width;
    let match_mark = if app.passwords_match() {
        Span::styled("  ✓", Style::default().fg(app.palette.success))
    } else {
        Span::raw("")
    };

    let mut first = app.field_line(
        &format!("nueva ({})", app.settings.username),
        app.password.value(),
        app.password.visual_cursor(),
        app.field_focus == 0,
        true,
        width,
    );
    first.spans.push(match_mark.clone());
    if app.field_focus == 0 {
        first = first.style(Style::default().bg(app.palette.row_bg));
    }

    let mut second = app.field_line(
        "confirmar",
        app.password2.value(),
        app.password2.visual_cursor(),
        app.field_focus == 1,
        true,
        width,
    );
    second.spans.push(match_mark);
    if app.field_focus == 1 {
        second = second.style(Style::default().bg(app.palette.row_bg));
    }

    let lines = vec![first, second, Line::from("")];
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Down => {
            app.field_focus = if app.field_focus == 0 { 1 } else { 0 };
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.go_back();
            } else if app.passwords_match() {
                app.advance();
            } else {
                app.status = Some("las contraseñas no coinciden".into());
            }
        }
        _ => {}
    }
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "mkpasswd -m yescrypt --stdin".into(),
        format!(
            "→ {}/persist/secrets/{}.hash (0600)",
            app.repo_dir.display(),
            app.settings.username
        ),
    ]
}
