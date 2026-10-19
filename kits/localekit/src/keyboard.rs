//! Teclados: la lista xkb completa (layouts y variantes) embebida desde
//! `base.lst` de `xkeyboard-config`.

use std::sync::OnceLock;

/// `share/X11/xkb/rules/base.lst`, embebido por el paquete
/// (`KEEL_XKB_BASE_LST`).
const XKB_BASE_LST: &str = include_str!(env!("KEEL_XKB_BASE_LST"));

/// Un layout base o una de sus variantes.
#[derive(Debug, Clone)]
pub struct Layout {
    /// Id único: `us` para un layout, `us:intl` para una variante.
    pub code: String,
    /// Nombre legible (de `base.lst`).
    pub name: String,
    /// xkb layout (se escribe como `keyboardLayout`).
    pub xkb: String,
    /// xkb variant (se escribe como `keyboardVariant`, vacío si no hay).
    pub variant: String,
}

/// Todos los layouts y variantes, layouts primero; se parsea una vez.
pub fn all() -> &'static [Layout] {
    static CACHE: OnceLock<Vec<Layout>> = OnceLock::new();
    CACHE.get_or_init(parse_base_lst)
}

/// Busca una entrada por su id.
pub fn by_id(id: &str) -> Option<Layout> {
    all().iter().find(|l| l.code == id).cloned()
}

/// El nombre legible de un par layout/variante, `es` → `Español`.
pub fn display(layout: &str, variant: &str) -> String {
    all()
        .iter()
        .find(|l| l.xkb == layout && l.variant == variant)
        .map(|l| l.name.clone())
        .unwrap_or_else(|| layout.to_string())
}

/// El layout base sugerido por un código de locale (`es_MX` → `latam`).
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

fn by_name(a: &Layout, b: &Layout) -> std::cmp::Ordering {
    a.name
        .to_lowercase()
        .cmp(&b.name.to_lowercase())
        .then_with(|| a.code.cmp(&b.code))
}

fn split_first(s: &str) -> (&str, &str) {
    match s.find(char::is_whitespace) {
        Some(i) => (&s[..i], s[i..].trim_start()),
        None => (s, ""),
    }
}
