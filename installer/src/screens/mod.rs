//! One module per screen. Each exposes `draw`, `key`, `height`, `commands` and
//! `buttons`. `key` returns whether it consumed the event (text fields, list
//! navigation, search); otherwise the app handles globals.

pub mod disk;
pub mod done;
pub mod encryption;
pub mod git;
pub mod host;
pub mod installing;
pub mod license;
pub mod locale;
pub mod network;
pub mod password;
pub mod picker;
pub mod summary;
pub mod timezone;
pub mod user;
pub mod welcome;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// True when the key should be delivered to a text field.
pub fn is_edit_key(key: &KeyEvent) -> bool {
    !key.modifiers.contains(KeyModifiers::CONTROL)
        && !matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Enter
                | KeyCode::Esc
        )
}
