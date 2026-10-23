use std::path::{Path, PathBuf};

use keel_userd_core::backend::{IdentityBackendKind, PackageBackendKind};
use serde::Deserialize;

use crate::Error;

/// Configuración generada por el módulo NixOS en
/// `/etc/keel/userd.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub state_dir: PathBuf,
    pub userdb_dir: PathBuf,
    pub inputs_json: PathBuf,
    pub identity_priority: Vec<IdentityBackendKind>,
    pub package_priority: Vec<PackageBackendKind>,
    pub max_builds: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state_dir: PathBuf::from("/var/lib/keel-userd"),
            userdb_dir: PathBuf::from("/etc/userdb"),
            inputs_json: PathBuf::from("/etc/keel/userd-inputs.json"),
            identity_priority: vec![
                IdentityBackendKind::Homed,
                IdentityBackendKind::Userdb,
                IdentityBackendKind::Declared,
            ],
            package_priority: vec![
                PackageBackendKind::NixProfile,
                PackageBackendKind::HomeManager,
                PackageBackendKind::Flatpak,
            ],
            max_builds: 2,
        }
    }
}

impl Config {
    /// Carga el JSON del módulo. Si no existe, usa los valores por defecto
    /// (útil para desarrollo y para el test de la VM).
    pub fn load(path: &Path) -> Result<Self, Error> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| Error::Config(format!("{}: {error}", path.display()))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!(path = %path.display(), "config not found, using defaults");
                Ok(Self::default())
            }
            Err(error) => Err(error.into()),
        }
    }

    #[must_use]
    pub fn users_dir(&self) -> PathBuf {
        self.state_dir.join("users")
    }

    #[must_use]
    pub fn journal_path(&self) -> PathBuf {
        self.state_dir.join("state.redb")
    }
}
