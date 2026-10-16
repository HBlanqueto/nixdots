//! Teclados: la tabla compartida vive en `nixdots-lists`; aquí queda lo que sí
//! toca el sistema: detectar el layout activo y aplicarlo a la consola.

use std::process::{Command, Stdio};

pub use nixdots_lists::keyboard::{all, by_id, display, suggest, Layout};

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
