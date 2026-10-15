//! Small shared widgets: buttons, editable fields, list rows and the dialog
//! dimmer. Everything is a single text size, rounded borders, and marks focus
//! with a cell background color.

use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct Btn {
    pub label: String,
    pub enabled: bool,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }
}

/// Dims the whole frame behind a dialog: every cell keeps its glyph but loses
/// the bold weight, takes the muted foreground and the base background, so the
/// screen (logo, text, buttons) is still legible underneath. There is no alpha
/// in a terminal.
pub fn dim_buffer(f: &mut Frame, palette: &Palette) {
    let area = f.area();
    let buf = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = palette.disabled;
                cell.bg = palette.reset;
                cell.modifier.remove(Modifier::BOLD);
            }
        }
    }
}

/// Dims the frame and draws a centered dialog: one rounded accent border on
/// the well background. `content` fills the inner rectangle.
pub fn draw_dialog(
    f: &mut Frame,
    area: Rect,
    palette: &Palette,
    width: u16,
    height: u16,
    content: impl FnOnce(&mut Frame, Rect),
) {
    dim_buffer(f, palette);
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(palette.primary))
        .style(Style::default().bg(palette.well));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    content(f, inner);
}

/// A centered row of buttons, shared by every screen and dialog.
///
/// Every button is plain muted text by default; only the focused one becomes a
/// capsule with the accent fill, white bold text (Nerd Font rounded ends on a
/// real terminal, `‹ text ›` on the Linux console). A disabled button is dim
/// and never takes the focus.
pub struct ButtonRow<'a> {
    pub buttons: &'a [Btn],
    pub focus: usize,
    pub focused: bool,
    pub palette: &'a Palette,
}

impl Widget for ButtonRow<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nerd = !crate::theme::console::is_linux_console();
        let mut flat: Vec<Span<'static>> = Vec::new();
        for (i, button) in self.buttons.iter().enumerate() {
            if i > 0 {
                flat.push(Span::raw("   "));
            }
            let is_focus = self.focused && i == self.focus && button.enabled;
            if !button.enabled {
                let style = Style::default().fg(self.palette.disabled);
                push_button(&mut flat, &button.label, style, style, nerd, false);
            } else if is_focus {
                let glyph = Style::default()
                    .fg(self.palette.primary)
                    .bg(self.palette.reset);
                let text = Style::default()
                    .fg(self.palette.white)
                    .bg(self.palette.primary)
                    .add_modifier(Modifier::BOLD);
                push_button(&mut flat, &button.label, glyph, text, nerd, true);
            } else {
                let style = Style::default().fg(self.palette.muted);
                push_button(&mut flat, &button.label, style, style, nerd, false);
            }
        }
        Paragraph::new(Line::from(flat))
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

/// Appends one button's spans. The capsule ends are only drawn when focused;
/// otherwise spaces keep the width, so selecting a button nothing moves.
fn push_button(
    flat: &mut Vec<Span<'static>>,
    label: &str,
    glyph: Style,
    text: Style,
    nerd: bool,
    capsule: bool,
) {
    if nerd {
        let (left, right) = if capsule {
            ("\u{e0b6}", "\u{e0b4}")
        } else {
            (" ", " ")
        };
        flat.push(Span::styled(left.to_string(), glyph));
        flat.push(Span::styled(format!(" {label} "), text));
        flat.push(Span::styled(right.to_string(), glyph));
    } else {
        flat.push(Span::styled(format!("‹ {label} ›"), text));
    }
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
