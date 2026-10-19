//! Datos de cuentas fuera del repo, un archivo por usuario en
//! `${NIXDOTS_ROOT}/persist/nixdots/users/<usuario>.toml` (respaldo: `.json`).
//!
//! El flake los lee y crea con ellos `users.users.<nombre>` y su home-manager;
//! no se versionan ni se pierden al reconstruir. `hashedPasswordFile` no se
//! guarda: se deriva del usuario. El repo vive en `/persist/nixdots/repo`,
//! nunca dentro de estos archivos.
//!
//! Los datos del sistema (locale, zona, teclado, autologin…) NO van aquí: van
//! en `machine.toml` (ver [`crate::machine`]). La semilla de apariencia y
//! accesibilidad vive aparte, en `users/<usuario>.prefs.toml`.

use crate::store;
use color_eyre::eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use toml_edit::{value, Item, Table};

/// Preferencias de accesibilidad (todas `false` por defecto).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct A11y {
    #[serde(default)]
    pub large_text: bool,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub screen_reader: bool,
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default)]
    pub screen_keyboard: bool,
}

/// Identidad de GitHub/Git del usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Git {
    pub name: String,
    pub email: String,
}

fn default_true() -> bool {
    true
}

fn default_template() -> String {
    "nixdots".into()
}

fn default_version() -> u32 {
    1
}

/// Lo que se escribe en `users/<usuario>.toml` (camelCase, como lo lee el flake).
/// Los campos opcionales se omiten si son `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    #[serde(default = "default_version")]
    pub version: u32,
    pub username: String,
    /// Nombre visible («Humberto Blanqueto»).
    pub full_name: String,
    /// Miembro de `wheel`.
    #[serde(default = "default_true")]
    pub admin: bool,
    /// Plantilla de home-manager (`"nixdots"` o `"minimal"`).
    #[serde(default = "default_template")]
    pub template: String,
    /// Importar `/persist/nixdots/users/<usuario>/home.nix` sobre la plantilla.
    #[serde(default)]
    pub personal_config: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<Git>,
    /// `"light"` o `"dark"`: preferencia de tema elegida en el asistente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    /// Acento elegido (`"neptunia"` o un id de GNOME).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// Color del avatar elegido en el asistente (id de acento).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    /// Ruta de la foto del avatar (variante con foto).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a11y: Option<A11y>,
}

/// Semilla de un solo uso con la apariencia y accesibilidad del asistente, en
/// `users/<usuario>.prefs.toml`. `nixdots-home-init` la copia al flake del
/// usuario y borra el original.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a11y: Option<A11y>,
}

/// `${NIXDOTS_ROOT}/persist/nixdots/users`. Construido como cadena: sin
/// `NIXDOTS_ROOT` la ruta debe quedar absoluta (`/persist/...`), no relativa.
pub fn users_dir() -> PathBuf {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    PathBuf::from(format!("{root}/persist/nixdots/users"))
}

/// `users/<username>.toml`.
pub fn path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.toml"))
}

/// `users/<username>.json` (respaldo transitorio).
pub fn json_path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.json"))
}

/// `users/<username>.prefs.toml`.
pub fn prefs_path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.prefs.toml"))
}

fn set_str(doc: &mut toml_edit::DocumentMut, key: &str, s: Option<&str>) {
    match s {
        Some(v) => doc[key] = value(v),
        None => {
            doc.remove(key);
        }
    }
}

impl UserData {
    /// Escritura atómica en `users/<usuario>.toml` conservando comentarios,
    /// orden y claves desconocidas; copia la versión anterior a `.bak`.
    pub fn write(&self) -> Result<()> {
        let path = path_for(&self.username);
        let mut doc = store::load_document(&path)?;
        doc["version"] = value(self.version as i64);
        doc["username"] = value(&self.username);
        doc["fullName"] = value(&self.full_name);
        doc["admin"] = value(self.admin);
        doc["template"] = value(&self.template);
        doc["personalConfig"] = value(self.personal_config);
        // La apariencia y accesibilidad viven en la semilla `.prefs.toml`, no en
        // la cuenta: se retiran del archivo de usuario al reescribirlo.
        for key in [ "colorScheme", "accentColor", "avatarColor", "avatarPath", "a11y" ] {
            doc.remove(key);
        }
        match &self.git {
            Some(git) => {
                let mut table = Table::new();
                table["name"] = value(&git.name);
                table["email"] = value(&git.email);
                doc["git"] = Item::Table(table);
            }
            None => {
                doc.remove("git");
            }
        }
        store::write_document_atomic(&path, &doc)
    }
}

