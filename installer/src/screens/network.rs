use crate::app::{Action, App};
use crate::net;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

const MAX_ROWS: usize = 6;

pub fn height(app: &App) -> u16 {
    if (app.net.ethernet && app.net.online) || app.net.wifi.is_empty() {
        2
    } else {
        (app.net.wifi.len().min(MAX_ROWS) as u16) + 2
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    if app.net.ethernet && app.net.online {
        lines.push(Line::from(vec![
            Span::styled("ethernet ", Style::default().fg(app.palette.muted)),
            Span::styled("✓ conectado", Style::default().fg(app.palette.success)),
        ]));
        lines.push(Line::from(Span::styled(
            "cache.nixos.org alcanzable",
            Style::default().fg(app.palette.muted),
        )));
        f.render_widget(Paragraph::new(lines), area);
        return;
    }

    if app.net.wifi.is_empty() {
        lines.push(Line::from(Span::styled(
            if app.net.backend == net::Backend::None {
                "no hay backend de red (nmcli/iwctl/wpa_cli)"
            } else {
                "no se encontraron redes · pulsa r"
            },
            Style::default().fg(app.palette.warning),
        )));
        if let Some(msg) = &app.net.message {
            lines.push(Line::from(Span::styled(msg.clone(), Style::default().fg(app.palette.error))));
        }
        f.render_widget(Paragraph::new(lines), area);
        return;
    }

    for (i, wifi) in app.net.wifi.iter().take(MAX_ROWS).enumerate() {
        let selected = i == app.net.selected && !app.net.editing;
        let security = if wifi.secured { "WPA2" } else { "abierta" };
        let text = format!("{:<18}", wifi.ssid);
        let trailing = format!("{}  {security:<7}", wifi.bars());
        lines.push(widgets::list_row(&app.palette, selected, text, Some(trailing)));
    }

    let password_focused = app.net.editing;
    lines.push(widgets::input_line(
        &app.palette,
        "contraseña",
        app.net.password.value(),
        app.net.password.visual_cursor(),
        password_focused,
        true,
        area.width,
        true,
    ));

    if let Some(msg) = &app.net.message {
        lines.push(Line::from(Span::styled(
            msg.clone(),
            Style::default().fg(app.palette.muted),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.net.editing {
        match key.code {
            KeyCode::Esc => {
                app.net.editing = false;
                return true;
            }
            KeyCode::Enter => {
                app.net.editing = false;
                return false;
            }
            _ if super::is_edit_key(&key) => {
                app.net.password.handle_event(&Event::Key(key));
                return true;
            }
            _ => return false,
        }
    }
    match key.code {
        KeyCode::Up => {
            if app.net.selected > 0 {
                app.net.selected -= 1;
            }
            true
        }
        KeyCode::Down => {
            if app.net.selected + 1 < app.net.wifi.len().min(MAX_ROWS) {
                app.net.selected += 1;
            } else {
                app.net.editing = true;
            }
            true
        }
        KeyCode::Char('r') => {
            rescan(app);
            true
        }
        _ => false,
    }
}

pub fn buttons(app: &App) -> Vec<(Btn, Action)> {
    let primary = if app.net.online {
        (Btn::primary("Continuar ›"), Action::Next)
    } else {
        (Btn::primary("Conectar ›"), Action::Connect)
    };
    vec![(Btn::secondary("‹ Atrás"), Action::Back), primary]
}

pub fn connect(app: &mut App) {
    if app.net.online {
        app.advance();
        return;
    }
    let Some(wifi) = app.net.wifi.get(app.net.selected).cloned() else {
        app.net.message = Some("selecciona una red".into());
        return;
    };
    let password = if wifi.secured {
        app.net.password.value().to_string()
    } else {
        String::new()
    };
    if app.dry_run {
        app.net.online = true;
        app.net.message = Some(format!("(simulación) conectado a {}", wifi.ssid));
        return;
    }
    match net::connect(app.net.backend, &wifi.ssid, &password) {
        Ok(()) => {
            app.refresh_network();
            app.net.message = Some(if app.net.online {
                "conectado".into()
            } else {
                "asociado, pero sin salida a cache.nixos.org".into()
            });
        }
        Err(err) => app.net.message = Some(format!("error: {err}")),
    }
}

pub fn rescan(app: &mut App) {
    app.refresh_network();
    if app.net.wifi.is_empty() {
        app.net.message = Some("sin resultados".into());
    }
}

pub fn commands(app: &App) -> Vec<String> {
    let ssid = app
        .net
        .wifi
        .get(app.net.selected)
        .map(|w| w.ssid.clone())
        .unwrap_or_else(|| "<ssid>".into());
    vec![
        "nmcli device wifi list".into(),
        format!("nmcli device wifi connect {ssid} password ••••"),
        "conectar a cache.nixos.org:443".into(),
    ]
}
