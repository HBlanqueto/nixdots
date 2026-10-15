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
    /// A destructive action ("Borrar todo"): red text while unfocused.
    pub danger: bool,
}

impl Btn {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            danger: false,
        }
    }

    pub fn danger(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            danger: true,
        }
    }
}

/// The vertical layout shared by every wizard screen: a progress bar, a bold
/// title, a 46-column rounded box, an optional line outside it, the buttons and
/// the shortcut line. The shortcut line is dropped first on a short terminal;
/// the box and buttons are never hidden.
pub struct StepLayout {
    pub progress: Rect,
    pub title: Rect,
    pub body: Rect,
    pub inner: Rect,
    pub note: Rect,
    pub buttons: Rect,
    pub shortcuts: Rect,
    pub commands: Rect,
}

impl StepLayout {
    pub const WIDTH: u16 = 46;

    /// `body_h` is the full box height (content + two borders); `note_h`
    /// reserves lines between the box and the buttons; `commands_h` appends the
    /// `v` commands column.
    pub fn compute(area: Rect, body_h: u16, note_h: u16, commands_h: u16) -> Self {
        let width = Self::WIDTH.min(area.width);
        let x = area.x + area.width.saturating_sub(width) / 2;
        let gap: u16 = 1;
        let shortcuts_h = u16::from(area.height >= 30);
        let mut total = 1 + gap + 1 + gap + body_h + gap + 1;
        if note_h > 0 {
            total += note_h + gap;
        }
        if shortcuts_h > 0 {
            total += shortcuts_h + gap;
        }
        if commands_h > 0 {
            total += commands_h + gap;
        }
        let mut y = area.y + area.height.saturating_sub(total) / 2;

        let progress = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let title = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let body = Rect::new(x, y, width, body_h);
        let inner = Rect::new(
            body.x + 1,
            body.y + 1,
            body.width.saturating_sub(2),
            body.height.saturating_sub(2),
        );
        y += body_h + gap;
        let note = Rect::new(x, y, width, note_h);
        if note_h > 0 {
            y += note_h + gap;
        }
        let buttons = Rect::new(x, y, width, 1);
        y += 1 + gap;
        let shortcuts = Rect::new(x, y, width, shortcuts_h);
        if shortcuts_h > 0 {
            y += shortcuts_h + gap;
        }
        let commands = Rect::new(x, y, width, commands_h);

        Self {
            progress,
            title,
            body,
            inner,
            note,
            buttons,
            shortcuts,
            commands,
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
                let color = if button.danger {
                    self.palette.error
                } else {
                    self.palette.muted
                };
                let style = Style::default().fg(color);
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

/// An editable text field: a gray label on top and, below it, the value with an
/// underline that turns accent while the field has the focus. `masked` shows
/// bullets; an empty field shows `placeholder` in muted until something is
/// typed. Returns the two lines (label, value) to push inside the box.
#[allow(clippy::too_many_arguments)]
pub fn field(
    palette: &Palette,
    label: &str,
    value: &str,
    cursor: usize,
    focused: bool,
    masked: bool,
    width: u16,
    enabled: bool,
    placeholder: &str,
) -> Vec<Line<'static>> {
    let label_style = if !enabled {
        Style::default().fg(palette.disabled)
    } else {
        Style::default().fg(palette.muted)
    };
    let label_line = Line::from(Span::styled(label.to_string(), label_style));

    let rule = if !enabled {
        palette.disabled
    } else if focused {
        palette.primary
    } else {
        palette.rule
    };
    let underline = Style::default()
        .underline_color(rule)
        .add_modifier(Modifier::UNDERLINED);
    let empty = value.is_empty();
    let value_color = if !enabled {
        palette.disabled
    } else if empty {
        palette.muted
    } else if focused {
        palette.white
    } else {
        palette.text
    };

    let mut display: Vec<char> = if empty {
        placeholder.chars().collect()
    } else if masked {
        vec!['•'; value.chars().count()]
    } else {
        value.chars().collect()
    };
    let cursor = cursor.min(if empty { 0 } else { display.len() });
    let avail = width.max(1) as usize;
    let start = if display.len() > avail {
        cursor.saturating_sub(avail.saturating_sub(1))
    } else {
        0
    };
    let end = (start + avail).min(display.len());
    let mut window: Vec<char> = display.drain(start..end).collect();
    while window.len() < avail {
        window.push(' ');
    }
    let cw = cursor.saturating_sub(start);
    let before: String = window[..cw.min(window.len())].iter().collect();
    let at: char = window.get(cw).copied().unwrap_or(' ');
    let after: String = window.get(cw + 1..).unwrap_or(&[]).iter().collect();

    let mut spans = vec![Span::styled(
        before,
        Style::default().fg(value_color).patch(underline),
    )];
    if focused && enabled {
        spans.push(Span::styled(
            at.to_string(),
            Style::default()
                .fg(palette.on_primary)
                .bg(palette.primary)
                .patch(underline),
        ));
    } else {
        spans.push(Span::styled(
            at.to_string(),
            Style::default().fg(value_color).patch(underline),
        ));
    }
    spans.push(Span::styled(
        after,
        Style::default().fg(value_color).patch(underline),
    ));
    vec![label_line, Line::from(spans)]
}

/// A tick box: `[ ]` / `[x]`, the mark in accent, the label in text.
pub fn checkbox(palette: &Palette, checked: bool, label: &str, focused: bool) -> Line<'static> {
    let mark = if checked { "[x]" } else { "[ ]" };
    let mark_style = Style::default().fg(if checked {
        palette.primary
    } else {
        palette.muted
    });
    let label_style = if focused {
        Style::default().fg(palette.white)
    } else {
        Style::default().fg(palette.text)
    };
    Line::from(vec![
        Span::styled(format!("{mark} "), mark_style),
        Span::styled(label.to_string(), label_style),
    ])
}

/// A selectable list row: an accent capsule with white bold text when selected,
/// plain `#adbac7` otherwise. The trailing text is muted.
pub fn list_row(palette: &Palette, selected: bool, text: String, trailing: Option<String>) -> Line<'static> {
    let (marker, style) = if selected {
        (
            "› ",
            Style::default()
                .fg(palette.white)
                .bg(palette.primary)
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
