//! "Teclado": a searchable list of layouts in the shared box, the language's
//! suggestion highlighted, and a test field under the box.

use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    super::picker::ROWS as u16 + 1
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.keyboards else {
        return;
    };
    let search = super::picker::search_line(
        &app.palette,
        picker,
        "buscar distribución",
        "distribuciones",
        area.width,
        !app.focus_buttons && !app.key_test_focus && !app.no_focus(),
    );
    f.render_widget(Paragraph::new(search), Rect::new(area.x, area.y, area.width, 1));
    let list = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
    super::picker::draw_centered(f, &app.palette, list, picker, app.keyboard_suggested);
}

/// The `prueba` field shown under the box (the screen's note, two lines).
pub fn draw_test(f: &mut Frame, app: &App, area: Rect) {
    let focus = app.key_test_focus && !app.focus_buttons && !app.no_focus();
    let lines = widgets::field(
        &app.palette,
        "prueba",
        app.key_test.value(),
        app.key_test.visual_cursor(),
        focus,
        false,
        area.width,
        true,
        "escribe aquí para probar el teclado",
    );
    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        area,
    );
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.key_test_focus {
        return match key.code {
            KeyCode::Esc => {
                app.key_test_focus = false;
                true
            }
            KeyCode::Tab => {
                app.key_test_focus = false;
                app.focus_buttons_on_primary();
                true
            }
            _ if crate::screens::is_edit_key(&key) => {
                app.key_test.handle_event(&Event::Key(key));
                true
            }
            _ => false,
        };
    }
    match key.code {
        KeyCode::Tab => {
            app.key_test_focus = true;
            true
        }
        KeyCode::Enter => {
            if app.focus_buttons {
                return false;
            }
            if let Some(index) = app.keyboards.as_ref().and_then(|p| p.selected) {
                app.choose_keyboard(index);
                app.focus_buttons_on_primary();
                true
            } else {
                false
            }
        }
        _ => app
            .keyboards
            .as_mut()
            .map(|picker| super::picker::key(picker, key))
            .unwrap_or(false),
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("loadkeys {}", app.keyboard_layout),
        format!("settings.nix: keyboardLayout = \"{}\";", app.keyboard_layout),
    ]
}
