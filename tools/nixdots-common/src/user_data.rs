//! Datos de cuentas fuera del repo, un archivo por usuario en
//! `${NIXDOTS_ROOT}/persist/nixdots/users/<usuario>.json`.
//!
//! El flake los lee y crea con ellos `users.users.<nombre>` y su home-manager;
//! no se versionan ni se pierden al reconstruir. `hashedPasswordFile` no se
//! guarda: se deriva del usuario. El repo vive en `/persist/nixdots/repo`,
//! nunca dentro de estos archivos.
//!
//! Los datos del sistema (locale, zona, teclado, autologin…) NO van aquí: van
//! en `machine.json` (ver [`crate::machine`]).

use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// Lo que se escribe en `users/<usuario>.json` (camelCase, como lo lee el flake).
/// Los campos opcionales se omiten si son `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
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
    /// Acento elegido (`"neptunia"` o un id de GNOME); `"neptunia"` no se
    /// escribe en dconf.
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

/// `${NIXDOTS_ROOT}/persist/nixdots/users`. Construido como cadena: sin
/// `NIXDOTS_ROOT` la ruta debe quedar absoluta (`/persist/...`), no relativa.
pub fn users_dir() -> PathBuf {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    PathBuf::from(format!("{root}/persist/nixdots/users"))
}

/// `users/<username>.json`.
pub fn path_for(username: &str) -> PathBuf {
    users_dir().join(format!("{username}.json"))
}

impl UserData {
    /// Escritura atómica (temporal + rename), modo 0644, y copia de la versión
    /// anterior a `users/<username>.json.bak`.
    pub fn write(&self) -> Result<()> {
        write_json_atomic(&path_for(&self.username), &serde_json::to_value(self)?)
    }
}

/// Escribe un JSON de forma atómica (temporal + rename), modo 0644, y copia la
/// versión anterior a `<archivo>.bak`. Nunca escribe sobre el original en sitio.
pub fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<()> {
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| eyre!("{} no tiene directorio padre", path.display()))?,
    )?;
    if path.exists() {
        let bak = path.with_extension("json.bak");
        std::fs::copy(path, &bak)?;
        set_mode(&bak, 0o644)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(value)?))?;
    set_mode(&tmp, 0o644)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}
