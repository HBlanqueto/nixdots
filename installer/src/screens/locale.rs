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

/// The locale's name in its own language, e.g. `es_MX` → `Español (México)`.
/// Falls back to the language name plus region code, then to the raw code.
/// The readable name of a locale code, e.g. `es_MX.UTF-8` → `Español (México)`.
pub fn display(code: &str) -> String {
    display_name(code)
}

fn display_name(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let base = base.split('@').next().unwrap_or(base);
    if let Some(name) = exact_name(base) {
        return name.to_string();
    }
    let mut parts = base.split('_');
    let lang = parts.next().unwrap_or("");
    let region = parts.next().unwrap_or("");
    let Some(language) = language_name(lang) else {
        return code.to_string();
    };
    if region.is_empty() {
        language.to_string()
    } else {
        format!("{language} ({region})")
    }
}

fn exact_name(base: &str) -> Option<&'static str> {
    Some(match base {
        "en_US" => "English (US)",
        "en_GB" => "English (UK)",
        "en_CA" => "English (Canada)",
        "en_AU" => "English (Australia)",
        "es_MX" => "Español (México)",
        "es_ES" => "Español (España)",
        "es_AR" => "Español (Argentina)",
        "es_CO" => "Español (Colombia)",
        "es_CL" => "Español (Chile)",
        "es_PE" => "Español (Perú)",
        "pt_BR" => "Português (Brasil)",
        "pt_PT" => "Português (Portugal)",
        "fr_FR" => "Français",
        "fr_CA" => "Français (Canada)",
        "de_DE" => "Deutsch",
        "it_IT" => "Italiano",
        "nl_NL" => "Nederlands",
        "ru_RU" => "Русский",
        "ja_JP" => "日本語",
        "zh_CN" => "中文 (中国)",
        "zh_TW" => "中文 (台灣)",
        "ko_KR" => "한국어",
        "pl_PL" => "Polski",
        "sv_SE" => "Svenska",
        "tr_TR" => "Türkçe",
        "cs_CZ" => "Čeština",
        "da_DK" => "Dansk",
        "fi_FI" => "Suomi",
        "el_GR" => "Ελληνικά",
        "he_IL" => "עברית",
        "uk_UA" => "Українська",
        "ca_ES" => "Català",
        "hi_IN" => "हिन्दी",
        "ar_EG" => "العربية",
        _ => return None,
    })
}

fn language_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "en" => "English",
        "es" => "Español",
        "fr" => "Français",
        "de" => "Deutsch",
        "pt" => "Português",
        "it" => "Italiano",
        "ca" => "Català",
        "nl" => "Nederlands",
        "ru" => "Русский",
        "ja" => "日本語",
        "zh" => "中文",
        "ko" => "한국어",
        "ar" => "العربية",
        "hi" => "हिन्दी",
        "pl" => "Polski",
        "sv" => "Svenska",
        "tr" => "Türkçe",
        "cs" => "Čeština",
        "da" => "Dansk",
        "fi" => "Suomi",
        "el" => "Ελληνικά",
        "he" => "עברית",
        "uk" => "Українська",
        _ => return None,
    })
}
