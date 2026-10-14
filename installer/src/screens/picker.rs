//! Shared rendering and key handling for the incremental-search list pickers
//! used by the timezone and locale screens.

use crate::app::Picker;
use crate::theme::Palette;
use crate::widgets;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub const ROWS: usize = 8;

pub fn height() -> u16 {
    ROWS as u16 + 1
}

pub fn draw(f: &mut Frame, palette: &Palette, area: Rect, picker: &Picker) {
    let total = picker.filtered.len();
    // Window around the selection.
    let start = picker
        .selected
        .saturating_sub(ROWS / 2)
        .min(total.saturating_sub(ROWS));
    let mut lines: Vec<Line> = Vec::new();
    for row in 0..ROWS {
        let Some(&item_idx) = picker.filtered.get(start + row) else {
            lines.push(Line::from(""));
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = start + row == picker.selected;
        let text = format!("{:<28}", item.code);
        let trailing = if item.extra.is_empty() {
            format!("  {}", item.name)
        } else {
            format!("{:<10} {}", item.extra, item.name)
        };
        lines.push(widgets::list_row(palette, selected, text, Some(trailing)));
    }
    let counter = if total == 0 {
        "0 de 0".to_string()
    } else {
        format!("{} de {}", picker.selected + 1, total)
    };
    lines.push(Line::from(Span::styled(counter, Style::default().fg(palette.muted))));
    f.render_widget(Paragraph::new(lines), area);
}

/// Draws the search prompt row (call after `draw`, on the counter row).
pub fn draw_search(f: &mut Frame, palette: &Palette, area: Rect, picker: &Picker) {
    if !picker.searching && picker.query.value().is_empty() {
        return;
    }
    let row = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    let text = format!("/{}", picker.query.value());
    f.render_widget(
        Paragraph::new(Span::styled(
            text,
            Style::default().fg(palette.focus).bg(palette.well),
        )),
        row,
    );
}

/// Handles navigation and incremental search. Returns true if consumed.
pub fn key(picker: &mut Picker, key: KeyEvent) -> bool {
    if picker.searching {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => picker.searching = false,
            _ => {
                picker.query.handle_event(&Event::Key(key));
            }
        }
        picker.refilter();
        return true;
    }
    match key.code {
        KeyCode::Up => {
            picker.up();
            true
        }
        KeyCode::Down => {
            picker.down();
            true
        }
        KeyCode::Char('/') => {
            picker.searching = true;
            picker.query = tui_input::Input::default();
            picker.refilter();
            true
        }
        _ => false,
    }
}
