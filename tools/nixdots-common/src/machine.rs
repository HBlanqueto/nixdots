//! Datos del sistema fuera del repo, en
//! `${NIXDOTS_ROOT}/persist/nixdots/machine.json`.
//!
//! Es el archivo del sistema (hostName, cifrado, Secure Boot, extras) más los
//! campos de sistema que elige el asistente (prettyHostName, defaultLocale,
//! region, timeZone, timeZoneAuto, keyboardLayout/Variant, autoLoginUser). El
//! instalador y el asistente escriben ambos; gana el último en escribir.
//!
//! Nunca se deserializa a un struct: se fusiona sobre un `serde_json::Value`
//! para conservar cualquier clave que no gestionemos (luksUuid, encrypt,
//! extras…).

use color_eyre::eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};

/// `${NIXDOTS_ROOT}/persist/nixdots/machine.json`.
pub fn path() -> PathBuf {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    PathBuf::from(format!("{root}/persist/nixdots/machine.json"))
}

/// Fusiona `fields` (cambiando solo esas claves) sobre el `machine.json` que
/// haya en [`path`].
pub fn merge_fields(fields: serde_json::Map<String, serde_json::Value>) -> Result<()> {
    merge_fields_at(&path(), fields)
}

/// Igual que [`merge_fields`] pero sobre una ruta concreta (el instalador
/// escribe en el destino, no en `${NIXDOTS_ROOT}`).
pub fn merge_fields_at(
    path: &Path,
    fields: serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    let mut value = match std::fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => serde_json::from_str::<serde_json::Value>(&text)
            .with_context(|| format!("{} no tiene el formato esperado", path.display()))?,
        _ => serde_json::Value::Object(serde_json::Map::new()),
    };

    let object = value
        .as_object_mut()
        .ok_or_else(|| eyre!("{} no es un objeto JSON", path.display()))?;
    for (key, field) in fields {
        object.insert(key, field);
    }

    crate::user_data::write_json_atomic(path, &value)
}
