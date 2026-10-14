use crate::app::App;
use crate::install::Step;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const SPINNER: [&str; 8] = ["⢿", "⣻", "⣽", "⣾", "⣷", "⣯", "⣟", "⡿"];

pub fn height(app: &App) -> u16 {
    if app.install.error.is_some() {
        6
    } else {
        4
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let current = app.install.current;
    let spinner = SPINNER[(app.tick as usize / 2) % SPINNER.len()];

    let step_text = current.map(|s| s.label()).unwrap_or("iniciando");
    let note = app
        .install
        .note
        .clone()
        .unwrap_or_else(|| format!("{step_text}…"));

    let mut lines = vec![
        Line::from(Span::styled(
            step_text.to_string(),
            Style::default().fg(app.palette.primary),
        )),
        Line::from(vec![
            Span::styled(format!("{spinner} "), Style::default().fg(app.palette.focus)),
            Span::styled(note, Style::default().fg(app.palette.text)),
        ]),
    ];

    // Progress bar with percentage and n/total.
    let total = app.install.total.max(1);
    let built = app.install.built.min(total);
    let width = area.width.saturating_sub(12).max(4) as usize;
    let filled = ((built as f64 / total as f64) * width as f64).round() as usize;
    let percent = (built as f64 / total as f64 * 100.0).round() as u64;
    lines.push(Line::from(vec![
        Span::styled("█".repeat(filled), Style::default().fg(app.palette.primary)),
        Span::styled(
            "░".repeat(width.saturating_sub(filled)),
            Style::default().fg(app.palette.muted),
        ),
        Span::styled(
            format!("  {percent}% {built}/{total}"),
            Style::default().fg(app.palette.muted),
        ),
    ]));

    // Per-step status line.
    let mut step_spans: Vec<Span> = Vec::new();
    for (i, step) in Step::ALL.iter().enumerate() {
        if i > 0 {
            step_spans.push(Span::raw("  "));
        }
        let (mark, color) = if app.install.done.contains(step) {
            ("✓", app.palette.success)
        } else if Some(*step) == current {
            ("●", app.palette.focus)
        } else {
            ("○", app.palette.muted)
        };
        step_spans.push(Span::styled(
            format!("{mark} {}", step.label()),
            Style::default().fg(color),
        ));
    }
    lines.push(Line::from(step_spans));

    if let Some(error) = &app.install.error {
        lines.push(Line::from(Span::styled(
            format!("error: {error}"),
            Style::default().fg(app.palette.error),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}

/// No per-screen keys: Ctrl+C (handled globally) is the only action.
pub fn key(_app: &App, _key: KeyEvent) {}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
