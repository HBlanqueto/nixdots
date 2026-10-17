//! Datos del usuario fuera del repo, en `user.json`.
//!
//! El flake los lee desde `${NIXDOTS_ROOT}/persist/nixdots/user.json` y los
//! mezcla con `settings.nix` (mismas claves), así que no se versionan ni se
//! pierden al reconstruir. `hashedPasswordFile` no se guarda: se deriva del
//! usuario. El repo vive en `/persist/nixdots/repo`, nunca dentro de user.json.

use color_eyre::eyre::{Context, Result};
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

/// Lo que se escribe en `user.json` (camelCase, como lo lee el flake). Los
/// campos opcionales se omiten si son `None`, para que el `//` del flake deje
/// el valor de `settings.nix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserData {
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userdescription: Option<String>,
    /// Nombre visible estilo macOS («ThinkPad L14 de Humberto»). Sustituye, vía
    /// el `//` de lib/data.nix, al que escribió el instalador en settings.nix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pretty_host_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_locale: Option<String>,
    /// Locale de región y formatos (p. ej. `es_MX.UTF-8`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    /// Zona horaria automática (geoclue); si es `true`, `timeZone` es respaldo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone_auto: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard_variant: Option<String>,
    /// `"light"` o `"dark"`: preferencia de tema elegida en el asistente.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    /// Acento elegido (`"neptunia"` o un id de GNOME); `"neptunia"` no se
    /// escribe en dconf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// Entrar directamente al escritorio sin pasar por GDM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_login: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a11y: Option<A11y>,
}

/// `${NIXDOTS_ROOT}/persist/nixdots/user.json`. Construido como cadena: sin
/// `NIXDOTS_ROOT` la ruta debe quedar absoluta (`/persist/...`), no relativa.
pub fn path() -> PathBuf {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    PathBuf::from(format!("{root}/persist/nixdots/user.json"))
}

impl UserData {
    /// Lee el archivo, o `None` si no existe.
    pub fn read() -> Result<Option<Self>> {
        let path = path();
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Some(
                serde_json::from_str(&text).wrap_err("user.json no tiene el formato esperado")?,
            )),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err).wrap_err_with(|| format!("no se pudo leer {}", path.display())),
        }
    }

    /// Escritura atómica (temporal + rename), modo 0644, y copia de la versión
    /// anterior a `user.json.bak`.
    pub fn write(&self) -> Result<()> {
        let path = path();
        std::fs::create_dir_all(path.parent().expect("user.json has a parent"))?;
        if path.exists() {
            std::fs::copy(&path, path.with_extension("json.bak"))?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(self)?))?;
        set_mode(&tmp, 0o644)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}
