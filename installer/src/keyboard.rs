//! Keyboard layouts: the full xkb list (layouts and variants) embedded from
//! `xkeyboard-config`'s `base.lst`, the layout currently in use, and applying
//! it to the bare Linux console with `loadkeys`.
//!
//! Nothing is asked of the running system: the table is compiled into the
//! binary, so it is complete even on the minimal ISO.

use std::process::{Command, Stdio};
use std::sync::OnceLock;

/// `share/X11/xkb/rules/base.lst` from `xkeyboard-config`, embedded at compile
/// time by the package (`NIXDOTS_XKB_BASE_LST`).
const XKB_BASE_LST: &str = include_str!(env!("NIXDOTS_XKB_BASE_LST"));

/// One entry of the picker: either a base layout or one of its variants.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Unique picker id: `us` for a layout, `us:intl` for a variant.
    pub code: String,
    /// Human-readable name (from `base.lst`).
    pub name: String,
    /// xkb layout (written to settings.nix as `keyboardLayout`).
    pub xkb: String,
    /// xkb variant (written as `keyboardVariant`, empty when none).
    pub variant: String,
}

/// Every layout and variant, layouts first; parsed once and cached.
pub fn all() -> &'static [Layout] {
    static CACHE: OnceLock<Vec<Layout>> = OnceLock::new();
    CACHE.get_or_init(parse_base_lst)
}

/// Looks up an entry by its picker id.
pub fn by_id(id: &str) -> Option<Layout> {
    all().iter().find(|l| l.code == id).cloned()
}

fn parse_base_lst() -> Vec<Layout> {
    let mut layouts: Vec<Layout> = Vec::new();
    let mut variants: Vec<Layout> = Vec::new();
    let mut section = "";
    for raw in XKB_BASE_LST.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix('!') {
            section = rest.trim();
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match section {
            "layout" => {
                let (code, name) = split_first(line);
                if !code.is_empty() {
                    layouts.push(Layout {
                        code: code.to_string(),
                        name: name.to_string(),
                        xkb: code.to_string(),
                        variant: String::new(),
                    });
                }
            }
            "variant" => {
                let (variant, rest) = split_first(line);
                let (layout, name) = match rest.split_once(':') {
                    Some((layout, name)) => (layout.trim(), name.trim()),
                    None => (rest, ""),
                };
                if variant.is_empty() || layout.is_empty() {
                    continue;
                }
                variants.push(Layout {
                    code: format!("{layout}:{variant}"),
                    name: name.to_string(),
                    xkb: layout.to_string(),
                    variant: variant.to_string(),
                });
            }
            _ => {}
        }
    }

    layouts.sort_by(by_name);
    variants.sort_by(by_name);

    let mut all: Vec<Layout> = Vec::with_capacity(layouts.len() + variants.len());
    for layout in layouts.into_iter().chain(variants) {
        if !all.iter().any(|l| l.code == layout.code) {
            all.push(layout);
        }
    }
    all
}

/// Sorts layouts and variants by human name, then by code.
fn by_name(a: &Layout, b: &Layout) -> std::cmp::Ordering {
    a.name
        .to_lowercase()
        .cmp(&b.name.to_lowercase())
        .then_with(|| a.code.cmp(&b.code))
}

/// Splits `"code   rest"` into its first whitespace-delimited column and the
/// trimmed remainder.
fn split_first(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}

/// The layout currently in use, guessed from `localectl` (falls back to "us").
pub fn detect_active() -> String {
    if let Ok(output) = Command::new("localectl")
        .arg("status")
        .stdin(Stdio::null())
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut xkb = String::new();
        let mut variant = String::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("X11 Layout:") {
                xkb = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("X11 Variant:") {
                variant = rest.trim().to_string();
            }
        }
        if !xkb.is_empty() {
            let full = if variant.is_empty() {
                xkb.clone()
            } else {
                format!("{xkb}:{variant}")
            };
            if all().iter().any(|l| l.code == full) {
                return full;
            }
            if all().iter().any(|l| l.code == xkb) {
                return xkb;
            }
        }
    }
    "us".to_string()
}

/// Applies a layout to the bare Linux console with `loadkeys`.
///
/// Its output is discarded: many xkb layouts have no console keymap, so
/// `loadkeys` fails often, and anything it writes to the terminal would land
/// on top of the TUI. The caller only needs the success flag.
pub fn apply_console(layout: &Layout) -> bool {
    Command::new("loadkeys")
        .arg(console_keymap(layout))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The console keymap (`loadkeys`) that matches an xkb layout/variant. When
/// there is no exact match the xkb layout name is used, which is correct for
/// the common layouts whose names coincide.
fn console_keymap(layout: &Layout) -> String {
    match (layout.xkb.as_str(), layout.variant.as_str()) {
        ("us", "") => "us".to_string(),
        ("us", "intl") => "us-acentos".to_string(),
        ("latam", "") => "la-latin1".to_string(),
        ("gb", "") => "uk".to_string(),
        ("br", "") => "br-abnt2".to_string(),
        ("jp", "") => "jp106".to_string(),
        _ => layout.xkb.clone(),
    }
}
