use crate::app::{Action, App};
use crate::widgets::Btn;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::process::Command;

pub fn height(_app: &App) -> u16 {
    4
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let done = app.done.as_ref();
    let ok = done.map(|d| d.ok).unwrap_or(true);
    let host = done.map(|d| d.host.clone()).unwrap_or_default();
    let duration = done.map(|d| d.duration).unwrap_or_default();

    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} nixdots instalado", if ok { "✔" } else { "✘" }),
            Style::default().fg(if ok { app.palette.success } else { app.palette.error }),
        )),
        Line::from(Span::styled(
            format!("{host} · {:.1}s", duration.as_secs_f32()),
            Style::default().fg(app.palette.text),
        )),
        Line::from(Span::styled(
            "actualiza con flake-update-sw",
            Style::default().fg(app.palette.muted),
        )),
    ];
    if let Some(log) = &app.install.log_path {
        lines.push(Line::from(Span::styled(
            format!("log {}", log.display()),
            Style::default().fg(app.palette.muted),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::Char('r') {
        reboot(app);
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("Salir"), Action::Quit),
        (Btn::primary("Reiniciar ›"), Action::Reboot),
    ]
}

pub fn reboot(app: &mut App) {
    if app.dry_run {
        app.status = Some("(simulación) umount -R /mnt && reboot".into());
        app.should_quit = true;
        return;
    }
    let _ = Command::new("sh")
        .arg("-c")
        .arg("umount -R /mnt && reboot")
        .status();
    app.should_quit = true;
}

pub fn commands(_app: &App) -> Vec<String> {
    vec!["umount -R /mnt && reboot".into()]
}
