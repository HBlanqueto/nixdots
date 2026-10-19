//! Datos del sistema fuera del repo, en
//! `${NIXDOTS_ROOT}/persist/nixdots/machine.toml` (respaldo: `machine.json`).
//!
//! Es el archivo del sistema (hostName, cifrado, Secure Boot, extras) más los
//! campos de sistema que elige el asistente (prettyHostName, defaultLocale,
//! region, timeZone, timeZoneAuto, keyboardLayout/Variant, autoLoginUser). El
//! instalador y el asistente escriben ambos; gana el último en escribir.
//!
//! Nunca se deserializa a un struct: se fusiona sobre el documento TOML para
//! conservar comentarios, orden y cualquier clave que no gestionemos.

use crate::store;
use color_eyre::eyre::Result;
use std::path::{Path, PathBuf};

/// `${NIXDOTS_ROOT}/persist/nixdots/machine.toml`.
pub fn path() -> PathBuf {
    crate::paths::persist().join("machine.toml")
}

/// Fusiona `fields` (cambiando solo esas claves) sobre el `machine.toml` que
/// haya en [`path`]. Un valor `null` elimina la clave.
pub fn merge_fields(fields: serde_json::Map<String, serde_json::Value>) -> Result<()> {
    merge_fields_at(&path(), fields)
}

/// Igual que [`merge_fields`] pero sobre una ruta concreta (el instalador
/// escribe en el destino, no en `${NIXDOTS_ROOT}`).
pub fn merge_fields_at(
    path: &Path,
    fields: serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    let mut doc = store::load_document(path)?;
    for (key, field) in fields {
        match store::item_from_json(&field) {
            Some(item) => doc[key.as_str()] = item,
            None => {
                doc.remove(&key);
            }
        }
    }
    store::write_document_atomic(path, &doc)
}
