use crate::app::{Action, App, Click};
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

pub fn height(app: &App) -> u16 {
    // 6 rows + blank + warning (+ checkbox on ThinkPad) (+ mv note).
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
        .map(|d| format!("{} ({})", d.path(), d.size_human()))
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
        Line::from(""),
    ];

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

    // Warn, don't fix: /var/log is both the @log subvolume and an impermanence
    // directory.
    lines.push(Line::from(Span::styled(
        "/var/log definido dos veces: subvolumen @log e impermanence",
        Style::default().fg(app.palette.warning),
    )));

    f.render_widget(Paragraph::new(lines), area);
    // The uutils checkbox is on line 8 (6 rows + a blank + the warning below).
    if uutils_option(app) {
        app.targets.borrow_mut().push((
            Rect::new(area.x, area.y + 7, area.width, 1),
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
            "nix build /mnt/home/{}/nixdots#nixosConfigurations.{}... --store /mnt",
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
