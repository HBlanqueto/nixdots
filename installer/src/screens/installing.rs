use crate::app::App;
use crate::install::Step;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Plain ASCII so the spinner renders on the ISO's VGA console (TERM=linux).
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
/// Cells of the big progress bar.
const BAR: usize = 60;

pub fn height(_app: &App) -> u16 {
    5
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let cancel = app.install_cancel;
    let buttons: Vec<Btn> = app.buttons().into_iter().map(|(b, _)| b).collect();
    let has_buttons = !buttons.is_empty();
    let show_shortcuts = !has_buttons;

    // title + blank + bar + blank + status
    let total: u16 = 5
        + if has_buttons { 2 } else { 0 }
        + if show_shortcuts { 2 } else { 0 };
    let mut y = area.y + area.height.saturating_sub(total) / 2;
    let line = |y: u16| Rect::new(area.x, y, area.width, 1);

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
    y += 1;

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
                "v ver registro · ctrl+c cancelar",
                Style::default().fg(app.palette.muted),
            )))
            .alignment(Alignment::Center),
            line(y),
        );
    }
}

fn step_weight(step: Step) -> u64 {
    match step {
        Step::Disk => 5,
        Step::Settings => 5,
        Step::System => 80,
        Step::Boot => 10,
    }
}

/// Overall percentage: weights of the finished steps, plus the system step's
/// real nix progress (80 %). 100 % only once everything is done.
pub fn overall_percent(app: &App) -> u64 {
    let iv = &app.install;
    let mut p = 0u64;
    for s in Step::ALL {
        if iv.done.contains(&s) {
            p += step_weight(s);
        } else if iv.current == Some(s) && s == Step::System && iv.total > 0 {
            p += step_weight(s) * iv.built.min(iv.total) / iv.total;
        }
    }
    p.min(100)
}

/// The big ASCII bar (`#` done, `-` pending) plus the weighted percentage.
fn bar_line(app: &App) -> Line<'static> {
    let percent = overall_percent(app);
    let filled = (percent as usize * BAR / 100).min(BAR);
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
            format!("  {percent}%"),
            Style::default().fg(app.palette.white),
        ),
    ])
}

/// Spinner + Spanish gerund + ETA, or the error (`✗ etapa: primera línea`).
fn status_line(app: &App) -> Line<'static> {
    if let Some(err) = &app.install.error {
        let stage = app.install.failed.map(|s| s.label()).unwrap_or("error");
        let first = err.lines().next().unwrap_or("").to_string();
        return Line::from(vec![
            Span::styled("✗ ", Style::default().fg(app.palette.error)),
            Span::styled(
                format!("{stage}: {first}"),
                Style::default().fg(app.palette.error),
            ),
        ]);
    }
    let spinner = SPINNER[(app.tick as usize) % SPINNER.len()];
    let gerund = match app.install.current {
        Some(Step::Disk) => "Preparando el disco",
        Some(Step::Settings) => "Ajustando el sistema",
        Some(Step::System) => "Construyendo el sistema",
        Some(Step::Boot) => "Instalando el arranque",
        None => "Iniciando",
    };
    let mut spans = vec![
        Span::styled(format!("{spinner} "), Style::default().fg(app.palette.primary)),
        Span::styled(gerund.to_string(), Style::default().fg(app.palette.text)),
    ];
    if let Some(secs) = app.install.eta_secs() {
        spans.push(Span::styled(
            format!(" · quedan ~{} min", secs.div_ceil(60)),
            Style::default().fg(app.palette.muted),
        ));
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
                app.install_log_view = !app.install_log_view;
                true
            }
            _ => false,
        };
    }
    match key.code {
        KeyCode::Char('v') => {
            app.install_log_view = !app.install_log_view;
            if app.install_log_view {
                app.install_log_scroll = 0;
            }
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

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
