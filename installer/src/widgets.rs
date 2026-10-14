//! Small shared widgets: buttons, editable fields, list rows and the dialog
//! dimmer. Everything is a single text size, rounded borders, and marks focus
//! with a cell background color.

use crate::theme::Palette;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Destructive,
}

#[derive(Debug, Clone)]
pub struct Btn {
    pub label: String,
    pub kind: Kind,
    pub enabled: bool,
}

impl Btn {
    pub fn primary(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: Kind::Primary,
            enabled: true,
        }
    }

    pub fn secondary(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: Kind::Secondary,
            enabled: true,
        }
    }

    pub fn destructive(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            kind: Kind::Destructive,
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Dims a rectangle by re-styling its cells to the muted foreground color
/// (there is no alpha in a terminal).
pub fn dim(f: &mut Frame, area: Rect, palette: &Palette) {
    let buf = f.buffer_mut();
    let rect = area.intersection(*buf.area());
    for y in rect.top()..rect.bottom() {
        for x in rect.left()..rect.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = palette.muted;
            }
        }
    }
}

/// The middle text of a button, without its surrounding arrows.
fn btn_core(label: &str) -> String {
    let mut s = label.trim();
    if let Some(rest) = s.strip_prefix("‹ ") {
        s = rest;
    } else if let Some(rest) = s.strip_prefix('‹') {
        s = rest.trim_start();
    }
    if let Some(rest) = s.strip_suffix(" ›") {
        s = rest;
    } else if let Some(rest) = s.strip_suffix('›') {
        s = rest.trim_end();
    }
    s.trim().to_string()
}

fn fill_color(kind: Kind, palette: &Palette) -> Color {
    match kind {
        Kind::Primary => palette.primary,
        Kind::Secondary => palette.row_bg,
        Kind::Destructive => palette.error,
    }
}

