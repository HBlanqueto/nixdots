//! "Lenguaje": a searchable list of locales named in their own language,
//! centered inside the shared box. The search field is always visible; typing
//! filters. The chosen code is shown by the app under the box.
//!
//! La tabla vive en `nixdots-lists`; aquí solo se adapta al TUI.

use crate::app::{Action, App, PickItem};
use crate::widgets::Btn;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub use nixdots_lists::locale::display;

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
        app.blink(),
    );
    super::picker::draw_centered(
        f,
        &app.palette,
        list,
        picker,
        None,
        app.list_focus && !app.focus_buttons && !app.no_focus(),
    );
}

pub fn key(app: &mut App, key: KeyEvent) -> bool {
    super::picker::update_list_focus(app, &key);
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

/// Adapta la tabla compartida al tipo del TUI.
pub fn load_items() -> Vec<PickItem> {
    nixdots_lists::locale::all()
        .into_iter()
        .map(|l| PickItem {
            name: l.name,
            code: l.code,
            extra: String::new(),
        })
        .collect()
}
