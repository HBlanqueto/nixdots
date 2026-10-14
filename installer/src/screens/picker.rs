//! Shared rendering and key handling for the searchable list pickers used by
//! the timezone and locale screens. The search field is always visible with a
//! placeholder; the list starts at the top with nothing preselected.

use crate::app::{Click, Picker, PickerKind, ScrollKind};
use crate::theme::Palette;
use crate::widgets;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub const ROWS: usize = 8;

pub fn height() -> u16 {
    ROWS as u16 + 1
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    placeholder: &str,
    unit: &str,
    focused: bool,
    kind: Option<PickerKind>,
    targets: Option<&std::cell::RefCell<Vec<(Rect, Click)>>>,
    scrollbars: Option<&std::cell::RefCell<Vec<(Rect, ScrollKind)>>>,
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
    let pad = (area.width.saturating_sub(1) as usize)
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
        if let (Some(kind), Some(targets)) = (kind, targets) {
            targets.borrow_mut().push((
                Rect::new(area.x, area.y + 1 + row as u16, area.width, 1),
                Click::PickerRow(kind, start + row),
            ));
        }
        lines.push(widgets::list_row(palette, selected, text, Some(trailing)));
    }

    // Leave the last column for a scrollbar, like the license text.
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    f.render_widget(Paragraph::new(lines), content);

    if total > ROWS {
        if let (Some(kind), Some(scrollbars)) = (kind, scrollbars) {
            scrollbars.borrow_mut().push((
                Rect::new(area.right().saturating_sub(1), area.y + 1, 1, ROWS as u16),
                ScrollKind::Picker(kind),
            ));
        }
        let bar_area = Rect::new(area.x, area.y + 1, area.width, ROWS as u16);
        let mut state = ScrollbarState::new(total)
            .position(start)
            .viewport_content_length(ROWS);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(palette.row_bg))
            .thumb_style(Style::default().fg(palette.primary));
        f.render_stateful_widget(scrollbar, bar_area, &mut state);
    }
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
