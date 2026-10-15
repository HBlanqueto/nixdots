use crate::app::{Action, App};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(_app: &App) -> u16 {
    3
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let detection = |ok: bool, text: &str, value: String| {
        Line::from(vec![
            Span::styled(
                if ok { "✓ " } else { "✗ " },
                Style::default().fg(if ok { app.palette.success } else { app.palette.error }),
            ),
            Span::styled(format!("{text:<14}"), Style::default().fg(app.palette.muted)),
            Span::styled(value, Style::default().fg(app.palette.text)),
        ])
    };
    let disk = app
        .selected_disk()
        .map(|d| format!("{} ({})", d.path(), d.size_human()))
        .unwrap_or_else(|| "ninguno".into());
    let lines = vec![
        detection(
            app.arch.is_some(),
            "arquitectura",
            app.arch.clone().unwrap_or_else(|| "desconocida".into()),
        ),
        detection(
            app.virt.is_some(),
            "entorno",
            app.virt.clone().unwrap_or_else(|| "bare metal".into()),
        ),
        detection(app.selected_disk().is_some(), "disco", disk),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(_app: &mut App, _key: KeyEvent) -> bool {
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Continuar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "uname -m".into(),
        "systemd-detect-virt".into(),
        "lsblk -J -b -d -o NAME,SIZE,MODEL,TYPE".into(),
        format!("git clone --depth 1 {} {}", app.repo_spec, app.repo_dir.display()),
    ]
}
