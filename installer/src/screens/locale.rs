//! "Lenguaje": a braille globe, a heading, and a searchable list of locales in
//! their own language. The search field is always visible; typing filters.

use crate::app::{Action, App, PickItem};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Braille globe (20×10) drawn for the `Lenguaje` screen.
const GLOBE: [&str; 10] = [
    "     ⣀⣤⣴⡶⠶⠶⢶⣦⣤⣀",
    "  ⢀⣴⣾⠟⢁⡞    ⢳⡈⠻⣿⣦⡀",
    " ⢠⡞⣳⠓⠒⡞⠒⠒⠒⠒⠒⠒⢳⠒⠚⣞⢻⣆",
    "⢀⡟⢠⠃ ⢠⠇      ⠸⡄ ⠘⡄⢻⡄",
    "⢸⡇⢸  ⢸        ⡇  ⡇⠈⡇",
    "⢸⡏⢹⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⡏⠉⠉⡏⠉⡇",
    "⠈⣧⠘⡄ ⠘⡆      ⢰⠃ ⢠⠃⣸⠃",
    " ⠘⣧⡽⡤⠤⢧⠤⠤⠤⠤⠤⠤⡼⠤⢤⠯⣴⠏",
    "  ⠈⠻⣿⣦⡈⢧    ⡼⢁⣴⣯⠞⠁",
    "    ⠈⠙⠛⠿⠷⠦⠴⠾⠿⠛⠋⠁",
];

/// Plain-ASCII fallback for the Linux console (`TERM=linux`), which has no
/// braille glyphs.
const GLOBE_ASCII: [&str; 9] = [
    "       .-'''''-.",
    "     .'  .-+-.  '.",
    "    /   /  |  \\   \\",
    "   |---+---+---+---|",
    "    \\   \\  |  /   /",
    "     '.  '-+-'  .'",
    "       '-.....-'",
    "",
    "",
];

pub fn height(_app: &App) -> u16 {
    (super::picker::ROWS + 4) as u16
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let console = crate::theme::console::is_linux_console();
    let globe: Vec<&str> = if console {
        GLOBE_ASCII.to_vec()
    } else {
        GLOBE.to_vec()
    };
    let globe_h = globe.len() as u16;

    let box_w = super::picker::CENTERED_W
        .min(area.width.saturating_sub(4))
        .max(24);
    let box_h = (super::picker::ROWS as u16 + 2)
        .min(area.height.saturating_sub(globe_h + 6))
        .max(4);

    // globe + gap + heading + search + box + gray + buttons
    let total = globe_h + 1 + 1 + 1 + box_h + 1 + 2;
    let mut y = area.y + area.height.saturating_sub(total) / 2 + 1;

    for line in &globe {
        f.render_widget(
            Paragraph::new(Span::styled(
                line.to_string(),
                Style::default().fg(app.palette.primary),
            ))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
    y += 1;

    f.render_widget(
        Paragraph::new(Span::styled(
            "Lenguaje",
            Style::default()
                .fg(app.palette.text)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
    y += 1;

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    if let Some(picker) = &app.loc {
        let search = super::picker::search_line(
            &app.palette,
            picker,
            "buscar idioma",
            "idiomas",
            box_w,
            !app.focus_buttons,
        );
        f.render_widget(Paragraph::new(search), Rect::new(box_x, y, box_w, 1));
    }
    y += 1;

    if let Some(picker) = &app.loc {
        super::picker::draw_centered(
            f,
            &app.palette,
            Rect::new(box_x, y, box_w, box_h),
            picker,
            None,
        );
    }
    y += box_h + 1;

    let chosen = app
        .loc
        .as_ref()
        .and_then(|p| p.current())
        .map(|item| item.code.clone())
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(Span::styled(chosen, Style::default().fg(app.palette.muted)))
            .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );

    app.draw_buttons_at(
        f,
        Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 1),
    );
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
    vec![format!(
        "settings.nix: defaultLocale = \"{}\";",
        app.settings.default_locale
    )]
}

/// `share/i18n/SUPPORTED` from glibc, embedded at compile time by the package
/// (`NIXDOTS_LOCALES`). The live ISO only generates `C.UTF-8` and
/// `en_US.UTF-8`, so the list cannot come from `locale -a`.
const SUPPORTED: &str = include_str!(env!("NIXDOTS_LOCALES"));

/// Every UTF-8 locale glibc supports, with duplicate display names collapsed so
/// the list stays short.
pub fn load_items() -> Vec<PickItem> {
    let mut codes = parse_supported(SUPPORTED);
    codes.sort();
    codes.dedup();
    let mut seen = std::collections::HashSet::new();
    codes
        .into_iter()
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
        .collect()
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
