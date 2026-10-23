pub mod accounts;
pub mod home;
pub mod identity;
pub mod packages;

use crate::ids::Username;

pub use accounts::AccountsSpec;
pub use home::HomeSpec;
pub use identity::IdentitySpec;
pub use packages::{PackageRef, PackagesSpec};

/// Versión del esquema KDL que entiende este binario.
pub const SCHEMA_VERSION: u32 = 1;

/// Configuración completa de un usuario.
#[derive(Debug, Clone, PartialEq)]
pub struct UserSpec {
    /// Versión del esquema; obligatoria en el KDL.
    pub version: u32,
    pub name: Username,
    pub identity: IdentitySpec,
    pub accounts: AccountsSpec,
    pub home: HomeSpec,
    pub packages: PackagesSpec,
}

impl UserSpec {
    #[must_use]
    pub fn new(name: Username) -> Self {
        Self {
            version: SCHEMA_VERSION,
            name,
            identity: IdentitySpec::default(),
            accounts: AccountsSpec::default(),
            home: HomeSpec::default(),
            packages: PackagesSpec::default(),
        }
    }
}
