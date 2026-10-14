//! "Idioma y teclado": two columns (language and keyboard layout), plus a
//! test field to type into.

use crate::app::{Action, App, PickItem, PickerKind};
use crate::detect;
use crate::widgets::{self, Btn};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

const ROWS: usize = 9;

pub fn height(_app: &App) -> u16 {
    // titles + pickers + blank + test field
    (ROWS + 3) as u16
}

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let half = area.width / 2;
    let cols_h = ROWS as u16;
    let left = Rect::new(area.x, area.y + 1, half.saturating_sub(1), cols_h);
    let right = Rect::new(area.x + half + 1, area.y + 1, area.width.saturating_sub(half + 1), cols_h);

    let title = |text: &str, focused: bool| {
        Span::styled(
            text.to_string(),
            Style::default().fg(if focused {
                app.palette.primary
            } else {
                app.palette.muted
            }),
        )
    };
    let lang_focus = app.locale_focus == 0 && !app.focus_buttons;
    let kb_focus = app.locale_focus == 1 && !app.focus_buttons;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            title("Idioma", lang_focus),
            Span::raw(" ".repeat((half as usize).saturating_sub(7))),
            title("Distribución del teclado", kb_focus),
        ])),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if let Some(picker) = &app.loc {
        super::picker::draw(f, &app.palette, left, picker, "buscar idioma", "idiomas", lang_focus, Some(PickerKind::Language), Some(&app.targets), Some(&app.scrollbars));
    }
    if let Some(picker) = &app.keyboards {
        super::picker::draw(
            f,
            &app.palette,
            right,
            picker,
            "buscar distribución",
            "distribuciones",
            kb_focus,
            Some(PickerKind::Keyboard),
            Some(&app.targets),
            Some(&app.scrollbars),
        );
    }
    // Column backgrounds are pushed last so a click on a row selects it.
    app.targets.borrow_mut().push((left, crate::app::Click::LocaleColumn(0)));
    app.targets.borrow_mut().push((right, crate::app::Click::LocaleColumn(1)));

    // Test field.
    let test_y = area.y + 1 + cols_h + 1;
    let test_focus = app.locale_focus == 2 && !app.focus_buttons;
    let field = widgets::input_line(
        &app.palette,
        "prueba",
        app.key_test.value(),
        app.key_test.visual_cursor(),
        test_focus,
        false,
        area.width,
        true,
        "escribe aquí para probar el teclado",
    );
    f.render_widget(Paragraph::new(field), Rect::new(area.x, test_y, area.width, 1));
    app.targets
        .borrow_mut()
        .push((Rect::new(area.x, test_y, area.width, 1), crate::app::Click::LocaleColumn(2)));
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Tab if app.locale_focus < 2 => {
            app.locale_focus += 1;
            true
        }
        KeyCode::BackTab if app.locale_focus > 0 => {
            app.locale_focus -= 1;
            true
        }
        _ => match app.locale_focus {
            0 => {
                let consumed = app
                    .loc
                    .as_mut()
                    .map(|p| super::picker::key(p, key))
                    .unwrap_or(false);
                if consumed {
                    if let Some(item) = app.loc.as_ref().and_then(|p| p.current()) {
                        app.settings.default_locale = item.code.clone();
                    }
                }
                consumed
            }
            1 => {
                let consumed = app
                    .keyboards
                    .as_mut()
                    .map(|p| super::picker::key(p, key))
                    .unwrap_or(false);
                if consumed {
                    let code = app.keyboards.as_ref().and_then(|p| p.current()).map(|i| i.code.clone());
                    if let Some(code) = code {
                        app.apply_keyboard_q(&code, false);
                    }
                }
                consumed
            }
            _ => {
                if super::is_edit_key(&key) {
                    app.key_test.handle_event(&Event::Key(key));
                    true
                } else {
                    false
                }
            }
        },
    }
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
        format!("localectl set-x11-keymap {} {}", app.keyboard_layout, app.keyboard_variant),
        format!(
            "settings.nix: defaultLocale = \"{}\";",
            app.settings.default_locale
        ),
        format!("settings.nix: keyboardLayout = \"{}\";", app.keyboard_layout),
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
        codes = ["C.UTF-8", "en_US.UTF-8", "es_ES.UTF-8", "es_MX.UTF-8"]
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