/// Renders a centered button row.
///
/// Unfocused buttons are plain muted text. The focused button becomes a
/// capsule: Nerd Font rounded ends on a real terminal, or `‹ text ›` with a
/// flat fill on the Linux console. A disabled button is dim and takes no
/// focus.
/// Returns the on-screen x-ranges of the enabled buttons, for mouse hover.
pub fn render_buttons(
    f: &mut Frame,
    area: Rect,
    palette: &Palette,
    buttons: &[Btn],
    focus: usize,
    focused: bool,
) -> Vec<(u16, u16, usize)> {
    let nerd = !crate::theme::console::is_linux_console();
    // Each button becomes a group of spans; we remember the columns.
    let mut groups: Vec<(Vec<Span<'static>>, Option<usize>)> = Vec::new();
    for (i, button) in buttons.iter().enumerate() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        let is_focus = focused && i == focus && button.enabled;
        // Every button always has the capsule shape (same width), so selecting
        // it only changes the colors — nothing moves or jumps.
        let core = btn_core(&button.label);
        let (glyph_style, text_style) = if !button.enabled {
            // Disabled: dim, no focus, no response.
            (
                Style::default().fg(palette.disabled),
                Style::default().fg(palette.disabled),
            )
        } else if is_focus {
            let fill = fill_color(button.kind, palette);
            (
                Style::default().fg(fill).bg(palette.reset),
                Style::default()
                    .fg(palette.white)
                    .bg(fill)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            // Unfocused: camouflaged. The rounded ends are not drawn at all
            // (a space keeps the width), so nothing shows but the muted text.
            (Style::default(), Style::default().fg(palette.muted))
        };
        if nerd {
            // The round ends are only drawn when focused; otherwise spaces, so
            // the unfocused button blends with the background.
            let (left, right) = if is_focus {
                ("\u{e0b6}", "\u{e0b4}")
            } else {
                (" ", " ")
            };
            spans.push(Span::styled(left.to_string(), glyph_style));
            spans.push(Span::styled(format!(" {core} "), text_style));
            spans.push(Span::styled(right.to_string(), glyph_style));
        } else {
            spans.push(Span::styled(format!("‹ {core} ›"), text_style));
        }
        // Disabled buttons get no hitbox (mouse ignores them).
        groups.push((spans, if button.enabled { Some(i) } else { None }));
    }

    // Flatten for rendering, and record the hitbox of each enabled button.
    let mut flat: Vec<Span<'static>> = Vec::new();
    let mut hitboxes: Vec<(u16, u16, usize)> = Vec::new();
    for (group, index) in groups {
        // A group's first span may be the separator; the button spans follow.
        let group_width: u16 = group.iter().map(span_width).sum();
        let sep: u16 = group
            .first()
            .filter(|s| s.content == "   ")
            .map(|s| span_width(s))
            .unwrap_or(0);
        let start_col: u16 = flat.iter().map(span_width).sum();
        if let Some(i) = index {
            hitboxes.push((start_col + sep, start_col + group_width - 1, i));
        }
        flat.extend(group);
    }

    let total: u16 = flat.iter().map(span_width).sum();
    let offset = area.x + area.width.saturating_sub(total) / 2;
    let hitboxes = hitboxes
        .into_iter()
        .map(|(x0, x1, i)| (offset + x0, offset + x1, i))
        .collect();

    f.render_widget(
        Paragraph::new(Line::from(flat)).alignment(Alignment::Center),
        area,
    );
    hitboxes
}

fn span_width(span: &Span) -> u16 {
    span.content.chars().count() as u16
}

/// An editable field with a block cursor. `masked` shows bullets; an empty
/// field shows `placeholder` in muted until something is typed.
#[allow(clippy::too_many_arguments)]
pub fn input_line(
    palette: &Palette,
    label: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    masked: bool,
    width: u16,
    enabled: bool,
    placeholder: &str,
) -> Line<'static> {
    let marker = if focused { "› " } else { "  " };
    let label_style = if !enabled {
        Style::default().fg(palette.border)
    } else if focused {
        Style::default()
            .fg(palette.primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.muted)
    };
    let value_color = if !enabled {
        palette.muted
    } else if focused {
        palette.white
    } else {
        palette.text
    };

    if value.is_empty() {
        let mut spans = vec![
            Span::styled(
                marker,
                Style::default().fg(if focused && enabled {
                    palette.primary
                } else {
                    palette.muted
                }),
            ),
            Span::styled(format!("{label}: "), label_style),
        ];
        if focused && enabled {
            spans.push(Span::styled(
                " ",
                Style::default().fg(palette.white).bg(palette.primary),
            ));
        }
        if !placeholder.is_empty() {
            spans.push(Span::styled(
                placeholder.to_string(),
                Style::default().fg(palette.muted),
            ));
        }
        let mut line = Line::from(spans);
        if focused && enabled {
            line = line.style(Style::default().bg(palette.row_bg));
        }
        return line;
    }

    let chars: Vec<char> = if masked {
        vec!['•'; value.chars().count()]
    } else {
        value.chars().collect()
    };
    let cursor = cursor.min(chars.len());
    let available = width.saturating_sub(label.len() as u16 + 4).max(1) as usize;
    let start = if chars.len() > available {
        cursor.saturating_sub(available.saturating_sub(1))
    } else {
        0
    };
    let end = (start + available).min(chars.len());
    let window: Vec<char> = chars[start..end].to_vec();
    let cursor_in_window = cursor.saturating_sub(start);

    let before: String = window[..cursor_in_window.min(window.len())].iter().collect();
    let at: char = window.get(cursor_in_window).copied().unwrap_or(' ');
    let after: String = window
        .get(cursor_in_window + 1..)
        .unwrap_or(&[])
        .iter()
        .collect();

    let mut spans = vec![
        Span::styled(
            marker,
            Style::default().fg(if focused && enabled {
                palette.primary
            } else {
                palette.muted
            }),
        ),
        Span::styled(format!("{label}: "), label_style),
        Span::styled(before, Style::default().fg(value_color)),
    ];
    if focused && enabled {
        spans.push(Span::styled(
            at.to_string(),
            Style::default().fg(palette.white).bg(palette.primary),
        ));
    } else {
        spans.push(Span::styled(at.to_string(), Style::default().fg(value_color)));
    }
    spans.push(Span::styled(after, Style::default().fg(value_color)));
    let mut line = Line::from(spans);
    if focused && enabled {
        line = line.style(Style::default().bg(palette.row_bg));
    }
    line
}

/// The tick-box glyph: Nerd Font on a real terminal, ASCII on the Linux
/// console (`U+F096` empty / `U+F14A` ticked).
pub fn tick(checked: bool) -> &'static str {
    if crate::theme::console::is_linux_console() {
        if checked {
            "[x]"
        } else {
            "[ ]"
        }
    } else if checked {
        "\u{f14a}"
    } else {
        "\u{f096}"
    }
}

pub fn checkbox(palette: &Palette, checked: bool, label: &str, focused: bool) -> Line<'static> {
    let mark = tick(checked);
    let label_style = if focused && checked {
        // Focused+toggled rows read as selected.
        Style::default().fg(palette.text).bg(palette.well)
    } else {
        Style::default().fg(palette.text)
    };
    Line::from(vec![
        Span::styled(
            format!("{mark} "),
            Style::default().fg(if checked {
                palette.success
            } else {
                palette.muted
            }),
        ),
        Span::styled(label.to_string(), label_style),
    ])
}

/// A selectable list row: `›`, lbg background and white text when selected.
pub fn list_row(palette: &Palette, selected: bool, text: String, trailing: Option<String>) -> Line<'static> {
    let (marker, style) = if selected {
        (
            "› ",
            Style::default()
                .fg(palette.white)
                .bg(palette.row_bg)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("  ", Style::default().fg(palette.text))
    };
    let mut spans = vec![Span::styled(marker, style), Span::styled(text, style)];
    if let Some(trailing) = trailing {
        spans.push(Span::styled(
            trailing,
            Style::default().fg(if selected {
                palette.white
            } else {
                palette.muted
            }),
        ));
    }
    Line::from(spans)
}
