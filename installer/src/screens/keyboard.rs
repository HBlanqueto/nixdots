//! "Teclado": a searchable list of layouts in the same centered style as
//! Lenguaje, the language's suggestion highlighted, and a test field under a
//! separator.

use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    (super::picker::ROWS + 7) as u16
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let box_w = super::picker::CENTERED_W
        .min(area.width.saturating_sub(4))
        .max(24);
    let box_h = (super::picker::ROWS as u16 + 2)
        .min(area.height.saturating_sub(9))
        .max(4);

    // heading + search + box + gray + separator + prueba + buttons
    let total = 1 + 1 + box_h + 1 + 1 + 1 + 2;
    let mut y = area.y + area.height.saturating_sub(total) / 2 + 1;

    f.render_widget(
        Paragraph::new(Span::styled(
            "Teclado",
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    if let Some(picker) = &app.keyboards {
        let search = super::picker::search_line(
            &app.palette,
            picker,
            "buscar distribución",
            "distribuciones",
            box_w,
            !app.focus_buttons && !app.key_test_focus,
        );
        f.render_widget(Paragraph::new(search), Rect::new(box_x, y, box_w, 1));
    }
    y += 1;

    if let Some(picker) = &app.keyboards {
        super::picker::draw_centered(
            f,
            &app.palette,
            Rect::new(box_x, y, box_w, box_h),
            picker,
            app.keyboard_suggested,
        );
    }
    y += box_h + 1;

    // The layout currently applied, in gray.
    f.render_widget(
        Paragraph::new(Span::styled(
            app.keyboard_layout.clone(),
            Style::default().fg(app.palette.muted),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    // Separator above the test field, spanning the box.
    f.render_widget(
        Paragraph::new(Span::styled(
            "─".repeat(box_w as usize),
            Style::default().fg(app.palette.row_bg),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    let test_focus = app.key_test_focus && !app.focus_buttons;
    let field = widgets::input_line(
        &app.palette,
        "prueba",
        app.key_test.value(),
        app.key_test.visual_cursor(),
        test_focus,
        false,
        box_w,
        true,
        "escribe aquí para probar el teclado",
    );
    f.render_widget(
        Paragraph::new(field).alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );

    app.draw_buttons(
        f,
        Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 1),
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
