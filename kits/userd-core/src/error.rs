use thiserror::Error;

use crate::ids::IdError;

/// Errores del núcleo.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error(transparent)]
    Id(#[from] IdError),
}
