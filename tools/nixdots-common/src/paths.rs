//! Rutas de los datos externos, derivadas de `KEEL_ROOT`.
//!
//! `KEEL_ROOT` vacío o ausente significa `/persist` (el sistema instalado).
//! El instalador, en cambio, la pone al punto de montaje destino (`/mnt`). Un
//! único sitio para no repetir `format!` ni equivocarse con la raíz.

use std::path::PathBuf;

/// Valor de `KEEL_ROOT` (o `NIXDOTS_ROOT` como respaldo transitorio).
pub fn root() -> String {
    let keel = std::env::var("KEEL_ROOT").unwrap_or_default();
    if keel.is_empty() {
        std::env::var("NIXDOTS_ROOT").unwrap_or_default()
    } else {
        keel
    }
}

/// `${KEEL_ROOT}/persist/keel`.
pub fn persist() -> PathBuf {
    let root = root();
    let keel = PathBuf::from(format!("{root}/persist/keel"));
    let legacy = PathBuf::from(format!("{root}/persist/nixdots"));
    // shortcut: lee /persist/nixdots hasta que `steward migrate` mueva los datos.
    if !keel.exists() && legacy.exists() {
        legacy
    } else {
        keel
    }
}

/// `${KEEL_ROOT}/persist/secrets` (hashes de contraseña, etc.).
pub fn secrets() -> PathBuf {
    PathBuf::from(format!("{}/persist/secrets", root()))
}
