//! Daemon de nixos-userd (F0: fundaciones).
//!
//! Expone `io.github.HBlanqueto.Userd1.Manager` y los objetos `Job`,
//! persiste los jobs en redb y mantiene el último KDL válido de cada usuario.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod backends;
pub mod bus;
pub mod config;
pub mod error;
pub mod jobs;
pub mod store;

use std::sync::Arc;

use tokio::signal::unix::{SignalKind, signal};
use userd_core::names::{BUS_NAME, ROOT_PATH};

pub use config::Config;
pub use error::Error;

use crate::bus::Manager;
use crate::jobs::Scheduler;
use crate::store::{StoreEvent, UserStore};

/// Arranca el daemon hasta que llegue SIGTERM o SIGINT.
pub async fn run(config: Config) -> Result<(), Error> {
    let config = Arc::new(config);

    let (store, mut events) = UserStore::open(config.users_dir())?;
    let store = Arc::new(store);
    store.start_watcher()?;

    let scheduler = Scheduler::open(&config)?;

    let connection = zbus::connection::Builder::system()?.build().await?;

    connection
        .object_server()
        .at(
            ROOT_PATH,
            Manager::new(
                Arc::clone(&scheduler),
                Arc::clone(&config),
                Some(connection.clone()),
            ),
        )
        .await?;
    bus::manager::register_jobs(&connection, &scheduler).await?;
    // Pedir el nombre después de registrar los objetos evita que una llamada
    // llegue antes de que existan las interfaces.
    connection.request_name(BUS_NAME).await?;

    let signal_connection = connection.clone();
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            let StoreEvent::ConfigRejected { name, reason } = event;
            tracing::warn!(name, reason, "configuration rejected");
            if let Err(error) = bus::manager::reject(&signal_connection, &name, &reason).await {
                tracing::warn!(%error, "could not emit ConfigRejected");
            }
        }
    });

    tracing::info!(bus = BUS_NAME, "nixos-userd started");

    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = terminate.recv() => {}
        result = tokio::signal::ctrl_c() => { result?; }
    }

    tracing::info!("nixos-userd stopping");
    Ok(())
}
