use crate::app::App;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(app: &App) -> u16 {
    // 4 data rows + blank + checkbox (+ note when uutils is on).
    if app.keep_uutils {
        7
    } else {
        6
    }
}

fn row(app: &App, label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<16}"), Style::default().fg(app.palette.muted)),
        Span::styled(value, Style::default().fg(app.palette.text)),
    ])
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let disk = app
        .selected_disk()
        .map(|d| format!("{} ({})", d.path(), d.size_human()))
        .unwrap_or_default();
    let mut lines = vec![
        row(app, "host", app.settings.host_name.clone()),
        row(app, "arquitectura", app.detected_system()),
        row(app, "usuario", app.settings.username.clone()),
        row(app, "disco", disk),
        Line::from(""),
        app.checkbox(
            app.keep_uutils,
            "uutils en vez de coreutils (experimental)",
            app.button_focus == 1,
        ),
    ];
    if app.keep_uutils {
        lines.push(Line::from(Span::styled(
            "nixos-rebuild puede pedir confirmar con mv",
            Style::default().fg(app.palette.muted),
        )));
    }
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.keep_uutils = !app.keep_uutils;
        }
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.go_back();
            } else {
                app.advance();
            }
        }
        _ => {}
    }
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    let mut commands = vec![
        format!("disko --mode destroy,format,mount disko.nix --argstr device {device}"),
        format!(
            "nix build {}/nixdots#nixosConfigurations.{}.config.system.build.toplevel --store /mnt --log-format internal-json -v",
            "/mnt/home/".to_string() + &app.settings.username,
            app.settings.host_name
        ),
        "nixos-install --system <ruta> --no-root-passwd".into(),
        "sbctl create-keys --directory /mnt/persist/var/lib/sbctl".into(),
    ];
    if !app.keep_uutils {
        commands.push("quitar ./uutils.nix de etc/default.nix".into());
    }
    commands
}
