//! Keyboard layouts: the list offered by the installer, the active one, and
//! applying it to the Linux console.

use std::process::Command;

/// One keyboard layout.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    /// Stable id used in the picker (e.g. "us-intl").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// xkb layout (written to settings.nix as `keyboardLayout`).
    pub xkb: &'static str,
    /// xkb variant (written as `keyboardVariant`, empty when none).
    pub variant: &'static str,
    /// Console keymap for `loadkeys` on TERM=linux.
    pub keymap: &'static str,
}

pub const LAYOUTS: &[Layout] = &[
    Layout { id: "us", name: "English (US)", xkb: "us", variant: "", keymap: "us" },
    Layout { id: "us-intl", name: "English (US, intl)", xkb: "us", variant: "intl", keymap: "us-acentos" },
    Layout { id: "latam", name: "Español (Latinoamérica)", xkb: "latam", variant: "", keymap: "la-latin1" },
    Layout { id: "es", name: "Español (España)", xkb: "es", variant: "", keymap: "es" },
    Layout { id: "gb", name: "English (UK)", xkb: "gb", variant: "", keymap: "uk" },
    Layout { id: "de", name: "Deutsch", xkb: "de", variant: "", keymap: "de" },
    Layout { id: "fr", name: "Français", xkb: "fr", variant: "", keymap: "fr" },
    Layout { id: "it", name: "Italiano", xkb: "it", variant: "", keymap: "it" },
    Layout { id: "pt", name: "Português", xkb: "pt", variant: "", keymap: "pt" },
    Layout { id: "br", name: "Português (Brasil)", xkb: "br", variant: "", keymap: "br-abnt2" },
    Layout { id: "ca", name: "Català", xkb: "ca", variant: "", keymap: "ca" },
    Layout { id: "ru", name: "Русский", xkb: "ru", variant: "", keymap: "ru" },
    Layout { id: "jp", name: "日本語", xkb: "jp", variant: "", keymap: "jp106" },
];

pub fn by_id(id: &str) -> Option<&'static Layout> {
    LAYOUTS.iter().find(|l| l.id == id)
}

/// The layout currently in use, guessed from `localectl` (falls back to "us").
pub fn detect_active() -> String {
    if let Ok(output) = Command::new("localectl").arg("status").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("X11 Layout:") {
                let value = rest.trim();
                if let Some(layout) = LAYOUTS.iter().find(|l| l.xkb == value) {
                    return layout.id.to_string();
                }
            }
            if let Some(rest) = line.trim().strip_prefix("VC Keymap:") {
                let value = rest.trim();
                if let Some(layout) = LAYOUTS.iter().find(|l| l.keymap == value) {
                    return layout.id.to_string();
                }
            }
        }
    }
    "us".to_string()
}

/// Applies a layout to the bare Linux console with `loadkeys`.
pub fn apply_console(layout: &Layout) -> bool {
    Command::new("loadkeys")
        .arg(layout.keymap)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
