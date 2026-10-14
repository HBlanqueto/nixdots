//! An in-TUI, full-screen license reader: word-wrapped text, a scrollbar and
//! incremental search with highlighting.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::theme::Palette;

pub struct LicenseViewer {
    /// Original lines of the LICENSE file (progress is reported on these).
    source: Vec<String>,
    /// Word-wrapped display lines.
    wrapped: Vec<String>,
    /// For each wrapped line, the 1-based source line it came from.
    source_of: Vec<usize>,
    width: usize,
    pub scroll: usize,
    pub query: String,
    pub searching: bool,
    /// Wrapped-line indices matching `query`.
    pub matches: Vec<usize>,
    pub current_match: Option<usize>,
}

impl LicenseViewer {
    pub fn new(text: &str, width: usize) -> Self {
        let width = width.max(8);
        let source: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        let mut viewer = Self {
            source,
            wrapped: Vec::new(),
            source_of: Vec::new(),
            width,
            scroll: 0,
            query: String::new(),
            searching: false,
            matches: Vec::new(),
            current_match: None,
        };
        viewer.rewrap();
        viewer
    }

    pub fn set_width(&mut self, width: usize) {
        let width = width.max(8);
        if width != self.width {
            self.width = width;
            self.rewrap();
            self.recompute_matches();
        }
    }

    fn rewrap(&mut self) {
        self.wrapped.clear();
        self.source_of.clear();
        for (i, line) in self.source.iter().enumerate() {
            let wrapped = wrap(line, self.width);
            if wrapped.is_empty() {
                self.wrapped.push(String::new());
                self.source_of.push(i + 1);
            } else {
                for chunk in wrapped {
                    self.wrapped.push(chunk);
                    self.source_of.push(i + 1);
                }
            }
        }
        self.clamp();
    }

    pub fn total_lines(&self) -> usize {
        self.source.len()
    }

    pub fn total_wrapped(&self) -> usize {
        self.wrapped.len()
    }

    pub fn current_source_line(&self) -> usize {
        self.source_of.get(self.scroll).copied().unwrap_or(1)
    }

    /// Source line of the last wrapped line (the end of the file).
    pub fn last_source_line(&self) -> usize {
        self.source_of.last().copied().unwrap_or(1)
    }

    pub fn max_scroll(&self, visible: usize) -> usize {
        self.wrapped.len().saturating_sub(visible)
    }

    fn clamp(&mut self) {
        if self.scroll >= self.wrapped.len() {
            self.scroll = self.wrapped.len().saturating_sub(1);
        }
    }

    pub fn scroll_by(&mut self, delta: isize, visible: usize) {
        let max = self.max_scroll(visible) as isize;
        self.scroll = (self.scroll as isize + delta).clamp(0, max) as usize;
    }

    pub fn home(&mut self) {
        self.scroll = 0;
    }

    pub fn end(&mut self, visible: usize) {
        self.scroll = self.max_scroll(visible);
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.recompute_matches();
        if let Some(first) = self.matches.first().copied() {
            self.current_match = Some(0);
            self.scroll = first;
        } else {
            self.current_match = None;
        }
    }

    fn recompute_matches(&mut self) {
        self.matches.clear();
        self.current_match = None;
        if self.query.is_empty() {
            return;
        }
        let needle = self.query.to_lowercase();
        for (i, line) in self.wrapped.iter().enumerate() {
            if line.to_lowercase().contains(&needle) {
                self.matches.push(i);
            }
        }
    }

