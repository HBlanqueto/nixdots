use crate::app::{Action, App, CheckState, Click};
use crate::detect::Profile;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

fn uutils_option(app: &App) -> bool {
    app.uutils_available()
}

fn gb(bytes: u64) -> String {
    format!("{:.0} GB", bytes as f64 / 1_000_000_000.0)
}

pub fn height(app: &App) -> u16 {
    // 6 rows + check + blank (+ checkbox on ThinkPad) (+ mv note).
    let mut h = 8;
    if uutils_option(app) {
        h += 1;
    }
    if app.keep_uutils && uutils_option(app) {
        h += 1;
    }
    h
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
        .map(|d| {
            let action = match app.disks.effective() {
                crate::disk::DiskDecision::WholeDisk => "todo el disco".to_string(),
                crate::disk::DiskDecision::FreeSpace { .. } => {
                    format!("en el espacio libre ({})", gb(d.free))
                }
                crate::disk::DiskDecision::Shrink { .. } => {
                    format!("junto a Windows ({})", gb(app.disks.shrink_bytes))
                }
                _ => String::new(),
            };
            format!("{} · {}", d.short_name(), action)
        })
        .unwrap_or_default();
    let region = format!("{} · {}", app.settings.time_zone, app.settings.default_locale);
    let git = match &app.settings.git_name {
        Some(name) if app.git.enabled => name.clone(),
        _ => "no configurado".into(),
    };
    let cifrado = if !app.disks.encrypt {
        "sin cifrar".to_string()
    } else if app.enc.tpm && app.enc.tpm_present {
        "LUKS2 · TPM".to_string()
    } else {
        "LUKS2".to_string()
    };

    let mut lines = vec![
        row(app, "hostname", app.settings.host_name.clone()),
        row(app, "región", region),
        row(app, "usuario", app.settings.username.clone()),
        row(app, "git", git),
        row(app, "disco", disk),
        row(app, "cifrado", cifrado),
    ];

    // The background repo evaluation, launched when the license was accepted.
    let (mark, text, color) = match &app.source_check {
        CheckState::Ok => ("✓ ", "configuración evalúa sin errores".to_string(), app.palette.success),
        CheckState::Failed(err) => ("✗ ", err.clone(), app.palette.error),
        CheckState::Running => ("… ", "evaluando configuración".to_string(), app.palette.muted),
        CheckState::Idle => ("", String::new(), app.palette.muted),
    };
    lines.push(Line::from(vec![
        Span::styled(mark.to_string(), Style::default().fg(color)),
        Span::styled(text, Style::default().fg(color)),
    ]));
    lines.push(Line::from(""));

    if uutils_option(app) {
        lines.push(widgets::checkbox(
            &app.palette,
            app.keep_uutils,
            "uutils en vez de coreutils (experimental)",
            !app.focus_buttons,
        ));
        if app.keep_uutils {
            lines.push(Line::from(Span::styled(
                "nixos-rebuild puede pedir confirmar con mv",
                Style::default().fg(app.palette.muted),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
    // The uutils checkbox follows the 6 rows, the check line and a blank.
    if uutils_option(app) {
        app.targets.borrow_mut().push((
            Rect::new(area.x, area.y + 8, area.width, 1),
            Click::SummaryUutils,
        ));
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Char('x') if uutils_option(app) => {
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
            "nix build --impure /mnt/home/{}/nixdots#nixosConfigurations.{}... --store /mnt",
            app.settings.username, app.settings.host_name
        ),
        "nixos-install --root /mnt --system <ruta> --no-root-passwd".into(),
    ];
    if app.profile == Profile::Thinkpad {
        commands.push("sbctl create-keys --directory /mnt/persist/var/lib/sbctl".into());
        if !app.keep_uutils {
            commands.push("quitar ./uutils.nix de modules/common/default.nix".into());
        }
    }
    commands
}
