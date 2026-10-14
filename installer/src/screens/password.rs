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
    3
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mark = if app.passwords_match() {
        Span::styled("  ✓", Style::default().fg(app.palette.success))
    } else {
        Span::raw("")
    };
    let mut first = widgets::input_line(
        &app.palette,
        &format!("nueva ({})", app.settings.username),
        app.password.p1.value(),
        app.password.p1.visual_cursor(),
        app.password.focus == 0 && !app.focus_buttons,
        true,
        area.width,
        true,
        "",
    );
    first.spans.push(mark.clone());
    let mut second = widgets::input_line(
        &app.palette,
        "confirmar",
        app.password.p2.value(),
        app.password.p2.visual_cursor(),
        app.password.focus == 1 && !app.focus_buttons,
        true,
        area.width,
        true,
        "",
    );
    second.spans.push(mark);
    let note = Line::from(Span::styled(
        "se guarda como hash en /persist",
        Style::default().fg(app.palette.muted),
    ));
    f.render_widget(Paragraph::new(vec![first, second, note]), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up | KeyCode::Down => {
            app.password.focus = if app.password.focus == 0 { 1 } else { 0 };
            true
        }
        // Shift+Tab goes back confirmar -> nueva.
        KeyCode::BackTab if app.password.focus == 1 => {
            app.password.focus = 0;
            true
        }
        // Enter goes nueva -> confirmar; from confirmar the app moves to
        // Continuar.
        KeyCode::Enter if app.password.focus == 0 => {
            app.password.focus = 1;
            true
        }
        _ if super::is_edit_key(&key) => {
            let target = if app.password.focus == 0 {
                &mut app.password.p1
            } else {
                &mut app.password.p2
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
        "mkpasswd -m yescrypt --stdin".into(),
        format!(
            "→ /persist/secrets/{}.hash (0600)",
            app.settings.username
        ),
    ]
}
