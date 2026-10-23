use std::sync::Arc;

use userd_core::names::ROOT_PATH;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, interface};

use crate::backends;
use crate::config::Config;
use crate::jobs::Scheduler;

/// `io.github.HBlanqueto.Userd1.Manager` en la ruta raíz.
pub struct Manager {
    pub(crate) scheduler: Arc<Scheduler>,
    pub(crate) config: Arc<Config>,
    pub(crate) connection: Option<Connection>,
}

impl Manager {
    #[must_use]
    pub fn new(
        scheduler: Arc<Scheduler>,
        config: Arc<Config>,
        connection: Option<Connection>,
    ) -> Self {
        Self {
            scheduler,
            config,
            connection,
        }
    }
}

#[interface(name = "io.github.HBlanqueto.Userd1.Manager")]
impl Manager {
    fn list_jobs(&self) -> Vec<OwnedObjectPath> {
        self.scheduler
            .records()
            .into_iter()
            .filter_map(|record| {
                OwnedObjectPath::try_from(userd_core::names::job_path(record.id)).ok()
            })
            .collect()
    }

    async fn list_backends(&self) -> Vec<(String, String, bool)> {
        backends::probe_all(self.connection.as_ref(), &self.config)
            .await
            .into_iter()
            .map(|status| {
                let name = status.kind.as_str().to_owned();
                (name.clone(), name, status.availability.is_available())
            })
            .collect()
    }

    #[zbus(signal)]
    pub async fn config_rejected(
        emitter: &SignalEmitter<'_>,
        name: &str,
        reason: &str,
    ) -> zbus::Result<()>;
}

/// Registra un objeto por cada job persistido.
pub async fn register_jobs(
    connection: &Connection,
    scheduler: &Arc<Scheduler>,
) -> Result<(), crate::Error> {
    for record in scheduler.records() {
        let path = userd_core::names::job_path(record.id);
        connection
            .object_server()
            .at(path, super::Job::new(Arc::clone(scheduler), record.id))
            .await
            .map_err(crate::Error::Dbus)?;
    }
    Ok(())
}

pub(crate) async fn reject(connection: &Connection, name: &str, reason: &str) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(connection, ROOT_PATH)?;
    Manager::config_rejected(&emitter, name, reason).await
}
