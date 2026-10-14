use crate::app::{Action, App, PickItem, PickerKind};
use crate::detect;
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;
use std::path::Path;

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
            Some(PickerKind::Timezone),
            Some(&app.targets),
            Some(&app.scrollbars),
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
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Continuar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "timedatectl list-timezones".into(),
        format!("settings.nix: timeZone = \"{}\";", app.settings.time_zone),
    ]
}

/// Builds the timezone list with UTC offsets, from `timedatectl` or
/// `/etc/zoneinfo`.
pub fn load_items() -> Vec<PickItem> {
    let zones = match detect::run_trim("timedatectl", &["list-timezones"]) {
        Some(list) => list
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect::<Vec<_>>(),
        None => scan_zoneinfo(),
    };
    let zones = if zones.is_empty() {
        vec!["UTC".to_string(), "America/Merida".to_string(), "Europe/Madrid".to_string()]
    } else {
        zones
    };

    let offsets = zone_offsets(&zones);
    zones
        .into_iter()
        .map(|zone| {
            let name = zone.rsplit('/').next().unwrap_or(&zone).replace('_', " ");
            let extra = offsets
                .iter()
                .find(|(z, _)| z == &zone)
                .map(|(_, off)| off.clone())
                .unwrap_or_default();
            PickItem {
                code: zone,
                name,
                extra,
            }
        })
        .collect()
}

fn zone_offsets(zones: &[String]) -> Vec<(String, String)> {
    let list = zones
        .iter()
        .map(|z| format!("'{}'", z.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!(
        "for z in {list}; do printf '%s\\t%s\\n' \"$z\" \"$(TZ=$z date +%:z 2>/dev/null)\"; done"
    );
    let Ok(output) = std::process::Command::new("sh").arg("-c").arg(&script).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (zone, off) = line.split_once('\t')?;
            Some((zone.to_string(), off.to_string()))
        })
        .collect()
}

fn scan_zoneinfo() -> Vec<String> {
    for base in ["/etc/zoneinfo", "/usr/share/zoneinfo"] {
        let mut zones = Vec::new();
        walk(Path::new(base), Path::new(base), &mut zones);
        if !zones.is_empty() {
            zones.sort();
            zones.dedup();
            return zones;
        }
    }
    Vec::new()
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    const SKIP_DIRS: [&str; 2] = ["posix", "right"];
    const SKIP_FILES: [&str; 12] = [
        "posix",
        "right",
        "zone.tab",
        "zone1970.tab",
        "iso3166.tab",
        "leapseconds",
        "leap-seconds.list",
        "localtime",
        "posixrules",
        "Factory",
        "+VERSION",
        "tzdata.zi",
    ];
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(base, &path, out);
        } else if !SKIP_FILES.contains(&name.as_str()) && name != "SECURITY" {
            if let Ok(rel) = path.strip_prefix(base) {
                let rel = rel.to_string_lossy().to_string();
                if !rel.is_empty() {
                    out.push(rel);
                }
            }
        }
    }
}
