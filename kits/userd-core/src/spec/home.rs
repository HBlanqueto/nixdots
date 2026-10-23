use std::collections::BTreeMap;

/// Opciones de home-manager pendientes de aplicar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HomeSpec {
    /// Ruta de opción (`programs.git.enable`) y su valor escalar.
    pub options: BTreeMap<String, serde_json::Value>,
}
