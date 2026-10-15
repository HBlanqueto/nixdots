use crate::app::{Action, App, CheckState};
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

fn row(app: &App, label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<15}"), Style::default().fg(app.palette.muted)),
        Span::styled(value, Style::default().fg(app.palette.text)),
    ])
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let disk = app
        .selected_disk()
        .map(|d| {
            let action = match app.disks.effective() {
                crate::disk::DiskDecision::WholeDisk => "todo el disco".to_string(),
                crate::disk::DiskDecision::FreeSpace { .. } => "en el espacio libre".to_string(),
                crate::disk::DiskDecision::Shrink { .. } => {
                    format!("junto a Windows ({})", gb(app.disks.shrink_bytes))
                }
                _ => String::new(),
            };
            format!("{} · {}", d.short_name(), action)
        })
        .unwrap_or_default();
    let git = match &app.settings.git_name {
        Some(name) if app.git.enabled => name.clone(),
        _ => "no configurado".into(),
    };
    let cifrado = if !app.disks.encrypt {
        "No".to_string()
    } else if app.enc.tpm && app.enc.tpm_present {
        "Sí · TPM".to_string()
    } else if app.enc.use_account {
        "Sí · contraseña de la cuenta".to_string()
    } else {
        "Sí · frase propia".to_string()
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{:<15}", "Equipo"),
                Style::default().fg(app.palette.muted),
            ),
            Span::styled(app.pretty_host_name(), Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.muted)),
            Span::styled(
                app.settings.host_name.clone(),
                Style::default().fg(app.palette.muted),
            ),
        ]),
        row(app, "Lenguaje", crate::screens::locale::display(&app.settings.default_locale)),
        row(
            app,
            "Teclado",
            crate::keyboard::display(&app.settings.keyboard_layout, &app.settings.keyboard_variant),
        ),
        row(app, "Zona horaria", app.settings.time_zone.clone()),
        row(app, "Usuario", app.settings.username.clone().unwrap_or_default()),
        row(app, "Git", git),
        row(app, "Disco", disk),
        row(app, "Cifrado", cifrado),
        Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(app.palette.rule),
        )),
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

    if uutils_option(app) {
        lines.push(widgets::checkbox(
            &app.palette,
            app.keep_uutils,
            "uutils en vez de coreutils (experimental)",
            !app.focus_buttons && !app.no_focus(),
        ));
        if app.keep_uutils {
            lines.push(Line::from(Span::styled(
                "nixos-rebuild puede pedir confirmar con mv",
                Style::default().fg(app.palette.muted),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
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
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Instalar"), Action::Next),
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
            app.settings.username.as_deref().unwrap_or(""),
            app.settings.host_name
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
