//! Small shared widgets: buttons, editable fields, list rows and the dialog
//! dimmer. Everything is a single text size, rounded borders, and marks focus
//! with a cell background color.

use crate::theme::Palette;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
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

/// Renders a centered button row.
pub fn render_buttons(f: &mut Frame, area: Rect, palette: &Palette, buttons: &[Btn], focus: usize) {
    let mut spans: Vec<Span> = Vec::new();
    for (i, button) in buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        let focused = i == focus;
        let style = match button.kind {
            Kind::Primary => Style::default()
                .fg(palette.white)
                .bg(palette.primary)
                .add_modifier(Modifier::BOLD),
            Kind::Destructive => Style::default()
                .fg(palette.white)
                .bg(palette.error)
                .add_modifier(Modifier::BOLD),
            Kind::Secondary => {
                if focused && button.enabled {
                    Style::default()
                        .fg(palette.white)
                        .bg(palette.row_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.muted)
                }
            }
        };
        let style = if button.enabled {
            style
        } else {
            Style::default().fg(palette.border)
        };
        // Pad so the background reads as a button, not tight text.
        spans.push(Span::styled(format!(" {} ", button.label), style));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

/// An editable field with a block cursor. `masked` shows bullets.
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

pub fn checkbox(palette: &Palette, checked: bool, label: &str, focused: bool) -> Line<'static> {
    let mark = if checked { "[x]" } else { "[ ]" };
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
