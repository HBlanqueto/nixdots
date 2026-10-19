//! Rutas de los datos externos, derivadas de `NIXDOTS_ROOT`.
//!
//! `NIXDOTS_ROOT` vacío o ausente significa `/persist` (el sistema instalado).
//! El instalador, en cambio, la pone al punto de montaje destino (`/mnt`). Un
//! único sitio para no repetir `format!` ni equivocarse con la raíz.

use std::path::PathBuf;

/// Valor de `NIXDOTS_ROOT` ("" si no está definida).
pub fn root() -> String {
    std::env::var("NIXDOTS_ROOT").unwrap_or_default()
}

/// `${NIXDOTS_ROOT}/persist/nixdots`.
pub fn persist() -> PathBuf {
    PathBuf::from(format!("{}/persist/nixdots", root()))
}

/// `${NIXDOTS_ROOT}/persist/secrets` (hashes de contraseña, etc.).
pub fn secrets() -> PathBuf {
    PathBuf::from(format!("{}/persist/secrets", root()))
}
