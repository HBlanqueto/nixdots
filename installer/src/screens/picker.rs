//! Shared rendering and key handling for the searchable list pickers used by
//! the timezone and locale screens. The search field is always visible with a
//! placeholder; the list starts at the top with nothing preselected.

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

pub fn draw(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    placeholder: &str,
    unit: &str,
    focused: bool,
) {
    let mut lines: Vec<Line> = Vec::new();

    let mut search: Vec<Span> = vec![Span::styled(
        "› ",
        Style::default().fg(if focused { palette.primary } else { palette.muted }),
    )];
    if picker.query.value().is_empty() {
        if focused {
            search.push(Span::styled(
                " ",
                Style::default().fg(palette.white).bg(palette.primary),
            ));
        }
        search.push(Span::styled(placeholder.to_string(), Style::default().fg(palette.muted)));
    } else {
        let value = picker.query.value();
        let cursor = picker.query.visual_cursor();
        let chars: Vec<char> = value.chars().collect();
        let cursor = cursor.min(chars.len());
        let before: String = chars[..cursor].iter().collect();
        let at: char = chars.get(cursor).copied().unwrap_or(' ');
        let after: String = chars.get(cursor + 1..).unwrap_or(&[]).iter().collect();
        search.push(Span::styled(before, Style::default().fg(palette.text)));
        if focused {
            search.push(Span::styled(
                at.to_string(),
                Style::default().fg(palette.white).bg(palette.primary),
            ));
        } else {
            search.push(Span::styled(at.to_string(), Style::default().fg(palette.text)));
        }
        search.push(Span::styled(after, Style::default().fg(palette.text)));
    }
    // Right-aligned total on the search row (e.g. "418 zonas").
    let total = picker.filtered.len();
    let total_text = format!("{total} {unit}");
    let used: usize = search.iter().map(|s| s.content.chars().count()).sum();
    let pad = (area.width as usize)
        .saturating_sub(used)
        .saturating_sub(total_text.chars().count());
    search.push(Span::raw(" ".repeat(pad)));
    search.push(Span::styled(total_text, Style::default().fg(palette.muted)));
    lines.push(Line::from(search));

    let start = picker
        .selected
        .unwrap_or(0)
        .saturating_sub(ROWS / 2)
        .min(total.saturating_sub(ROWS));
    for row in 0..ROWS {
        let Some(&item_idx) = picker.filtered.get(start + row) else {
            lines.push(Line::from(""));
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row);
        let text = format!("{:<28}", item.code);
        let trailing = if item.extra.is_empty() {
            format!("  {}", item.name)
        } else {
            format!("{:<10} {}", item.extra, item.name)
        };
        lines.push(widgets::list_row(palette, selected, text, Some(trailing)));
    }

    f.render_widget(Paragraph::new(lines), area);
}

/// Handles typing, filtering and list navigation. Returns true if consumed.
pub fn key(picker: &mut Picker, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            picker.up();
            true
        }
        KeyCode::Down => {
            picker.down();
            true
        }
        _ if super::is_edit_key(&key) => {
            picker.query.handle_event(&Event::Key(key));
            picker.refilter();
            true
        }
        _ => false,
    }
}
