use crate::app::{Action, App, RedFase};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

const MAX_ROWS: usize = 6;
const SPINNER: [&str; 8] = ["⢿", "⣻", "⣽", "⣾", "⣷", "⣯", "⣟", "⡿"];

pub fn height(app: &App) -> u16 {
    match &app.net.phase {
        RedFase::Lista => app.net.wifi.len().clamp(1, MAX_ROWS) as u16,
        RedFase::Contrasena { .. } => 2,
        RedFase::Conectando { .. } => 1,
        RedFase::Error { .. } => 2,
        RedFase::Conectado { .. } => 2,
    }
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    match &app.net.phase {
        RedFase::Lista => draw_list(f, app, area),
        RedFase::Contrasena { ssid, input, visible } => {
            let security = app
                .net
                .wifi
                .iter()
                .find(|w| &w.ssid == ssid)
                .map(|w| w.security())
                .unwrap_or("WPA2");
            let header = Line::from(vec![
                Span::styled("red  ", Style::default().fg(app.palette.muted)),
                Span::styled(ssid.clone(), Style::default().fg(app.palette.text)),
                Span::raw("  "),
                Span::styled(security.to_string(), Style::default().fg(app.palette.secondary)),
            ]);
            let field = widgets::input_line(
                &app.palette,
                "contraseña",
                input.value(),
                input.visual_cursor(),
                !app.focus_buttons,
                !*visible,
                area.width,
                true,
                "",
            );
            f.render_widget(Paragraph::new(vec![header, field]), area);
        }
        RedFase::Conectando { ssid } => {
            let spinner = SPINNER[(app.tick as usize / 2) % SPINNER.len()];
            let line = Line::from(vec![
                Span::styled(format!("{spinner} "), Style::default().fg(app.palette.focus)),
                Span::styled(
                    format!("conectando a {ssid}…"),
                    Style::default().fg(app.palette.text),
                ),
            ]);
            f.render_widget(Paragraph::new(line), area);
        }
        RedFase::Error { msg, .. } => {
            let lines = vec![
                Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(app.palette.error)),
                    Span::styled(msg.clone(), Style::default().fg(app.palette.error)),
                ]),
                Line::from(Span::styled(
                    "enter reintentar · esc volver a redes",
                    Style::default().fg(app.palette.muted),
                )),
            ];
            f.render_widget(Paragraph::new(lines), area);
        }
        RedFase::Conectado { ssid } => {
            let head = if ssid == "ethernet" {
                "ethernet".to_string()
            } else {
                format!("conectado a {ssid}")
            };
            let lines = vec![
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(app.palette.success)),
                    Span::styled(head, Style::default().fg(app.palette.text)),
                ]),
                Line::from(vec![
                    Span::styled("✓ ", Style::default().fg(app.palette.success)),
                    Span::styled(
                        "cache.nixos.org alcanzable",
                        Style::default().fg(app.palette.text),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), area);
        }
    }
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    if app.net.wifi.is_empty() {
        let msg = if app.net.backend.name() == "ninguno" {
            "no hay backend de red (nmcli/iwctl/wpa_cli)"
        } else {
            "no se encontraron redes · pulsa r"
        };
        f.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(app.palette.warning))),
            area,
        );
        return;
    }
    let lines: Vec<Line> = app
        .net
        .wifi
        .iter()
        .take(MAX_ROWS)
        .enumerate()
        .map(|(i, wifi)| {
            let text = format!("{:<18}", wifi.ssid);
            let trailing = format!("{}  {:<7}", wifi.bars(), wifi.security());
            widgets::list_row(&app.palette, i == app.net.selected, text, Some(trailing))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), area);
}

