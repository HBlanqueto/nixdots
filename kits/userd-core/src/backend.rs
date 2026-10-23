use std::fmt;
use std::future::Future;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::Error;

/// Backend de identidad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityBackendKind {
    Homed,
    Userdb,
    Declared,
}

/// Backend de paquetes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageBackendKind {
    NixProfile,
    HomeManager,
    Flatpak,
}

/// Cualquiera de los backends del daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    Identity(IdentityBackendKind),
    Package(PackageBackendKind),
}

impl IdentityBackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Homed => "homed",
            Self::Userdb => "userdb",
            Self::Declared => "declared",
        }
    }
}

impl PackageBackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NixProfile => "nix-profile",
            Self::HomeManager => "home-manager",
            Self::Flatpak => "flatpak",
        }
    }
}

impl BackendKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity(kind) => kind.as_str(),
            Self::Package(kind) => kind.as_str(),
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IdentityBackendKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "homed" => Ok(Self::Homed),
            "userdb" => Ok(Self::Userdb),
            "declared" => Ok(Self::Declared),
            other => Err(Error::InvalidConfig(format!(
                "unknown identity backend `{other}`"
            ))),
        }
    }
}

impl FromStr for PackageBackendKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "nix-profile" => Ok(Self::NixProfile),
            "home-manager" => Ok(Self::HomeManager),
            "flatpak" => Ok(Self::Flatpak),
            other => Err(Error::InvalidConfig(format!(
                "unknown package backend `{other}`"
            ))),
        }
    }
}

/// Motivo por el que un backend no está disponible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailableReason {
    NotInstalled,
    NotRunning,
    NoSession,
    NoPermission,
    Io(String),
}

impl fmt::Display for UnavailableReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => f.write_str("not installed"),
            Self::NotRunning => f.write_str("not running"),
            Self::NoSession => f.write_str("no user session"),
            Self::NoPermission => f.write_str("no permission"),
            Self::Io(message) => write!(f, "I/O error: {message}"),
        }
    }
}

/// Disponibilidad de un backend en este momento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Unavailable(UnavailableReason),
}

impl Availability {
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Interfaz mínima que implementa cada backend.
pub trait Backend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn probe(&self) -> impl Future<Output = Availability> + Send;
}

/// Devuelve el primer backend disponible según la prioridad configurada.
///
/// No inventa un orden si la lista viene vacía: sin prioridad no hay elección.
pub async fn first_available<B: Backend>(
    backends: &[B],
    priority: &[BackendKind],
) -> Option<BackendKind> {
    for wanted in priority {
        let Some(backend) = backends.iter().find(|backend| backend.kind() == *wanted) else {
            continue;
        };
        if backend.probe().await.is_available() {
            return Some(*wanted);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeBackend {
        kind: BackendKind,
        availability: Availability,
    }

    impl Backend for FakeBackend {
        fn kind(&self) -> BackendKind {
            self.kind
        }

        fn probe(&self) -> impl Future<Output = Availability> + Send {
            std::future::ready(self.availability.clone())
        }
    }

    fn fake(kind: BackendKind, available: bool) -> FakeBackend {
        FakeBackend {
            kind,
            availability: if available {
                Availability::Available
            } else {
                Availability::Unavailable(UnavailableReason::NotRunning)
            },
        }
    }

    #[tokio::test]
    async fn resolves_by_priority() {
        let backends = [
            fake(BackendKind::Identity(IdentityBackendKind::Homed), false),
            fake(BackendKind::Identity(IdentityBackendKind::Userdb), true),
            fake(BackendKind::Identity(IdentityBackendKind::Declared), true),
        ];
        let priority = [
            BackendKind::Identity(IdentityBackendKind::Homed),
            BackendKind::Identity(IdentityBackendKind::Userdb),
            BackendKind::Identity(IdentityBackendKind::Declared),
        ];
        assert_eq!(
            first_available(&backends, &priority).await,
            Some(BackendKind::Identity(IdentityBackendKind::Userdb))
        );
    }

    #[tokio::test]
    async fn ignores_unknown_and_unavailable() {
        let backends = [fake(
            BackendKind::Package(PackageBackendKind::Flatpak),
            false,
        )];
        let priority = [
            BackendKind::Package(PackageBackendKind::NixProfile),
            BackendKind::Package(PackageBackendKind::Flatpak),
        ];
        assert_eq!(first_available(&backends, &priority).await, None);
        assert_eq!(first_available(&backends, &[]).await, None);
    }
}
