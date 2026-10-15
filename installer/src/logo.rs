//! The NixOS logo, embedded from `assets/nixos-logo.ans` and converted to
//! ratatui `Text`.
//!
//! The art uses the two official NixOS blues (#5277c3 dark, #7fb7ff light) —
//! the only colors outside the neptunia palette. On the Linux console they are
//! mapped to the palette slots we reprogram (c4 / c12).

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const DARK_RGB: (u8, u8, u8) = (82, 119, 195);
const LIGHT_RGB: (u8, u8, u8) = (127, 183, 255);

const LOGO: &str = include_str!("../../assets/nixos-logo.ans");

fn blue(rgb: (u8, u8, u8), console: bool) -> Color {
    if !console {
        return Color::Rgb(rgb.0, rgb.1, rgb.2);
    }
    // On the Linux console use the reprogrammed palette slots.
    if rgb == DARK_RGB {
        Color::Indexed(4)
    } else {
        Color::Indexed(12)
    }
}

/// Parses the embedded ANSI art into 16 terminal lines of `Span`s.
pub fn nixos_logo(console: bool) -> Vec<Line<'static>> {
    let dark = blue(DARK_RGB, console);
    let light = blue(LIGHT_RGB, console);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_style = Style::default();
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;

    let mut chars = LOGO.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            let mut seq = String::new();
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
                seq.push(n);
            }
            apply_sgr(&seq, &mut fg, &mut bg, dark, light);
            continue;
        }
        if c == '\n' {
            flush(&mut spans, &mut run, run_style);
            lines.push(Line::from(std::mem::take(&mut spans)));
            continue;
        }
        let style = Style::default().fg(fg).bg(bg);
        if style != run_style {
            flush(&mut spans, &mut run, run_style);
            run_style = style;
        }
        run.push(c);
    }
    flush(&mut spans, &mut run, run_style);
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    lines
}

fn flush(spans: &mut Vec<Span<'static>>, run: &mut String, style: Style) {
    if !run.is_empty() {
        spans.push(Span::styled(std::mem::take(run), style));
    }
}

/// The compact NixOS logo: 8 rows of half-blocks (`▀▄█`) derived from the same
/// art as [`nixos_logo`], two source rows per output row. Where the top and
/// bottom halves differ, `▀` carries fg = top colour and bg = bottom colour.
pub fn nixos_logo_compact(console: bool) -> Vec<Line<'static>> {
    let dark = blue(DARK_RGB, console);
    let light = blue(LIGHT_RGB, console);
    let grid = parse_grid(dark, light);

    let mut out: Vec<Line<'static>> = Vec::new();
    let mut r = 0;
    while r < grid.len() {
        let top = &grid[r];
        let bottom = grid.get(r + 1);
        let width = top
            .len()
            .max(bottom.map(|b| b.len()).unwrap_or(0));
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut run = String::new();
        let mut run_style = Style::default();
        for c in 0..width {
            let t = top.get(c).copied().flatten();
            let b = bottom.and_then(|row| row.get(c).copied().flatten());
            let (ch, style) = match (t, b) {
                (Some(tc), Some(bc)) if tc == bc => ('\u{2588}', Style::default().fg(tc)),
                (Some(tc), Some(bc)) => ('\u{2580}', Style::default().fg(tc).bg(bc)),
                (Some(tc), None) => ('\u{2580}', Style::default().fg(tc)),
                (None, Some(bc)) => ('\u{2584}', Style::default().fg(bc)),
                (None, None) => (' ', Style::default()),
            };
            if style != run_style {
                flush(&mut spans, &mut run, run_style);
                run_style = style;
            }
            run.push(ch);
        }
        flush(&mut spans, &mut run, run_style);
        out.push(Line::from(spans));
        r += 2;
    }
    out
}

/// Parses the ANSI art into a grid where each cell is `Some(colour)` when it is
/// painted (a glyph, or a background-filled space) and `None` when transparent.
fn parse_grid(dark: Color, light: Color) -> Vec<Vec<Option<Color>>> {
    let mut grid: Vec<Vec<Option<Color>>> = Vec::new();
    let mut row: Vec<Option<Color>> = Vec::new();
    let mut fg = Color::Reset;
    let mut bg = Color::Reset;

    let mut chars = LOGO.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            let mut seq = String::new();
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
                seq.push(n);
            }
            apply_sgr(&seq, &mut fg, &mut bg, dark, light);
            continue;
        }
        if c == '\n' {
            grid.push(std::mem::take(&mut row));
            continue;
        }
        let cell = if c == ' ' {
            if bg == Color::Reset {
                None
            } else {
                Some(bg)
            }
        } else {
            Some(fg)
        };
        row.push(cell);
    }
    if !row.is_empty() {
        grid.push(row);
    }
    grid
}

fn apply_sgr(seq: &str, fg: &mut Color, bg: &mut Color, dark: Color, light: Color) {
    // `seq` still carries the leading '[' of the CSI (ESC '[' ... 'm').
    let seq = seq.strip_prefix('[').unwrap_or(seq);
    let params: Vec<&str> = seq.split(';').collect();
    match params.as_slice() {
        [""] | ["0"] => {
            *fg = Color::Reset;
            *bg = Color::Reset;
        }
        ["38", "2", r, g, b] => {
            *fg = map_rgb(r, g, b, dark, light);
        }
        ["48", "2", r, g, b] => {
            *bg = map_rgb(r, g, b, dark, light);
        }
        _ => {}
    }
}

fn map_rgb(r: &str, g: &str, b: &str, dark: Color, light: Color) -> Color {
    match (r.parse().ok(), g.parse().ok(), b.parse().ok()) {
        (Some(r), Some(g), Some(b)) if (r, g, b) == DARK_RGB => dark,
        (Some(r), Some(g), Some(b)) if (r, g, b) == LIGHT_RGB => light,
        (Some(r), Some(g), Some(b)) => Color::Rgb(r, g, b),
        _ => Color::Reset,
    }
}
