use crate::app::{Action, App};
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

pub fn height(_app: &App) -> u16 {
    2
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let default_hint = match app.arch.as_deref() {
        Some("aarch64") | Some("arm64") => "por defecto: máquina virtual aarch64",
        Some("x86_64") | Some("amd64") => "por defecto: x86_64",
        _ => "por defecto: nixos",
    };
    let mut line = widgets::input_line(
        &app.palette,
        "hostname",
        app.host.input.value(),
        app.host.input.visual_cursor(),
        true,
        false,
        area.width,
        true,
    );
    if !app.host.input.value().is_empty() && app.hostname_valid() {
        line.spans.push(Span::styled("  ✓", Style::default().fg(app.palette.success)));
    } else if !app.host.input.value().is_empty() {
        line.spans.push(Span::styled("  ✗", Style::default().fg(app.palette.error)));
    }
    let lines = vec![
        line,
        Line::from(Span::styled(default_hint, Style::default().fg(app.palette.muted))),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if super::is_edit_key(&key) {
        app.host.input.handle_event(&Event::Key(key));
        return true;
    }
    false
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Continuar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![format!(
        "settings.nix: hostName = \"{}\";",
        app.host.input.value()
    )]
}
