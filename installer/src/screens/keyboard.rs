//! "Teclado": a full-width list of layouts with a hidden `/` search, the
//! language's suggestion highlighted, and a test field under a separator.

use crate::app::{Action, App, Click, PickerKind, ScrollKind};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

const ROWS: usize = 8;
const BOX_W: u16 = 78;

pub fn height(_app: &App) -> u16 {
    (ROWS + 5) as u16
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let box_w = BOX_W.min(area.width.saturating_sub(4)).max(24);
    let box_h = (ROWS as u16 + 2).min(area.height.saturating_sub(8)).max(4);
    let search_h: u16 = if app.keyboard_search { 1 } else { 0 };
    // search + box + gap + separator + test + buttons
    let total = search_h + box_h + 1 + 1 + 1 + 3;
    let mut y = area.y + area.height.saturating_sub(total) / 2 + 1;

    if app.keyboard_search {
        let query = app
            .keyboards
            .as_ref()
            .map(|p| p.query.value().to_string())
            .unwrap_or_default();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("/ ", Style::default().fg(app.palette.primary)),
                Span::styled(query, Style::default().fg(app.palette.text)),
            ]))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    draw_list(f, app, Rect::new(box_x, y, box_w, box_h));
    y += box_h + 1;

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

    app.draw_buttons_at(f, Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 1));
}

fn draw_list(f: &mut Frame, app: &App, rect: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.muted))
        .title(Span::styled(
            " Teclado ",
            Style::default().fg(app.palette.muted),
        ));
    let inner = block.inner(rect);
    f.render_widget(Clear, rect);
    f.render_widget(block, rect);
    let Some(picker) = &app.keyboards else {
        return;
    };

    let total = picker.filtered.len();
    // With nothing chosen, scroll to the suggested layout so it is visible
    // (it stays unselected).
    let anchor = picker.selected.or_else(|| {
        app.keyboard_suggested
            .and_then(|s| picker.filtered.iter().position(|&i| i == s))
    });
    let start = anchor
        .unwrap_or(0)
        .saturating_sub(ROWS / 2)
        .min(total.saturating_sub(ROWS));
    let text_w = inner.width.saturating_sub(1) as usize;

    for row in 0..ROWS as u16 {
        if inner.y + row >= inner.bottom() {
            break;
        }
        let row_rect = Rect::new(inner.x, inner.y + row, inner.width, 1);
        app.targets.borrow_mut().push((
            row_rect,
            Click::PickerRow(PickerKind::Keyboard, start + row as usize),
        ));
        let Some(&item_idx) = picker.filtered.get(start + row as usize) else {
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row as usize);
        let suggested = app.keyboard_suggested == Some(item_idx);
        let style = if selected {
            Style::default()
                .fg(app.palette.white)
                .bg(app.palette.primary)
                .add_modifier(Modifier::BOLD)
        } else if suggested {
            Style::default()
                .fg(app.palette.focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.palette.text)
        };
        let name: String = item.name.chars().take(text_w).collect();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(name, style),
            ])),
            row_rect,
        );
    }

    draw_scrollbar(f, app, inner, start, total);
}

fn draw_scrollbar(f: &mut Frame, app: &App, inner: Rect, start: usize, total: usize) {
    if total <= ROWS || inner.width == 0 {
        return;
    }
    let x = inner.right().saturating_sub(1);
    let rows = ROWS.min(inner.height as usize);
    let thumb_h = (rows * rows / total).max(1);
    let travel = rows.saturating_sub(thumb_h);
    let max_start = total.saturating_sub(rows);
    let thumb_top = start.saturating_mul(travel).checked_div(max_start).unwrap_or(0);
    for row in 0..rows {
        let filled = row >= thumb_top && row < thumb_top + thumb_h;
        let (ch, style) = if filled {
            ("█", Style::default().fg(app.palette.primary))
        } else {
            ("│", Style::default().fg(app.palette.row_bg))
        };
        f.render_widget(
            Paragraph::new(Span::styled(ch, style)),
            Rect::new(x, inner.y + row as u16, 1, 1),
        );
    }
    app.scrollbars.borrow_mut().push((
        Rect::new(x, inner.y, 1, rows as u16),
        ScrollKind::Picker(PickerKind::Keyboard),
    ));
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
    if app.keyboard_search {
        return search_key(app, key);
    }
    match key.code {
        KeyCode::Char('/') => {
            app.keyboard_search = true;
            true
        }
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
        KeyCode::Up => {
            move_cursor(app, -1);
            true
        }
        KeyCode::Down => {
            move_cursor(app, 1);
            true
        }
        KeyCode::PageUp => {
            move_cursor(app, -(ROWS as isize));
            true
        }
        KeyCode::PageDown => {
            move_cursor(app, ROWS as isize);
            true
        }
        _ => false,
    }
}

fn search_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.keyboard_search = false;
            if let Some(p) = app.keyboards.as_mut() {
                p.query = tui_input::Input::default();
                p.refilter();
            }
            true
        }
        KeyCode::Enter => {
            app.keyboard_search = false;
            if let Some(index) = app.keyboards.as_ref().and_then(|p| p.selected) {
                app.choose_keyboard(index);
                app.focus_buttons_on_primary();
            }
            true
        }
        KeyCode::Up => {
            move_cursor(app, -1);
            true
        }
        KeyCode::Down => {
            move_cursor(app, 1);
            true
        }
        KeyCode::PageUp => {
            move_cursor(app, -(ROWS as isize));
            true
        }
        KeyCode::PageDown => {
            move_cursor(app, ROWS as isize);
            true
        }
        _ if crate::screens::is_edit_key(&key) => {
            if let Some(p) = app.keyboards.as_mut() {
                p.query.handle_event(&Event::Key(key));
                p.refilter();
            }
            true
        }
        _ => false,
    }
}

/// Moves the list cursor without applying the layout.
fn move_cursor(app: &mut App, delta: isize) {
    let total = app.keyboards.as_ref().map(|p| p.filtered.len()).unwrap_or(0);
    if total == 0 {
        return;
    }
    let current = app
        .keyboards
        .as_ref()
        .and_then(|p| p.selected)
        .map(|s| s as isize)
        .unwrap_or(if delta > 0 { -1 } else { 0 });
    let index = (current + delta).clamp(0, total as isize - 1) as usize;
    if let Some(picker) = app.keyboards.as_mut() {
        picker.selected = Some(index);
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
        format!("loadkeys {}", app.keyboard_layout),
        format!("settings.nix: keyboardLayout = \"{}\";", app.keyboard_layout),
    ]
}
