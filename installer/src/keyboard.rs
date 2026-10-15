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

/// The readable name for an xkb layout/variant pair, e.g. `es` → `Español`.
/// Falls back to the layout code when there is no matching entry.
pub fn display(layout: &str, variant: &str) -> String {
    all()
        .iter()
        .find(|l| l.xkb == layout && l.variant == variant)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| layout.to_string())
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
/// Its output is captured through [`crate::runner`] (a real command, never a
/// raw terminal write), so nothing lands on top of the TUI.
pub fn apply_console(layout: &Layout) -> ConsoleOutcome {
    match console_keymap(layout) {
        Some(keymap) => {
            if run_loadkeys(&keymap) {
                ConsoleOutcome::Applied
            } else {
                ConsoleOutcome::Failed
            }
        }
        None => {
            // No console equivalent: fall back to `us` and let the caller warn.
            let _ = run_loadkeys("us");
            ConsoleOutcome::Fallback
        }
    }
}

/// What happened when applying a layout on the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleOutcome {
    Applied,
    /// No console keymap for this layout; `us` was applied instead.
    Fallback,
    Failed,
}

fn run_loadkeys(keymap: &str) -> bool {
    let cmd = crate::runner::Cmd::new("loadkeys").arg(keymap);
    let mut sink = |_out: crate::runner::Stream| {};
    crate::runner::run(&cmd, &mut sink) == 0
}

/// The console keymap (`loadkeys`) that matches an xkb layout/variant. `None`
/// when the layout has no console equivalent.
fn console_keymap(layout: &Layout) -> Option<String> {
    Some(
        match (layout.xkb.as_str(), layout.variant.as_str()) {
            ("us", "") => "us",
            ("us", "intl") => "us-acentos",
            ("latam", "") => "la-latin1",
            ("es", "") => "es",
            ("gb", "") => "uk",
            ("br", "") => "br-abnt2",
            ("fr", "") => "fr",
            ("de", "") => "de",
            ("it", "") => "it",
            ("pt", "") => "pt-latin1",
            ("ru", "") => "ru",
            ("pl", "") => "pl",
            ("nl", "") => "nl",
            ("jp", "") => "jp106",
            _ => return None,
        }
        .to_string(),
    )
}

/// The base layout suggested by a locale code (`es_MX` → `latam`, `en_US` →
/// `us`, …). `None` when there is no known suggestion.
pub fn suggest(locale: &str) -> Option<&'static str> {
    let mut parts = locale.split(['_', '.', '@']);
    let lang = parts.next().unwrap_or("");
    let region = parts.next().unwrap_or("");
    Some(match (lang, region) {
        ("es", "MX") => "latam",
        ("es", _) => "es",
        ("en", "GB") => "gb",
        ("en", _) => "us",
        ("pt", "BR") => "br",
        ("pt", _) => "pt",
        ("fr", _) => "fr",
        ("de", _) => "de",
        ("it", _) => "it",
        ("nl", _) => "nl",
        ("ru", _) => "ru",
        ("ja", _) | ("jp", _) => "jp",
        ("pl", _) => "pl",
        _ => return None,
    })
}
