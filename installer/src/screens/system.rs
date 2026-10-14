use crate::app::{App, FieldKind};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(app: &App) -> u16 {
    app.fields.len() as u16
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let width = area.width;
    let lines: Vec<Line> = app
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let focused = i == app.field_focus;
            let value = field.input.value();
            let cursor = field.input.visual_cursor();
            let mut line = app.field_line(field.label, value, cursor, focused, false, width);
            if field.kind == FieldKind::TimeZone || field.kind == FieldKind::Username {
                let valid = app.field_valid(field.label, field.kind);
                if !value.is_empty() {
                    let (mark, color) = if valid {
                        ("✓", app.palette.success)
                    } else {
                        ("✗", app.palette.error)
                    };
                    line.spans.push(Span::styled(
                        format!("  {mark}"),
                        Style::default().fg(color),
                    ));
                }
            }
            if focused {
                line = line.style(Style::default().bg(app.palette.row_bg));
            }
            line
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    let count = app.fields.len();
    match key.code {
        KeyCode::Up => {
            app.field_focus = (app.field_focus + count - 1) % count;
        }
        KeyCode::Down => {
            app.field_focus = (app.field_focus + 1) % count;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.go_back();
            } else if app.username_valid() && app.validate_timezone() {
                app.advance();
            } else {
                app.status = Some("revisa usuario y zona horaria".into());
            }
        }
        _ => {}
    }
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("nix eval --json -f {}/settings.nix", app.repo_dir.display()),
        "timedatectl list-timezones".into(),
    ]
}
