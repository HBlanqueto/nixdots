use crate::app::{Action, App, PickItem};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::process::{Command, Stdio};

/// `share/zoneinfo/zone1970.tab` from tzdata, embedded at compile time by the
/// package (`NIXDOTS_ZONE1970_TAB`).
const ZONE1970_TAB: &str = include_str!(env!("NIXDOTS_ZONE1970_TAB"));

pub fn height(_app: &App) -> u16 {
    super::picker::height()
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    if let Some(picker) = &app.timezones {
        super::picker::draw(
            f,
            &app.palette,
            area,
            picker,
            "buscar zona horaria",
            "zonas",
            !app.focus_buttons,
        );
    }
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    let Some(picker) = app.timezones.as_mut() else {
        return false;
    };
    let consumed = super::picker::key(picker, key);
    if consumed {
        if let Some(item) = picker.current() {
            app.settings.time_zone = item.code.clone();
        }
    }
    consumed
}

pub fn buttons(_app: &App) -> Vec<(Btn, Action)> {
    vec![
        (Btn::new("Atrás"), Action::Back),
        (Btn::new("Continuar"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![format!("settings.nix: timeZone = \"{}\";", app.settings.time_zone)]
}

/// Builds the timezone list with each zone's current local time, from the
/// embedded tzdata table.
pub fn load_items() -> Vec<PickItem> {
    let zones = parse_zones(ZONE1970_TAB);
    let times = zone_times(&zones);
    zones
        .into_iter()
        .map(|zone| {
            let time = times
                .iter()
                .find(|(z, _)| z == &zone)
                .map(|(_, t)| t.clone())
                .unwrap_or_default();
            PickItem {
                code: zone,
                name: String::new(),
                extra: time,
            }
        })
        .collect()
}

/// Parses the tab-separated `zone1970.tab` (country, coordinates, TZ, notes),
/// always starting with UTC.
fn parse_zones(text: &str) -> Vec<String> {
    let mut zones = vec!["UTC".to_string()];
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let Some(zone) = line.split('\t').nth(2) else {
            continue;
        };
        let zone = zone.trim();
        if !zone.is_empty() {
            zones.push(zone.to_string());
        }
    }
    zones
}

fn zone_times(zones: &[String]) -> Vec<(String, String)> {
    let script = zones
        .iter()
        .map(|zone| {
            format!(
                "printf '%s\\t%s\\n' {z} \"$(TZ={z} date +%H:%M)\"",
                z = shell_quote(zone)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .stdin(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (zone, time) = line.split_once('\t')?;
            Some((zone.to_string(), time.to_string()))
        })
        .collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
