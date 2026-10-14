use crate::app::App;
use crate::install::Step;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// Plain ASCII so the spinner renders on the ISO's VGA console (TERM=linux).
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
/// Cells of the big progress bar.
const BAR: usize = 60;

pub fn height(_app: &App) -> u16 {
    9
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let cancel = app.install_cancel;
    let show_log = app.install_show_log && !cancel;
    let buttons: Vec<Btn> = app.buttons().into_iter().map(|(b, _)| b).collect();
    let has_buttons = !buttons.is_empty();
    let show_shortcuts = !has_buttons;
    // 6 lines normally; the support opens with 12 on failure.
    let log_h: u16 = if show_log {
        if app.install.error.is_some() {
            12
        } else {
            6
        }
    } else {
        0
    };

    // header + blank + title + blank + bar + blank + status + blank + steps,
    // then optional log / buttons / shortcuts blocks, each preceded by a blank.
    let total: u16 = 9
        + if show_log { 1 + log_h } else { 0 }
        + if has_buttons { 2 } else { 0 }
        + if show_shortcuts { 2 } else { 0 };

    let mut y = area.y + area.height.saturating_sub(total) / 2;
    let line = |y: u16| Rect::new(area.x, y, area.width, 1);

    f.render_widget(
        Paragraph::new(app.header_line()).alignment(Alignment::Center),
        line(y),
    );
    y += 2;

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("Instalando nixdots en {}", app.settings.host_name),
            Style::default().fg(app.palette.white).add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        line(y),
    );
    y += 2;

    f.render_widget(
        Paragraph::new(bar_line(app)).alignment(Alignment::Center),
        line(y),
    );
    y += 2;

    if cancel {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "¿Cancelar la instalación? El disco quedará a medias.",
                Style::default().fg(app.palette.text),
            )))
            .alignment(Alignment::Center),
            line(y),
        );
    } else {
        f.render_widget(
            Paragraph::new(status_line(app)).alignment(Alignment::Center),
            line(y),
        );
    }
    y += 2;

    f.render_widget(
        Paragraph::new(steps_line(app)).alignment(Alignment::Center),
        line(y),
    );
    y += 1;

    if show_log {
        y += 1;
        let w = 84.min(area.width);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let rect = Rect::new(x, y, w, log_h);
        f.render_widget(ratatui::widgets::Clear, rect);
        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(app.palette.muted));
        let inner = block.inner(rect);
        f.render_widget(block, rect);
        let width = inner.width.max(1) as usize;
        // Wrap each logical line to the panel width, then keep only the last
        // `inner.height` physical rows so the most recent output stays visible.
        let mut rows: Vec<Line> = Vec::new();
        for l in app.install.visible_log(200) {
            for chunk in wrap_line(l, width) {
                rows.push(Line::from(Span::styled(
                    chunk,
                    Style::default().fg(app.palette.muted),
                )));
            }
        }
        let rows = rows.split_off(rows.len().saturating_sub(inner.height as usize));
        f.render_widget(Paragraph::new(rows), inner);
        y += log_h;
    }

    if has_buttons {
        y += 1;
        app.button_hitboxes = widgets::render_buttons(
            f,
            line(y),
            &app.palette,
            &buttons,
            app.button_focus,
            app.focus_buttons,
        );
        y += 1;
    } else {
        app.button_hitboxes.clear();
    }

    if show_shortcuts {
        y += 1;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "ctrl+c cancelar · v log: on/off · l pantalla completa",
                Style::default().fg(app.palette.muted),
            )))
            .alignment(Alignment::Center),
            line(y),
        );
    }
}

