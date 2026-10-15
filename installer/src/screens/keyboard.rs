//! "Teclado": a searchable list of layouts in the shared box, the language's
//! suggestion highlighted, and a test field under the box.

use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.keyboards else {
        return;
    };
    let list = super::picker::draw_search(
        f,
        &app.palette,
        area,
        picker,
        !app.focus_buttons && !app.key_test_focus && !app.no_focus(),
    );
    // The test field sits at the bottom of the box, below the list.
    let test_y = area.bottom().saturating_sub(4);
    let test = widgets::text_field(
        &app.palette,
        "Prueba el teclado",
        app.key_test.value(),
        app.key_test.visual_cursor(),
        app.key_test_focus && !app.focus_buttons && !app.no_focus(),
        false,
        "ñ ¿ ¡ @ # á é",
        area.width,
        None,
        false,
    );
    f.render_widget(
        Paragraph::new(test),
        Rect::new(area.x, test_y, area.width, 4),
    );
    let list_h = test_y.saturating_sub(list.y + 1);
    super::picker::draw_centered(
        f,
        &app.palette,
        Rect::new(list.x, list.y, list.width, list_h),
        picker,
        app.keyboard_suggested,
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
