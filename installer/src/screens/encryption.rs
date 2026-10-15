//! The Cifrado screen: choose the LUKS2 passphrase, the recovery key and the
//! TPM unlock for the `System HD` partition.
//!
//! The recovery key is generated in memory ([`crate::crypto`]), shown once and
//! enrolled during the install; it is never written to the disk, the log or the
//! command panel.

use crate::app::{
    Action, App, ENC_ACCOUNT, ENC_CONFIRM, ENC_NOTED, ENC_PASS, ENC_TPM,
};
use crate::runner::{self, Cmd};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(app: &App) -> u16 {
    let mut h = 1; // account checkbox
    if !app.enc.use_account {
        h += 6; // frase(2), confirmar(2), seguridad(1), aviso(1)
        h += 1; // muted note
        if pass_has_non_ascii(app) {
            h += 1;
        }
    }
    if app.enc.tpm_present {
        h += 2; // TPM checkbox + note
    }
    h += 1; // blank before the recovery key
    h += 1; // "clave de recuperación" header
    h += 3; // key box (top border, text, bottom border)
    h += 1; // "guardar en USB"
    h += 1; // "La anoté" checkbox
    h
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mut y = area.y;

    // 1. Reuse the account password as the LUKS passphrase.
    f.render_widget(
        Paragraph::new(widgets::checkbox(
            &app.palette,
            app.enc.use_account,
            "Usar la misma contraseña de la cuenta",
            app.enc.focus == ENC_ACCOUNT && !app.focus_buttons,
        )),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    // 2. A custom passphrase, indented four columns.
    if !app.enc.use_account {
        let indent = area.x + 4;
        let indent_w = area.width.saturating_sub(4);

        let first = widgets::field(
            &app.palette,
            "frase",
            app.enc.pass.value(),
            app.enc.pass.visual_cursor(),
            app.enc.focus == ENC_PASS && !app.focus_buttons,
            true,
            indent_w,
            true,
            "",
        );
        f.render_widget(Paragraph::new(first), Rect::new(indent, y, indent_w, 2));
        y += 2;

        let mut second = widgets::field(
            &app.palette,
            "confirmar",
            app.enc.confirm.value(),
            app.enc.confirm.visual_cursor(),
            app.enc.focus == ENC_CONFIRM && !app.focus_buttons,
            true,
            indent_w,
            true,
            "",
        );
        if app.enc.passphrase_matches() {
            second[1].spans.push(Span::styled(
                "  ✓ coinciden",
                Style::default().fg(app.palette.success),
            ));
        }
        f.render_widget(Paragraph::new(second), Rect::new(indent, y, indent_w, 2));
        y += 2;

        // Strength bar (12 cells).
        let (filled, label, color) = strength(app.enc.pass.value());
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("seguridad  ", Style::default().fg(app.palette.muted)),
                Span::styled("━".repeat(filled), Style::default().fg(color)),
                Span::styled(
                    "─".repeat(12 - filled),
                    Style::default().fg(app.palette.row_bg),
                ),
                Span::styled(
                    format!("  {label}"),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
            ])),
            Rect::new(indent, y, indent_w, 1),
        );
        y += 1;

        // It is typed at boot, with the selected keyboard layout.
        let id = app.keyboard_active.as_str();
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("! se escribe al encender, con teclado: {id}"),
                Style::default().fg(app.palette.warning),
            )),
            Rect::new(indent, y, indent_w, 1),
        );
        y += 1;

        f.render_widget(
            Paragraph::new(Span::styled(
                "la cuenta sigue usando su propia contraseña",
                Style::default().fg(app.palette.muted),
            )),
            Rect::new(indent, y, indent_w, 1),
        );
        y += 1;

        if pass_has_non_ascii(app) {
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        "! contiene caracteres especiales: asegúrate de que el teclado al arrancar sea {id}"
                    ),
                    Style::default().fg(app.palette.warning),
                )),
                Rect::new(indent, y, indent_w, 1),
            );
            y += 1;
        }
    }

    // 3. TPM auto-unlock (only when a TPM 2.0 is present).
    if app.enc.tpm_present {
        f.render_widget(
            Paragraph::new(widgets::checkbox(
                &app.palette,
                app.enc.tpm,
                "Desbloquear automáticamente con TPM",
                app.enc.focus == ENC_TPM && !app.focus_buttons,
            )),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
        f.render_widget(
            Paragraph::new(Span::styled(
                "TPM 2.0 detectado",
                Style::default().fg(app.palette.muted),
            )),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }

    // 4. Recovery key.
    y += 1;
    f.render_widget(
        Paragraph::new(Span::styled(
            "clave de recuperación · anótala, no se volverá a mostrar",
            Style::default().fg(app.palette.muted),
        )),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    draw_key_box(f, app, area, y);
    y += 3;

    let usb = if let Some(dev) = &app.enc.usb_saved {
        format!("u  guardar en USB  ·  guardada en {dev}")
    } else {
        "u  guardar en USB".to_string()
    };
    f.render_widget(
        Paragraph::new(Span::styled(usb, Style::default().fg(app.palette.highlight))),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    f.render_widget(
        Paragraph::new(widgets::checkbox(
            &app.palette,
            app.enc.noted,
            "La anoté en un lugar seguro",
            app.enc.focus == ENC_NOTED && !app.focus_buttons,
        )),
        Rect::new(area.x, y, area.width, 1),
    );
}

fn draw_key_box(f: &mut Frame, app: &App, area: Rect, y: u16) {
    let key = app.enc.recovery.as_str();
    let inner_w = key.chars().count() as u16 + 2;
    let w = (inner_w + 2).min(area.width);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let rect = Rect::new(x, y, w, 3);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.warning));
    f.render_widget(block, rect);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            key.to_string(),
            Style::default()
                .fg(app.palette.white)
                .add_modifier(Modifier::BOLD),
        )))
        .alignment(Alignment::Center),
        Rect::new(rect.x + 1, rect.y + 1, rect.width.saturating_sub(2), 1),
    );
}

