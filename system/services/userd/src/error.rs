use thiserror::Error;
use userd_core::ids::JobId;

/// Errores internos del daemon.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] userd_core::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal error: {0}")]
    Journal(String),
    #[error("D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("job {0} not found")]
    JobNotFound(JobId),
    #[error("job {0} cannot be cancelled in its current state")]
    JobNotCancelable(JobId),
    #[error("config error: {0}")]
    Config(String),
}
