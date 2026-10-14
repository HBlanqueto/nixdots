use crate::app::{Action, App, PickItem};
use crate::detect;
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub fn height(_app: &App) -> u16 {
    super::picker::height()
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    if let Some(picker) = &app.loc {
        super::picker::draw(
            f,
            &app.palette,
            area,
            picker,
            "buscar idioma",
            "idiomas",
            !app.focus_buttons,
        );
    }
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
        (Btn::secondary("‹ Atrás"), Action::Back),
        (Btn::primary("Continuar ›"), Action::Next),
    ]
}

pub fn commands(app: &App) -> Vec<String> {
    vec![
        "locale -a".into(),
        format!(
            "settings.nix: defaultLocale = \"{}\";",
            app.settings.default_locale
        ),
    ]
}

/// Locales the live system supports, via `locale -a`.
pub fn load_items() -> Vec<PickItem> {
    let mut codes: Vec<String> = detect::run_trim("locale", &["-a"])
        .map(|out| {
            out.lines()
                .map(str::trim)
                .filter(|c| is_utf8_locale(c))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    codes.sort();
    codes.dedup();
    if codes.is_empty() {
        codes = [
            "C.UTF-8",
            "en_US.UTF-8",
            "es_ES.UTF-8",
            "es_MX.UTF-8",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
    }
    codes
        .into_iter()
        .map(|code| PickItem {
            name: display_name(&code),
            code,
            extra: String::new(),
        })
        .collect()
}

fn is_utf8_locale(code: &str) -> bool {
    let lower = code.to_lowercase();
    lower.ends_with(".utf-8") || lower.ends_with(".utf8")
}

/// Human-readable name derived from the locale code (language + region).
fn display_name(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let parts: Vec<&str> = base.split('_').collect();
    let language = language_name(parts.first().copied().unwrap_or(""));
    match parts.get(1).and_then(|r| region_name(r)) {
        Some(region) if !language.is_empty() => format!("{language} ({region})"),
        _ if !language.is_empty() => language.to_string(),
        _ => code.to_string(),
    }
}

fn language_name(code: &str) -> &'static str {
    match code {
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
        _ => "",
    }
}

fn region_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "US" => "Estados Unidos",
        "GB" => "Reino Unido",
        "MX" => "México",
        "ES" => "España",
        "AR" => "Argentina",
        "CO" => "Colombia",
        "CL" => "Chile",
        "PE" => "Perú",
        "BR" => "Brasil",
        "CA" => "Canadá",
        "FR" => "Francia",
        "DE" => "Alemania",
        "IT" => "Italia",
        "PT" => "Portugal",
        "NL" => "Países Bajos",
        "RU" => "Rusia",
        "JP" => "Japón",
        "CN" => "China",
        "KR" => "Corea",
        "AU" => "Australia",
        _ => return None,
    })
}
