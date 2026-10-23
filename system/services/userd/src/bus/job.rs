use std::sync::Arc;

use userd_core::ids::JobId;
use userd_core::job::JobState;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::bus::DbusError;
use crate::jobs::Scheduler;

/// `io.github.HBlanqueto.Userd1.Job` en `/…/Job/<id>`.
pub struct Job {
    scheduler: Arc<Scheduler>,
    id: JobId,
}

impl Job {
    #[must_use]
    pub fn new(scheduler: Arc<Scheduler>, id: JobId) -> Self {
        Self { scheduler, id }
    }

    fn record(&self) -> Option<userd_core::job::JobRecord> {
        self.scheduler.record(self.id)
    }
}

#[interface(name = "io.github.HBlanqueto.Userd1.Job")]
impl Job {
    #[zbus(property)]
    fn kind(&self) -> String {
        self.record()
            .map_or_else(String::new, |record| record.kind.as_str().to_owned())
    }

    #[zbus(property)]
    fn target(&self) -> String {
        self.record()
            .map_or_else(String::new, |record| record.target)
    }

    #[zbus(property)]
    fn state(&self) -> String {
        self.record()
            .map_or_else(String::new, |record| record.state.as_str().to_owned())
    }

    #[zbus(property)]
    fn block_reason(&self) -> String {
        self.record()
            .and_then(|record| {
                record
                    .state
                    .block_reason()
                    .map(|reason| reason.as_str().to_owned())
            })
            .unwrap_or_default()
    }

    #[zbus(property)]
    fn progress(&self) -> u32 {
        self.record()
            .and_then(|record| record.state.progress())
            .map_or(u32::MAX, |percent| u32::from(percent.get()))
    }

    #[zbus(property)]
    fn error(&self) -> String {
        self.record()
            .and_then(|record| record.state.error_message().map(ToOwned::to_owned))
            .unwrap_or_default()
    }

    async fn cancel(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), DbusError> {
        let record = self
            .scheduler
            .cancel(self.id)
            .map_err(|error| match error {
                crate::Error::JobNotFound(_) => DbusError::NotFound(self.id.to_string()),
                crate::Error::JobNotCancelable(_) => {
                    DbusError::Conflict("job is already running or finished".into())
                }
                other => DbusError::Failed(other.to_string()),
            })?;

        Self::finished(&emitter, record.state.as_str())
            .await
            .map_err(DbusError::Zbus)?;
        Ok(())
    }

    #[zbus(signal)]
    pub async fn finished(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;
}

/// Estado de un job sin el prefijo de variante, útil para señales.
#[must_use]
pub fn state_name(state: &JobState) -> &'static str {
    state.as_str()
}
