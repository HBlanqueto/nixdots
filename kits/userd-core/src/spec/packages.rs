use crate::backend::PackageBackendKind;

/// Referencia a un paquete instalado por un backend concreto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    /// Identificador en el backend (attr, app id o nombre del perfil).
    pub id: String,
    pub backend: PackageBackendKind,
    /// Atributo de nixpkgs, cuando el backend lo necesita.
    pub attr: Option<String>,
    /// Remoto de Flatpak, cuando el backend lo necesita.
    pub remote: Option<String>,
}

/// Paquetes declarados para el usuario.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackagesSpec {
    pub packages: Vec<PackageRef>,
}
