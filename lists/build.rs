//! Generates the locale name table from the glibc source locale files
//! (`LC_ADDRESS`'s `country_name` and `lang_name`), so the UI can show
//! «Español (Bolivia)» instead of «es_BO». The path comes from
//! `NIXDOTS_LOCALE_DIR` (the full glibc package's `share/i18n/locales`).

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=NIXDOTS_LOCALE_DIR");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("locale_names.rs");

    let mut entries: Vec<(String, String, String)> = Vec::new();
    if let Ok(dir) = std::env::var("NIXDOTS_LOCALE_DIR") {
        if let Ok(read) = std::fs::read_dir(&dir) {
            for entry in read.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let code = entry.file_name().to_string_lossy().to_string();
                let lang = field(&text, "lang_name");
                let country = field(&text, "country_name");
                if lang.is_some() || country.is_some() {
                    entries.push((code, lang.unwrap_or_default(), country.unwrap_or_default()));
                }
            }
        }
    }
    entries.sort();

    let mut src = String::from(
        "/// (locale code, language name, country name), generated from glibc.\n",
    );
    src.push_str("pub static LOCALE_NAMES: &[(&str, &str, &str)] = &[\n");
    for (code, lang, country) in &entries {
        let _ = writeln!(src, "    ({code:?}, {lang:?}, {country:?}),");
    }
    src.push_str("];\n");
    std::fs::write(&out, src).expect("write locale_names.rs");
}

/// The first quoted value of a `key` line inside the locale file.
fn field(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        let start = rest.find('"')?;
        let after = &rest[start + 1..];
        let end = after.find('"')?;
        return Some(after[..end].to_string());
    }
    None
}