/// Returns `true` when the key was handled by the screen (and must not fall
/// through to the global shortcuts). Input-focused sub-states return `true`
/// for every printable key, including `q`, `v` and `r`.
pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match &mut app.net.phase {
        RedFase::Lista => match key.code {
            KeyCode::Up => {
                if app.net.selected > 0 {
                    app.net.selected -= 1;
                }
                true
            }
            KeyCode::Down => {
                if app.net.selected + 1 < app.net.wifi.len().min(MAX_ROWS) {
                    app.net.selected += 1;
                }
                true
            }
            KeyCode::Char('r') => {
                rescan(app);
                true
            }
            KeyCode::Enter => {
                choose(app);
                true
            }
            _ => false,
        },
        RedFase::Contrasena { visible, .. } => match key.code {
            KeyCode::Enter => {
                connect(app);
                true
            }
            KeyCode::Esc => {
                to_menu(app);
                true
            }
            KeyCode::Tab => {
                *visible = !*visible;
                true
            }
            _ if super::is_edit_key(&key) => {
                if let RedFase::Contrasena { input, .. } = &mut app.net.phase {
                    input.handle_event(&Event::Key(key));
                }
                true
            }
            _ => false,
        },
        RedFase::Error { .. } => match key.code {
            KeyCode::Enter => {
                retry(app);
                true
            }
            KeyCode::Esc => {
                to_menu(app);
                true
            }
            _ => false,
        },
        // Conectando has no actions; Conectado lets the globals handle
        // Enter (Continuar) and Esc (back).
        RedFase::Conectando { .. } | RedFase::Conectado { .. } => false,
    }
}

/// Enter on a list entry: password prompt for a new secured network, direct
/// connection for open or already-saved ones.
fn choose(app: &mut App) {
    let Some(wifi) = app.net.wifi.get(app.net.selected).cloned() else {
        return;
    };
    if wifi.secured && !app.net.backend.is_saved(&wifi.ssid) {
        app.net.phase = RedFase::Contrasena {
            ssid: wifi.ssid,
            input: Input::default(),
            visible: false,
        };
        app.focus_primary();
    } else {
        app.start_net_connect(wifi.ssid.clone(), String::new());
    }
}

/// Starts the connection from the Contrasena phase.
pub fn connect(app: &mut App) {
    if let RedFase::Contrasena { ssid, input, .. } = &app.net.phase {
        let ssid = ssid.clone();
        let pass = input.value().to_string();
        if pass.is_empty() {
            return;
        }
        app.start_net_connect(ssid, pass);
    }
}

pub fn rescan(app: &mut App) {
    app.refresh_network();
}

pub fn to_menu(app: &mut App) {
    app.net.phase = RedFase::Lista;
    app.focus_primary();
    if app.net.selected >= app.net.wifi.len() {
        app.net.selected = 0;
    }
}

pub fn retry(app: &mut App) {
    if let RedFase::Error { ssid, .. } = &app.net.phase {
        app.net.phase = RedFase::Contrasena {
            ssid: ssid.clone(),
            input: Input::default(),
            visible: false,
        };
        app.focus_primary();
    }
}

pub fn shortcut(app: &App) -> String {
    match &app.net.phase {
        RedFase::Lista => "↑/↓ red · enter elegir · r reescanear".to_string(),
        RedFase::Contrasena { .. } => {
            "enter conectar · esc volver a redes · tab mostrar".to_string()
        }
        RedFase::Conectando { .. } => "conectando…".to_string(),
        RedFase::Error { .. } => "enter reintentar · esc volver a redes".to_string(),
        RedFase::Conectado { .. } => "enter continuar".to_string(),
    }
}

pub fn buttons(app: &App) -> Vec<(Btn, Action)> {
    match &app.net.phase {
        RedFase::Lista => vec![(Btn::secondary("‹ Atrás"), Action::Back)],
        RedFase::Contrasena { .. } => vec![
            (Btn::secondary("‹ Redes"), Action::NetMenu),
            (Btn::primary("Conectar ›"), Action::Connect),
        ],
        RedFase::Conectando { .. } => Vec::new(),
        RedFase::Error { .. } => vec![
            (Btn::secondary("‹ Redes"), Action::NetMenu),
            (Btn::primary("Reintentar ›"), Action::NetRetry),
        ],
        RedFase::Conectado { .. } => vec![
            (Btn::secondary("‹ Cambiar red"), Action::NetMenu),
            (Btn::primary("Continuar ›"), Action::Next),
        ],
    }
}

