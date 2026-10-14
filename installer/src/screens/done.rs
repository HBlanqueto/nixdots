use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn height(_app: &App) -> u16 {
    3
}

/// `* nixdots` + a blank line + the Salir / Reiniciar capsule row.
pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let total: u16 = 3;
    let y = area.y + area.height.saturating_sub(total) / 2;
    f.render_widget(
        Paragraph::new(app.header_line()).alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );

    let buttons: Vec<Btn> = app.buttons().into_iter().map(|(b, _)| b).collect();
    app.button_hitboxes = widgets::render_buttons(
        f,
        Rect::new(area.x, y + 2, area.width, 1),
        &app.palette,
        &buttons,
        app.button_focus,
        app.focus_buttons,
    );
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if key.code == KeyCode::Char('q') {
        salir(app);
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("Salir"), Action::Finish),
        (Btn::primary("Reiniciar ›"), Action::Reboot),
    ]
}

/// Copies the install log into /mnt/persist when that mount exists.
fn copy_log(app: &App) {
    if app.dry_run {
        return;
    }
    if let Some(path) = &app.install.log_path {
        let dir = Path::new("/mnt/persist");
        if dir.is_dir() {
            let _ = std::fs::copy(path, dir.join("nixdots-install.log"));
        }
    }
}

fn umount() {
    let _ = Command::new("umount").arg("-R").arg("/mnt").stdin(Stdio::null()).status();
}

/// Reiniciar: copy the log, unmount and reboot (simulation just closes).
pub fn restart(app: &mut App) {
    copy_log(app);
    if app.dry_run {
        app.should_quit = true;
        return;
    }
    umount();
    let _ = Command::new("reboot").stdin(Stdio::null()).status();
    app.should_quit = true;
}

/// Salir: copy the log, unmount and drop to the shell without rebooting.
pub fn salir(app: &mut App) {
    copy_log(app);
    if !app.dry_run {
        umount();
    }
    app.should_quit = true;
}

pub fn commands(_app: &App) -> Vec<String> {
    Vec::new()
}