/// The big ASCII bar (`#` done, `-` pending) plus the percentage, from the real
/// build progress. Drawn from a console-safe font, so it shows even at 0 %.
fn bar_line(app: &App) -> Line<'static> {
    let total = app.install.total;
    let built = app.install.built.min(if total == 0 { 0 } else { total });
    let frac = if total == 0 {
        0.0
    } else {
        built as f64 / total as f64
    };
    let filled = (frac * BAR as f64).round() as usize;
    let bar_color = if app.install.error.is_some() {
        app.palette.error
    } else {
        app.palette.primary
    };
    Line::from(vec![
        Span::styled("#".repeat(filled), Style::default().fg(bar_color)),
        Span::styled(
            "-".repeat(BAR - filled),
            Style::default().fg(app.palette.muted),
        ),
        Span::styled(
            format!("  {}%", (frac * 100.0).round() as u64),
            Style::default().fg(app.palette.white),
        ),
    ])
}

/// Spinner + current step + `hechos de total` + `quedan ~n min`.
fn status_line(app: &App) -> Line<'static> {
    let spinner = SPINNER[(app.tick as usize) % SPINNER.len()];
    let step = app.install.current.map(|s| s.label()).unwrap_or("iniciando");
    let mut spans = vec![
        Span::styled(format!("{spinner} "), Style::default().fg(app.palette.primary)),
        Span::styled(step.to_string(), Style::default().fg(app.palette.muted)),
    ];
    if app.install.total > 0 {
        spans.push(Span::styled(
            format!(" · {} de {}", app.install.built, app.install.total),
            Style::default().fg(app.palette.muted),
        ));
    }
    if let Some(secs) = app.install.eta_secs() {
        spans.push(Span::styled(
            format!(" · quedan ~{} min", secs.div_ceil(60)),
            Style::default().fg(app.palette.muted),
        ));
    }
    Line::from(spans)
}

/// `[x] disco     [>] sistema     [ ] arranque` (ASCII, console-safe).
fn steps_line(app: &App) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    for (i, step) in Step::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("     "));
        }
        let (mark, mark_color, name_color) = if app.install.failed == Some(*step) {
            ("[!]", app.palette.error, app.palette.error)
        } else if app.install.done.contains(step) {
            ("[x]", app.palette.success, app.palette.text)
        } else if app.install.current == Some(*step) {
            ("[>]", app.palette.primary, app.palette.highlight)
        } else {
            ("[ ]", app.palette.muted, app.palette.muted)
        };
        spans.push(Span::styled(format!("{mark} "), Style::default().fg(mark_color)));
        spans.push(Span::styled(step.label(), Style::default().fg(name_color)));
    }
    Line::from(spans)
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.install_cancel {
        return match key.code {
            KeyCode::Esc => {
                app.install_cancel = false;
                app.focus_buttons = false;
                true
            }
            KeyCode::Char('v') => {
                app.install_show_log = !app.install_show_log;
                true
            }
            // Tab / arrows / Enter drive the two dialog buttons.
            _ => false,
        };
    }
    match key.code {
        KeyCode::Char('v') => {
            app.install_show_log = !app.install_show_log;
            true
        }
        // Full, scrollable log overlay.
        KeyCode::Char('l') => {
            app.install_log_view = true;
            app.install_log_scroll = 0;
            true
        }
        // Esc is a second way into the cancel confirmation.
        KeyCode::Esc => {
            if app.install.error.is_none() {
                app.install_cancel = true;
                app.focus_buttons = true;
                app.button_focus = 0;
            }
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, crate::app::Action)> {
    Vec::new()
}

/// Splits `s` into rows of at most `width` chars. An empty input yields a single
/// empty row so blank log lines still advance the view.
fn wrap_line(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    if s.is_empty() {
        return vec![String::new()];
    }
    s.chars()
        .collect::<Vec<char>>()
        .chunks(width)
        .map(|c| c.iter().collect())
        .collect()
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::wrap_line;

    #[test]
    fn wraps_at_width() {
        assert_eq!(wrap_line("abcdef", 3), vec!["abc", "def"]);
    }

    #[test]
    fn keeps_short_lines_and_empty() {
        assert_eq!(wrap_line("hi", 80), vec!["hi"]);
        assert_eq!(wrap_line("", 80), vec![""]);
    }
}
