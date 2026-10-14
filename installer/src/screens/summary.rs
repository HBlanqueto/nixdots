use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(app: &App) -> u16 {
    if app.keep_uutils {
        8
    } else {
        7
    }
}

fn row(app: &App, label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<14}"), Style::default().fg(app.palette.muted)),
        Span::styled(value, Style::default().fg(app.palette.text)),
    ])
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let disk = app
        .selected_disk()
        .map(|d| format!("{} ({})", d.path(), d.size_human()))
        .unwrap_or_default();
    let region = format!("{} · {}", app.settings.time_zone, app.settings.default_locale);
    let git = match &app.settings.git_name {
        Some(name) if app.git.enabled => name.clone(),
        _ => "no configurado".into(),
    };

    let mut lines = vec![
        row(app, "hostname", app.settings.host_name.clone()),
        row(app, "región", region),
        row(app, "usuario", app.settings.username.clone()),
        row(app, "git", git),
        row(app, "disco", disk),
        Line::from(""),
        widgets::checkbox(
            &app.palette,
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

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') => {
            app.keep_uutils = !app.keep_uutils;
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Instalar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    let mut commands = vec![
        format!("disko --mode destroy,format,mount disko.nix --argstr device {device}"),
        format!(
            "nix build /mnt/home/{}/nixdots#nixosConfigurations.{}... --store /mnt",
            app.settings.username, app.settings.host_name
        ),
        "nixos-install --system <ruta> --no-root-passwd".into(),
        "sbctl create-keys --directory /mnt/persist/var/lib/sbctl".into(),
    ];
    if !app.keep_uutils {
        commands.push("quitar ./uutils.nix de etc/default.nix".into());
    }
    commands
}