pub fn commands(app: &App) -> Vec<String> {
    let backend = app.net.backend.name();
    match &app.net.phase {
        RedFase::Conectado { ssid } if ssid == "ethernet" => {
            vec!["conectar a cache.nixos.org:443".into()]
        }
        RedFase::Lista => vec![format!("{backend} device wifi list")],
        RedFase::Contrasena { ssid, .. } | RedFase::Conectando { ssid } => vec![format!(
            "{backend} device wifi connect {ssid} password ••••••"
        )],
        RedFase::Error { ssid, .. } => vec![format!("{backend} device wifi connect {ssid}")],
        RedFase::Conectado { ssid } => vec![
            format!("{backend}: {ssid}"),
            "conectar a cache.nixos.org:443".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Screen;
    use crate::net::Wifi;
    use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    fn app() -> App {
        let mut app = App::new(
            true,
            "test".into(),
            PathBuf::from("/tmp/nixdots-does-not-exist"),
        );
        app.screen = Screen::Network;
        app.net.phase = RedFase::Lista;
        app.net.wifi = vec![
            Wifi { ssid: "WPA-Net".into(), signal: 80, secured: true },
            Wifi { ssid: "Abierta".into(), signal: 40, secured: false },
        ];
        app.net.selected = 0;
        app
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
    }

    fn wait_for_reply(app: &mut App) {
        let start = Instant::now();
        while matches!(app.net.phase, RedFase::Conectando { .. })
            && start.elapsed() < Duration::from_secs(5)
        {
            app.poll_network();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn enter_on_wpa_opens_password() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        assert!(
            matches!(app.net.phase, RedFase::Contrasena { .. }),
            "Enter en una red WPA debe abrir Contrasena"
        );
    }

    #[test]
    fn typing_q_in_password_does_not_quit() {
        let mut app = app();
        press(&mut app, KeyCode::Enter); // -> Contrasena
        press(&mut app, KeyCode::Char('q'));
        press(&mut app, KeyCode::Char('v'));
        assert!(!app.should_quit, "q no debe cerrar mientras se escribe");
        match &app.net.phase {
            RedFase::Contrasena { input, .. } => assert_eq!(input.value(), "qv"),
            _ => panic!("debería seguir en Contrasena"),
        }
    }

    #[test]
    fn esc_returns_to_list() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.net.phase, RedFase::Lista));
    }

    #[test]
    fn tab_toggles_password_visibility() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Tab);
        match &app.net.phase {
            RedFase::Contrasena { visible, .. } => assert!(*visible),
            _ => panic!("debería seguir en Contrasena"),
        }
    }

    #[test]
    fn short_password_in_simulation_errors() {
        let mut app = app();
        press(&mut app, KeyCode::Enter); // -> Contrasena
        for ch in "corta".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::Enter); // -> Conectando
        assert!(matches!(app.net.phase, RedFase::Conectando { .. }));
        wait_for_reply(&mut app);
        assert!(
            matches!(app.net.phase, RedFase::Error { .. }),
            "una contraseña corta debe llevar a Error"
        );
    }

    #[test]
    fn long_password_in_simulation_connects() {
        let mut app = app();
        press(&mut app, KeyCode::Enter);
        for ch in "contrasena-larga".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::Enter);
        wait_for_reply(&mut app);
        assert!(matches!(app.net.phase, RedFase::Conectado { .. }));
    }

    #[test]
    fn only_connected_allows_advance() {
        let mut app = app();
        assert!(!app.primary_enabled(), "Lista no debe permitir avanzar");
        app.net.phase = RedFase::Conectado { ssid: "WPA-Net".into() };
        assert!(app.primary_enabled(), "Conectado debe permitir avanzar");
    }
}
