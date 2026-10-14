use crate::app::App;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn height(app: &App) -> u16 {
    let mut h = 4;
    if !app.settings_ok {
        h += 1;
    }
    h
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let detection = |ok: bool, text: &str, value: String| {
        Line::from(vec![
            Span::styled(
                if ok { "✓ " } else { "✗ " },
                Style::default().fg(if ok {
                    app.palette.success
                } else {
                    app.palette.error
                }),
            ),
            Span::styled(format!("{text:<14}"), Style::default().fg(app.palette.muted)),
            Span::styled(value, Style::default().fg(app.palette.text)),
        ])
    };

    let mut lines = vec![
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
        detection(app.disks_found, "discos", format!("{}", app.disks.len())),
        detection(
            app.network,
            "red",
            if app.network {
                "cache.nixos.org".into()
            } else {
                "sin conexión".into()
            },
        ),
    ];

    if !app.settings_ok {
        lines.push(Line::from(Span::styled(
            "settings.nix ilegible: usando valores por defecto",
            Style::default().fg(app.palette.warning),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            app.button_focus = if app.button_focus == 1 { 0 } else { 1 };
        }
        KeyCode::Enter => {
            if app.button_focus == 0 {
                app.quit = Some(crate::app::Quit::Normal);
            } else {
                app.advance();
            }
        }
        _ => {}
    }
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        format!("git clone --depth 1 {} {}", app.repo_spec, app.repo_dir.display()),
        "uname -m".into(),
        "systemd-detect-virt".into(),
        "lsblk -J -b -d -o NAME,SIZE,MODEL,TYPE".into(),
        "conectar a cache.nixos.org:443".into(),
    ]
}
