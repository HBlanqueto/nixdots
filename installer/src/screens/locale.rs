//! "Lenguaje": a braille globe, a heading, and a searchable list of locales in
//! their own language. The search field is hidden until `/` is pressed.

use crate::app::{Action, App, Click, PickItem, PickerKind, ScrollKind};
use crate::widgets::Btn;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use tui_input::backend::crossterm::EventHandler;

const ROWS: usize = 8;
const BOX_W: u16 = 46;

/// Braille globe (20×10) taken from the `Lenguaje.dc.html` mockup.
const GLOBE: [&str; 10] = [
    "    ⢀⣠⣴⢶⡶⠚⠓⢶⡶⣦⣄⡀",
    "  ⢀⡴⡿⠋⢠⠏    ⠹⡄⠙⢿⢦⡀",
    " ⣰⠯⡼⠥⠤⡾⠤⠤⠤⠤⠤⠤⢷⠤⠬⢧⠽⣆",
    "⢰⠃⢸⠁ ⢰⠃      ⠘⡆ ⠈⡇⠘⡆",
    "⢸ ⡏  ⢸        ⡇  ⢹ ⡇",
    "⢹⠉⣏⠉⠉⢹⠉⠉⠉⠉⠉⠉⠉⠉⡏⠉⠉⣹⠉⡇",
    "⠸⡄⢸⡀ ⠸⡄      ⢠⠇ ⢀⡇⢠⠇",
    " ⠹⣖⢳⡒⠒⢷⠒⠒⠒⠒⠒⠒⡾⠒⢒⡞⣲⠏",
    "  ⠈⠳⣷⣄⠘⣆    ⣰⠃⣠⣾⠞⠁",
    "    ⠈⠙⠻⠾⠷⠤⠤⠾⠷⠟⠋⠁",
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
    (ROWS + 4) as u16
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let console = crate::theme::console::is_linux_console();
    let globe: Vec<&str> = if console {
        GLOBE_ASCII.to_vec()
    } else {
        GLOBE.to_vec()
    };
    let globe_h = globe.len() as u16;

    let box_w = BOX_W.min(area.width.saturating_sub(4)).max(24);
    let box_h = (ROWS as u16 + 2).min(area.height.saturating_sub(globe_h + 6)).max(4);

    // globe + gap + heading + optional search + box + locale + buttons
    let search_h: u16 = if app.locale_search { 1 } else { 0 };
    let total = globe_h + 1 + 1 + search_h + box_h + 1 + 2;
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

    if app.locale_search {
        let query = app
            .loc
            .as_ref()
            .map(|p| p.query.value().to_string())
            .unwrap_or_default();
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("/ ", Style::default().fg(app.palette.primary)),
                Span::styled(query, Style::default().fg(app.palette.text)),
            ]))
            .alignment(Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }

    let box_x = area.x + area.width.saturating_sub(box_w) / 2;
    draw_list(f, app, Rect::new(box_x, y, box_w, box_h));
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

    app.draw_buttons_at(f, Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 1));
}

fn draw_list(f: &mut Frame, app: &App, rect: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.palette.muted));
    let inner = block.inner(rect);
    f.render_widget(Clear, rect);
    f.render_widget(block, rect);
    let Some(picker) = &app.loc else {
        return;
    };

    let total = picker.filtered.len();
    let start = picker
        .selected
        .unwrap_or(0)
        .saturating_sub(ROWS / 2)
        .min(total.saturating_sub(ROWS));
    let text_w = inner.width.saturating_sub(1) as usize;

    for row in 0..ROWS as u16 {
        if inner.y + row >= inner.bottom() {
            break;
        }
        let row_rect = Rect::new(inner.x, inner.y + row, inner.width, 1);
        app.targets.borrow_mut().push((
            row_rect,
            Click::PickerRow(PickerKind::Language, start + row as usize),
        ));
        let Some(&item_idx) = picker.filtered.get(start + row as usize) else {
            continue;
        };
        let item = &picker.items[item_idx];
        let selected = picker.selected == Some(start + row as usize);
        let style = if selected {
            Style::default()
                .fg(app.palette.white)
                .bg(app.palette.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.palette.text)
        };
        let name: String = item.name.chars().take(text_w).collect();
        let pad = text_w.saturating_sub(name.chars().count()) / 2;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(" ".repeat(pad)),
                Span::styled(name, style),
            ])),
            row_rect,
        );
    }

    draw_scrollbar(f, app, inner, start, total);
}

