//! Shared rendering and key handling for the searchable list pickers used by
//! the timezone and locale screens. The search field is always visible with a
//! placeholder; the list starts at the top with nothing preselected.

use crate::app::Picker;
use crate::theme::Palette;
use crate::widgets;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub const ROWS: usize = 8;
/// Width of the centered list box used by Lenguaje and Teclado.
pub const CENTERED_W: u16 = 46;

pub fn height() -> u16 {
    ROWS as u16 + 1
}

/// The always-visible search row: `› query` on the left, `N unit` on the
/// right, with a block cursor while `focused`.
pub fn search_line(
    palette: &Palette,
    picker: &Picker,
    placeholder: &str,
    unit: &str,
    width: u16,
    focused: bool,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        "› ",
        Style::default().fg(if focused { palette.primary } else { palette.muted }),
    )];
    let value = picker.query.value();
    if value.is_empty() {
        if focused {
            spans.push(Span::styled(
                " ",
                Style::default().fg(palette.white).bg(palette.primary),
            ));
        }
        spans.push(Span::styled(
            placeholder.to_string(),
            Style::default().fg(palette.muted),
        ));
    } else {
        let chars: Vec<char> = value.chars().collect();
        let cursor = picker.query.visual_cursor().min(chars.len());
        let before: String = chars[..cursor].iter().collect();
        let at: char = chars.get(cursor).copied().unwrap_or(' ');
        let after: String = chars.get(cursor + 1..).unwrap_or(&[]).iter().collect();
        spans.push(Span::styled(before, Style::default().fg(palette.text)));
        let at_style = if focused {
            Style::default().fg(palette.white).bg(palette.primary)
        } else {
            Style::default().fg(palette.text)
        };
        spans.push(Span::styled(at.to_string(), at_style));
        spans.push(Span::styled(after, Style::default().fg(palette.text)));
    }
    let total = picker.filtered.len();
    let total_text = format!("{total} {unit}");
    let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
    let pad = (width.saturating_sub(1) as usize)
        .saturating_sub(used)
        .saturating_sub(total_text.chars().count());
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(total_text, Style::default().fg(palette.muted)));
    Line::from(spans)
}

/// A centered, searchable list in a rounded box (no border title), with
/// centered names, an accent capsule on the chosen row and a `█`/`│`
/// scrollbar. `suggested` highlights a row without selecting it.
pub fn draw_centered(
    f: &mut Frame,
    palette: &Palette,
    area: Rect,
    picker: &Picker,
    suggested: Option<usize>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.muted));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let total = picker.filtered.len();
    // With nothing chosen, scroll to the suggested row so it is visible (it
    // stays unselected).
    let anchor = picker.selected.or_else(|| {
        suggested.and_then(|s| picker.filtered.iter().position(|&i| i == s))
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
        let Some(&item_idx) = picker.filtered.get(start + row as usize) else {
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row as usize);
        let is_suggested = suggested == Some(item_idx);
        let style = if selected {
            Style::default()
                .fg(palette.white)
                .bg(palette.primary)
                .add_modifier(Modifier::BOLD)
        } else if is_suggested {
            Style::default().fg(palette.focus).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text)
        };
        let name: String = item.name.chars().take(text_w).collect();
        let pad = text_w.saturating_sub(name.chars().count()) / 2;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" ".repeat(pad)),
                Span::styled(name, style),
            ])),
            row_rect,
        );
    }

    // Scrollbar.
    if total > ROWS && inner.width > 0 {
        let x = inner.right().saturating_sub(1);
        let rows = ROWS.min(inner.height as usize);
        let thumb_h = (rows * rows / total).max(1);
        let travel = rows.saturating_sub(thumb_h);
        let max_start = total.saturating_sub(rows);
        let thumb_top = start
            .saturating_mul(travel)
            .checked_div(max_start)
            .unwrap_or(0);
        for row in 0..rows {
            let filled = row >= thumb_top && row < thumb_top + thumb_h;
            let (ch, style) = if filled {
                ("█", Style::default().fg(palette.primary))
            } else {
                ("│", Style::default().fg(palette.row_bg))
            };
            f.render_widget(
                Paragraph::new(Span::styled(ch, style)),
                Rect::new(x, inner.y + row as u16, 1, 1),
            );
        }
    }
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
        lines.push(widgets::list_row(palette, selected, text, Some(trailing)));
    }

    // Leave the last column for a scrollbar, like the license text.
    let content = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
    f.render_widget(Paragraph::new(lines), content);

    if total > ROWS {
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
        KeyCode::PageUp => {
            picker.move_by(-(ROWS as isize));
            true
        }
        KeyCode::PageDown => {
            picker.move_by(ROWS as isize);
            true
        }
        KeyCode::Home => {
            picker.selected = (!picker.filtered.is_empty()).then_some(0);
            true
        }
        KeyCode::End => {
            picker.selected = picker.filtered.len().checked_sub(1);
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