fn pass_has_non_ascii(app: &App) -> bool {
    !app.enc.pass.value().is_ascii()
}

/// (filled cells out of 12, label, colour) for the strength bar.
fn strength(pass: &str) -> (usize, &'static str, Color) {
    let n = pass.chars().count();
    let mut classes = 0;
    if pass.chars().any(|c| c.is_ascii_lowercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_uppercase()) {
        classes += 1;
    }
    if pass.chars().any(|c| c.is_ascii_digit()) {
        classes += 1;
    }
    if pass.chars().any(|c| !c.is_ascii_alphanumeric()) {
        classes += 1;
    }
    let score = ((n.min(20) as f32) / 20.0) * 0.6 + (classes as f32 / 4.0) * 0.4;
    let filled = ((score * 12.0).round() as usize).clamp(1, 12);
    if n < 8 || score < 0.45 {
        (filled, "débil", Color::Red)
    } else if score < 0.72 {
        (filled, "aceptable", Color::Yellow)
    } else {
        (filled, "buena", Color::Green)
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Up => {
            app.enc.move_focus(-1);
            true
        }
        KeyCode::Down => {
            app.enc.move_focus(1);
            true
        }
        KeyCode::Char(' ') => match app.enc.focus {
            ENC_ACCOUNT => {
                let reuse = !app.enc.use_account;
                app.enc.set_use_account(reuse);
                true
            }
            ENC_TPM => {
                app.enc.tpm = !app.enc.tpm;
                true
            }
            ENC_NOTED => {
                app.enc.noted = !app.enc.noted;
                true
            }
            _ => false,
        },
        KeyCode::Char('u') | KeyCode::Char('U')
            if app.enc.focus != ENC_PASS && app.enc.focus != ENC_CONFIRM =>
        {
            app.open_usb_modal();
            true
        }
        KeyCode::Enter if app.enc.focus == ENC_PASS => {
            app.enc.focus = ENC_CONFIRM;
            true
        }
        _ if (app.enc.focus == ENC_PASS || app.enc.focus == ENC_CONFIRM)
            && super::is_edit_key(&key) =>
        {
            let target = if app.enc.focus == ENC_PASS {
                &mut app.enc.pass
            } else {
                &mut app.enc.confirm
            };
            target.handle_event(&Event::Key(key));
            true
        }
        _ => false,
    }
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn shortcuts(_app: &App) -> Vec<String> {
    vec!["espacio casilla".into(), "↑↓ mover".into()]
}

pub fn commands(app: &App) -> Vec<String> {
    let device = app
        .selected_disk()
        .map(|d| d.path())
        .unwrap_or_else(|| "/dev/xxx".into());
    vec![
        "disko --argstr encrypt true  (luksFormat con la frase por stdin)".into(),
        format!(
            "cryptsetup luksAddKey --key-file /run/... <partición crypto_LUKS de {device}> /run/..."
        ),
        format!("systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7 <partición LUKS de {device}>"),
    ]
}

pub fn usb_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => app.enc.usb_open = false,
        KeyCode::Up => app.enc.usb_index = app.enc.usb_index.saturating_sub(1),
        KeyCode::Down => {
            if app.enc.usb_index + 1 < app.enc.usb.len() {
                app.enc.usb_index += 1;
            }
        }
        KeyCode::Enter => {
            save_recovery(app);
            app.enc.usb_open = false;
        }
        _ => {}
    }
    true
}

