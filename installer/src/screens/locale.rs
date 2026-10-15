//! "Lenguaje": a searchable list of locales named in their own language,
//! centered inside the shared box. The search field is always visible; typing
//! filters. The chosen code is shown by the app under the box.

use crate::app::{Action, App, PickItem};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(picker) = &app.loc else {
        return;
    };
    let list = super::picker::draw_search(
        f,
        &app.palette,
        area,
        picker,
        !app.focus_buttons && !app.no_focus(),
    );
    super::picker::draw_centered(f, &app.palette, list, picker, None);
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    let Some(picker) = app.loc.as_mut() else {
        return false;
    };
    let consumed = super::picker::key(picker, key);
    if consumed {
        if let Some(item) = picker.current() {
            app.settings.default_locale = item.code.clone();
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
    vec![format!(
        "settings.nix: defaultLocale = \"{}\";",
        app.settings.default_locale
    )]
}

/// `share/i18n/SUPPORTED` from glibc, embedded at compile time by the package
/// (`NIXDOTS_LOCALES`). The live ISO only generates `C.UTF-8` and
/// `en_US.UTF-8`, so the list cannot come from `locale -a`.
const SUPPORTED: &str = include_str!(env!("NIXDOTS_LOCALES"));

/// Every UTF-8 locale glibc supports, named in its own language, sorted by
/// name and with duplicate display names collapsed. `C.UTF-8`, `POSIX` and the
/// non-UTF-8 entries are dropped.
pub fn load_items() -> Vec<PickItem> {
    let mut codes = parse_supported(SUPPORTED);
    codes.sort();
    codes.dedup();
    let mut seen = std::collections::HashSet::new();
    let mut items: Vec<PickItem> = codes
        .into_iter()
        .filter(|code| {
            let base = code.split('.').next().unwrap_or(code);
            base != "C" && base != "POSIX" && !base.starts_with("C_")
        })
        .filter_map(|code| {
            let name = display_name(&code);
            if seen.insert(name.clone()) {
                Some(PickItem {
                    name,
                    code,
                    extra: String::new(),
                })
            } else {
                None
            }
        })
        .collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

/// Turns entries like `en_US.UTF-8/UTF-8` (and `ca_ER/UTF-8`) into locale
/// codes, keeping only the UTF-8 ones.
fn parse_supported(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for token in text.split_whitespace() {
        let Some((name, charset)) = token.rsplit_once('/') else {
            continue;
        };
        if charset != "UTF-8" {
            continue;
        }
        let code = if name.contains('.') {
            name.to_string()
        } else {
            format!("{name}.UTF-8")
        };
        codes.push(code);
    }
    codes
}

include!(concat!(env!("OUT_DIR"), "/locale_names.rs"));

/// The locale's name in its own language, e.g. `es_MX.UTF-8` →
/// `Español (México)`, from the table generated out of glibc. Falls back to
/// the bare code when the locale has no data.
pub fn display(code: &str) -> String {
    display_name(code)
}

fn display_name(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let base = base.split('@').next().unwrap_or(base);
    if let Some((_, lang, country)) = LOCALE_NAMES.iter().find(|(c, _, _)| *c == base) {
        if let Some(name) = compose(lang, country) {
            return name;
        }
    }
    code.to_string()
}

/// «Español (Bolivia)» from the locale's language and country names.
fn compose(lang: &str, country: &str) -> Option<String> {
    match (lang.is_empty(), country.is_empty()) {
        (true, true) => None,
        (false, true) => Some(capitalize(lang)),
        (true, false) => Some(country.to_string()),
        (false, false) => Some(format!("{} ({country})", capitalize(lang))),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