/// A `█`-over-`│` scrollbar in the last inner column.
fn draw_scrollbar(f: &mut Frame, app: &App, inner: Rect, start: usize, total: usize) {
    if total <= ROWS || inner.width == 0 {
        return;
    }
    let x = inner.right().saturating_sub(1);
    let rows = ROWS.min(inner.height as usize);
    let thumb_h = (rows * rows / total).max(1);
    let travel = rows.saturating_sub(thumb_h);
    let max_start = total.saturating_sub(rows);
    let thumb_top = start.saturating_mul(travel).checked_div(max_start).unwrap_or(0);
    for row in 0..rows {
        let ch = if row >= thumb_top && row < thumb_top + thumb_h {
            "█"
        } else {
            "│"
        };
        let style = if ch == "█" {
            Style::default().fg(app.palette.primary)
        } else {
            Style::default().fg(app.palette.row_bg)
        };
        f.render_widget(
            Paragraph::new(Span::styled(ch, style)),
            Rect::new(x, inner.y + row as u16, 1, 1),
        );
    }
    app.scrollbars.borrow_mut().push((
        Rect::new(x, inner.y, 1, rows as u16),
        ScrollKind::Picker(PickerKind::Language),
    ));
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    if app.locale_search {
        return search_key(app, key);
    }
    match key.code {
        KeyCode::Char('/') => {
            app.locale_search = true;
            true
        }
        KeyCode::Up => {
            if let Some(p) = app.loc.as_mut() {
                p.up();
            }
            sync_locale(app);
            true
        }
        KeyCode::Down => {
            if let Some(p) = app.loc.as_mut() {
                p.down();
            }
            sync_locale(app);
            true
        }
        KeyCode::PageUp | KeyCode::PageDown => {
            let delta = if key.code == KeyCode::PageUp { -(ROWS as isize) } else { ROWS as isize };
            app.move_locale_by(delta);
            sync_locale(app);
            true
        }
        _ => false,
    }
}

fn search_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.locale_search = false;
            if let Some(p) = app.loc.as_mut() {
                p.query = tui_input::Input::default();
                p.refilter();
            }
            true
        }
        KeyCode::Enter => {
            app.locale_search = false;
            sync_locale(app);
            true
        }
        KeyCode::Up => {
            if let Some(p) = app.loc.as_mut() {
                p.up();
            }
            sync_locale(app);
            true
        }
        KeyCode::Down => {
            if let Some(p) = app.loc.as_mut() {
                p.down();
            }
            sync_locale(app);
            true
        }
        _ if crate::screens::is_edit_key(&key) => {
            if let Some(p) = app.loc.as_mut() {
                p.query.handle_event(&Event::Key(key));
                p.refilter();
            }
            sync_locale(app);
            true
        }
        _ => false,
    }
}

fn sync_locale(app: &mut App) {
    if let Some(item) = app.loc.as_ref().and_then(|p| p.current()) {
        app.settings.default_locale = item.code.clone();
    }
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

/// Every UTF-8 locale glibc supports, taken from the embedded table.
pub fn load_items() -> Vec<PickItem> {
    let mut codes = parse_supported(SUPPORTED);
    codes.sort();
    codes.dedup();
    codes
        .into_iter()
        .map(|code| PickItem {
            name: display_name(&code),
            code,
            extra: String::new(),
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
/// Falls back to the language name, then to the raw code.
fn display_name(code: &str) -> String {
    let base = code.split('.').next().unwrap_or(code);
    let base = base.strip_suffix("@euro").unwrap_or(base);
    if let Some(name) = exact_name(base) {
        return name.to_string();
    }
    let lang = base.split('_').next().unwrap_or("");
    if let Some(name) = language_name(lang) {
        return name.to_string();
    }
    code.to_string()
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