pub fn draw_usb(f: &mut Frame, app: &App, area: Rect) {
    widgets::dim_buffer(f, &app.palette);
    let w = 58.min(area.width);
    let rows = app.enc.usb.len() as u16;
    let h = (rows + 4).clamp(5, area.height.saturating_sub(2));
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let rect = Rect::new(x, y, w, h);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.primary))
        .title(Span::styled(
            " Guardar en USB ",
            Style::default().fg(app.palette.primary),
        ));
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    if app.enc.usb.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "no hay medios extraíbles montables",
                    Style::default().fg(app.palette.muted),
                ))
                .alignment(Alignment::Center),
                Line::from(Span::styled(
                    "conecta un USB y vuelve a abrir esta lista",
                    Style::default().fg(app.palette.muted),
                ))
                .alignment(Alignment::Center),
            ]),
            Rect::new(inner.x, inner.y + 1, inner.width, inner.height.saturating_sub(2)),
        );
    } else {
        let mut lines = Vec::new();
        for (i, medium) in app.enc.usb.iter().enumerate() {
            let row = i as u16;
            if row >= inner.height.saturating_sub(1) {
                break;
            }
            lines.push(widgets::list_row(
                &app.palette,
                i == app.enc.usb_index,
                format!("{:<12}", medium.path),
                Some(medium.label.clone()),
            ));
        }
        f.render_widget(
            Paragraph::new(lines),
            Rect::new(inner.x, inner.y, inner.width, inner.height.saturating_sub(1)),
        );
    }

    f.render_widget(
        Paragraph::new(Span::styled(
            "↑↓ elegir · enter guardar · esc cerrar",
            Style::default().fg(app.palette.muted),
        ))
        .alignment(Alignment::Center),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

/// Writes the recovery key to the chosen medium as
/// `nixdots-recovery-<hostname>.txt`. The key is the only secret ever written
/// to a removable disk; it never touches the target disk.
fn save_recovery(app: &mut App) {
    let Some(medium) = app.enc.usb.get(app.enc.usb_index).cloned() else {
        return;
    };
    let name = format!("nixdots-recovery-{}.txt", app.settings.host_name);
    let content = format!(
        "nixdots · clave de recuperación LUKS2\n\
         hostname: {}\n\
         disco: {}\n\n\
         {}\n\n\
         Anótala en un lugar seguro. Se pedirá al arrancar si el TPM no desbloquea.\n",
        app.settings.host_name,
        app.selected_disk().map(|d| d.path()).unwrap_or_default(),
        app.enc.recovery.as_str(),
    );

    if app.dry_run {
        app.enc.usb_saved = Some(medium.path.clone());
        app.status = Some(format!("simulación: {name} → {}", medium.path));
        return;
    }

    let dir = "/tmp/nixdots-usb";
    let _ = std::fs::create_dir_all(dir);
    let mount = Cmd::new("mount")
        .arg("-o")
        .arg("rw")
        .arg(&medium.path)
        .arg(dir);
    match runner::capture(&mount) {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            app.status = Some(format!(
                "no se pudo montar {}: {}",
                medium.path,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
            return;
        }
        Err(err) => {
            app.status = Some(format!("mount: {err}"));
            return;
        }
    }

    let path = format!("{dir}/{name}");
    let write = std::fs::write(&path, content.as_bytes());
    let _ = runner::capture(&Cmd::new("sync"));
    let _ = runner::capture(&Cmd::new("umount").arg(dir));
    match write {
        Ok(()) => {
            app.enc.usb_saved = Some(medium.path.clone());
            app.status = Some(format!("clave guardada en {}", medium.path));
        }
        Err(err) => app.status = Some(format!("no se pudo escribir la clave: {err}")),
    }
}