fn a11y_table(a11y: &A11y) -> Table {
    let mut table = Table::new();
    table["largeText"] = value(a11y.large_text);
    table["highContrast"] = value(a11y.high_contrast);
    table["screenReader"] = value(a11y.screen_reader);
    table["reduceMotion"] = value(a11y.reduce_motion);
    table["screenKeyboard"] = value(a11y.screen_keyboard);
    table
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            version: 1,
            color_scheme: None,
            accent_color: None,
            avatar_color: None,
            avatar_path: None,
            a11y: None,
        }
    }
}

impl Prefs {
    /// Escribe la semilla en `users/<usuario>.prefs.toml`.
    pub fn write(&self, username: &str) -> Result<()> {
        let path = prefs_path_for(username);
        let mut doc = store::load_document(&path)?;
        doc["version"] = value(self.version as i64);
        set_str(&mut doc, "colorScheme", self.color_scheme.as_deref());
        set_str(&mut doc, "accentColor", self.accent_color.as_deref());
        set_str(&mut doc, "avatarColor", self.avatar_color.as_deref());
        set_str(&mut doc, "avatarPath", self.avatar_path.as_deref());
        match &self.a11y {
            Some(a11y) => doc["a11y"] = Item::Table(a11y_table(a11y)),
            None => {
                doc.remove("a11y");
            }
        }
        store::write_document_atomic(&path, &doc)
    }

    /// Lee la semilla si existe.
    pub fn read(username: &str) -> Result<Option<Self>> {
        let path = prefs_path_for(username);
        if !path.exists() {
            return Ok(None);
        }
        let doc = store::load_document(&path)?;
        let get = |key: &str| doc.get(key).and_then(|i| i.as_str()).map(str::to_string);
        let a11y = doc.get("a11y").and_then(|i| i.as_table()).map(|t| A11y {
            large_text: t.get("largeText").and_then(|i| i.as_bool()).unwrap_or(false),
            high_contrast: t.get("highContrast").and_then(|i| i.as_bool()).unwrap_or(false),
            screen_reader: t.get("screenReader").and_then(|i| i.as_bool()).unwrap_or(false),
            reduce_motion: t.get("reduceMotion").and_then(|i| i.as_bool()).unwrap_or(false),
            screen_keyboard: t.get("screenKeyboard").and_then(|i| i.as_bool()).unwrap_or(false),
        });
        Ok(Some(Self {
            version: doc.get("version").and_then(|i| i.as_integer()).unwrap_or(1) as u32,
            color_scheme: get("colorScheme"),
            accent_color: get("accentColor"),
            avatar_color: get("avatarColor"),
            avatar_path: get("avatarPath"),
            a11y,
        }))
    }
}

/// `${NIXDOTS_ROOT}/persist/nixdots/user.json`, el formato antiguo.
pub fn legacy_path() -> PathBuf {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    PathBuf::from(format!("{root}/persist/nixdots/user.json"))
}

/// Formato antiguo de `user.json` (un único usuario con los campos de sistema
/// mezclados). Solo se usa para migrarlo a `users/<usuario>.toml` + machine.toml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegacyUser {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub userdescription: Option<String>,
    #[serde(default)]
    pub pretty_host_name: Option<String>,
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
    #[serde(default)]
    pub default_locale: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub time_zone: Option<String>,
    #[serde(default)]
    pub time_zone_auto: Option<bool>,
    #[serde(default)]
    pub keyboard_layout: Option<String>,
    #[serde(default)]
    pub keyboard_variant: Option<String>,
    #[serde(default)]
    pub color_scheme: Option<String>,
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub avatar_color: Option<String>,
    #[serde(default)]
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub auto_login: Option<bool>,
    #[serde(default)]
    pub a11y: Option<A11y>,
}

impl LegacyUser {
    /// Lee el `user.json` antiguo, o `None` si no existe.
    pub fn read() -> Result<Option<Self>> {
        let path = legacy_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(
                serde_json::from_str(&text).wrap_err("user.json no tiene el formato esperado")?,
            )),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err).wrap_err_with(|| format!("no se pudo leer {}", path.display())),
        }
    }
}

/// Convierte `machine.json` y `users/*.json` a TOML sin borrar los originales
/// (quedan como respaldo). Devuelve cuántos archivos se escribieron.
pub fn migrate_to_toml() -> Result<usize> {
    let mut written = 0;
    if store::migrate_to_toml(&crate::machine::path())? {
        written += 1;
    }
    let dir = users_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(written);
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".json") || name.ends_with(".bak") {
            continue;
        }
        let toml = dir.join(name.replace(".json", ".toml"));
        if store::migrate_to_toml(&toml)? {
            written += 1;
        }
    }
    Ok(written)
}

/// Comprueba que `path` no existe todavía; error legible si sí.
pub fn ensure_absent(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(eyre!("{} ya existe", path.display()));
    }
    Ok(())
}