    pub fn next_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        let next = self.current_match.map_or(0, |c| (c + 1) % self.matches.len());
        self.current_match = Some(next);
        self.scroll = self.matches[next];
    }

    pub fn prev_match(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        let prev = self
            .current_match
            .map_or(self.matches.len() - 1, |c| (c + self.matches.len() - 1) % self.matches.len());
        self.current_match = Some(prev);
        self.scroll = self.matches[prev];
    }

    pub fn visible_lines(&self, visible: usize) -> Vec<(usize, String)> {
        self.wrapped
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible)
            .map(|(i, l)| (i, l.clone()))
            .collect()
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, palette: &Palette) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.primary))
            .title(Span::styled(
                " Licencia · GNU GPL v2.0 ",
                Style::default().fg(palette.primary),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let visible = inner.height as usize;
        let needle = self.query.to_lowercase();
        let lines: Vec<Line> = self
            .visible_lines(visible)
            .into_iter()
            .map(|(i, text)| {
                let is_match = !needle.is_empty() && text.to_lowercase().contains(&needle);
                let is_current = self.current_match.map(|c| self.matches.get(c) == Some(&i)).unwrap_or(false);
                let style = if is_current {
                    Style::default()
                        .fg(palette.on_primary)
                        .bg(palette.primary)
                        .add_modifier(Modifier::BOLD)
                } else if is_match {
                    Style::default()
                        .fg(palette.on_primary)
                        .bg(palette.warning)
                } else {
                    Style::default().fg(palette.text)
                };
                Line::from(Span::styled(text, style))
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);

        // Scrollbar, primary on lbg.
        let mut state = ScrollbarState::new(self.wrapped.len())
            .position(self.scroll)
            .viewport_content_length(visible);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(palette.border).bg(palette.well))
            .thumb_style(Style::default().fg(palette.primary).bg(palette.row_bg));
        f.render_stateful_widget(scrollbar, inner, &mut state);

        // Progress bottom-right and the allowUnfree note bottom-left.
        let total = self.total_lines().max(1);
        let line = self.current_source_line();
        let percent = (line * 100) / total;
        let bottom = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
        let progress = format!("línea {line}/{total} · {percent}%");
        let mut spans = vec![Span::styled(
            "incluye paquetes no libres (allowUnfree)",
            Style::default().fg(palette.muted),
        )];
        // Right-align the progress by padding.
        let pad = (area.width as usize)
            .saturating_sub("incluye paquetes no libres (allowUnfree)".len())
            .saturating_sub(progress.chars().count())
            .saturating_sub(2);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.push(Span::styled(progress, Style::default().fg(palette.muted)));
        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(palette.reset)),
            bottom,
        );

        if self.searching || !self.query.is_empty() {
            let search = Rect::new(area.x + 1, area.y, area.width.saturating_sub(2), 1);
            let text = format!("/{}", self.query);
            f.render_widget(
                Paragraph::new(Span::styled(
                    text,
                    Style::default().fg(palette.focus).bg(palette.well),
                )),
                search,
            );
        }
    }
}

fn wrap(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut count = 0;
    for word in line.split_inclusive(' ') {
        let wlen = word.chars().count();
        if count + wlen > width && count > 0 {
            out.push(std::mem::take(&mut current));
            count = 0;
        }
        if wlen > width {
            // Long token: hard-break it.
            for ch in word.chars() {
                if count == width {
                    out.push(std::mem::take(&mut current));
                    count = 0;
                }
                current.push(ch);
                count += 1;
            }
        } else {
            current.push_str(word);
            count += wlen;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn license_text() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../LICENSE"),
        )
        .unwrap_or_else(|_| "GNU GENERAL PUBLIC LICENSE\nVersion 2, June 1991\n".to_string())
    }

    #[test]
    fn loads_and_scrolls_to_end() {
        let text = license_text();
        assert!(text.contains("GNU"));
        let mut viewer = LicenseViewer::new(&text, 80);
        assert!(viewer.total_lines() > 10);
        viewer.end(20);
        assert_eq!(viewer.scroll, viewer.max_scroll(20));
        assert_eq!(viewer.last_source_line(), viewer.total_lines());
    }

    #[test]
    fn search_finds_gnu() {
        let text = license_text();
        let mut viewer = LicenseViewer::new(&text, 80);
        viewer.set_query("GNU".into());
        assert!(!viewer.matches.is_empty(), "la búsqueda de «GNU» no encontró nada");
        let first = viewer.scroll;
        viewer.next_match();
        // With several matches the cursor advances (or wraps to the first).
        assert!(viewer.current_match.is_some());
        assert!(viewer.scroll >= first || viewer.matches.len() == 1);
    }

    #[test]
    fn wrap_breaks_long_lines() {
        let wrapped = wrap(&"x".repeat(50), 10);
        assert_eq!(wrapped.len(), 5);
        assert!(wrapped.iter().all(|l| l.chars().count() <= 10));
    }
}
