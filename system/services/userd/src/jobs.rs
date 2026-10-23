use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use tokio::sync::{Semaphore, mpsc};
use userd_core::ids::JobId;
use userd_core::job::{JobFailure, JobRecord, JobState};

use crate::Error;
use crate::config::Config;

const JOBS: TableDefinition<u64, &[u8]> = TableDefinition::new("jobs");

/// Journal de jobs sobre redb.
pub struct Journal {
    database: Database,
    path: std::path::PathBuf,
}

impl Journal {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let database = Database::create(path).map_err(|error| Error::Journal(error.to_string()))?;
        std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;

        let transaction = database
            .begin_write()
            .map_err(|error| Error::Journal(error.to_string()))?;
        {
            transaction
                .open_table(JOBS)
                .map_err(|error| Error::Journal(error.to_string()))?;
        }
        transaction
            .commit()
            .map_err(|error| Error::Journal(error.to_string()))?;

        Ok(Self {
            database,
            path: path.to_path_buf(),
        })
    }

    pub fn put(&self, record: &JobRecord) -> Result<(), Error> {
        let bytes =
            serde_json::to_vec(record).map_err(|error| Error::Journal(error.to_string()))?;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| Error::Journal(error.to_string()))?;
        {
            let mut table = transaction
                .open_table(JOBS)
                .map_err(|error| Error::Journal(error.to_string()))?;
            table
                .insert(record.id.get(), bytes.as_slice())
                .map_err(|error| Error::Journal(error.to_string()))?;
        }
        transaction
            .commit()
            .map_err(|error| Error::Journal(error.to_string()))
    }

    pub fn all(&self) -> Result<Vec<JobRecord>, Error> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| Error::Journal(error.to_string()))?;
        let table = transaction
            .open_table(JOBS)
            .map_err(|error| Error::Journal(error.to_string()))?;
        let mut records = Vec::new();
        for entry in table
            .iter()
            .map_err(|error| Error::Journal(error.to_string()))?
        {
            let (_, value) = entry.map_err(|error| Error::Journal(error.to_string()))?;
            let record = serde_json::from_slice(value.value())
                .map_err(|error| Error::Journal(error.to_string()))?;
            records.push(record);
        }
        Ok(records)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Ejecuta el trabajo de un job.
pub trait Handler: Send + Sync {
    fn run<'a>(
        &'a self,
        job: &'a JobRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>>;
}

/// Serializa los jobs por usuario y limita los builds concurrentes.
pub struct Scheduler {
    journal: Journal,
    jobs: RwLock<HashMap<JobId, JobRecord>>,
    actors: tokio::sync::Mutex<HashMap<String, mpsc::Sender<JobId>>>,
    semaphore: Arc<Semaphore>,
    handlers: RwLock<HashMap<userd_core::job::JobKind, Arc<dyn Handler>>>,
    next_id: AtomicU64,
}

impl Scheduler {
    /// Abre el journal y reanuda los jobs no terminales.
    pub fn open(config: &Config) -> Result<Arc<Self>, Error> {
        std::fs::create_dir_all(&config.state_dir)?;
        let journal = Journal::open(&config.journal_path())?;
        let mut records = journal.all()?;

        let mut max_id = 0;
        for record in &mut records {
            max_id = max_id.max(record.id.get());
            if record.resume_after_restart() {
                journal.put(record)?;
                tracing::info!(id = record.id.get(), "requeued job after restart");
            }
        }

        let scheduler = Arc::new(Self {
            journal,
            jobs: RwLock::new(records.into_iter().map(|job| (job.id, job)).collect()),
            actors: tokio::sync::Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(config.max_builds.max(1))),
            handlers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(max_id.saturating_add(1)),
        });

        Ok(scheduler)
    }

    /// Persiste antes de encolar: un job nunca existe solo en memoria.
    pub async fn enqueue(self: &Arc<Self>, record: JobRecord) -> Result<(), Error> {
        self.journal.put(&record)?;
        self.next_id
            .fetch_max(record.id.get().saturating_add(1), Ordering::Relaxed);
        if let Ok(mut jobs) = self.jobs.write() {
            jobs.insert(record.id, record.clone());
        }
        self.dispatch(record.id).await
    }

    pub fn register(&self, kind: userd_core::job::JobKind, handler: Arc<dyn Handler>) {
        if let Ok(mut handlers) = self.handlers.write() {
            handlers.insert(kind, handler);
        }
    }

    #[must_use]
    pub fn next_id(&self) -> JobId {
        JobId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    #[must_use]
    pub fn records(&self) -> Vec<JobRecord> {
        let Ok(jobs) = self.jobs.read() else {
            return Vec::new();
        };
        let mut records: Vec<_> = jobs.values().cloned().collect();
        records.sort_by_key(|record| record.id);
        records
    }

    #[must_use]
    pub fn record(&self, id: JobId) -> Option<JobRecord> {
        self.jobs.read().ok()?.get(&id).cloned()
    }

    /// Solo se cancelan jobs que aún no se están ejecutando.
    pub fn cancel(&self, id: JobId) -> Result<JobRecord, Error> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|_| Error::Config("job map poisoned".into()))?;
        let record = jobs.get_mut(&id).ok_or(Error::JobNotFound(id))?;
        match record.state {
            JobState::Queued | JobState::Blocked(_) => {
                record.set_state(JobState::Cancelled);
                self.journal.put(record)?;
                Ok(record.clone())
            }
            _ => Err(Error::JobNotCancelable(id)),
        }
    }

    fn set_state(&self, id: JobId, state: JobState) -> Result<JobRecord, Error> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|_| Error::Config("job map poisoned".into()))?;
        let record = jobs.get_mut(&id).ok_or(Error::JobNotFound(id))?;
        record.set_state(state);
        self.journal.put(record)?;
        Ok(record.clone())
    }

    fn handler(&self, kind: userd_core::job::JobKind) -> Option<Arc<dyn Handler>> {
        self.handlers.read().ok()?.get(&kind).cloned()
    }

    async fn dispatch(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let Some(record) = self.record(id) else {
            return Err(Error::JobNotFound(id));
        };
        let key = record
            .user
            .as_ref()
            .map_or_else(|| "_global".to_owned(), ToString::to_string);

        let mut actors = self.actors.lock().await;
        let sender = if let Some(sender) = actors.get(&key) {
            sender.clone()
        } else {
            let (sender, receiver) = mpsc::channel(64);
            actors.insert(key, sender.clone());
            tokio::spawn(actor(Arc::clone(self), receiver));
            sender
        };
        drop(actors);

        sender
            .send(id)
            .await
            .map_err(|_| Error::Config("job actor stopped".into()))
    }
}

async fn actor(scheduler: Arc<Scheduler>, mut receiver: mpsc::Receiver<JobId>) {
    while let Some(id) = receiver.recv().await {
        let Some(record) = scheduler.record(id) else {
            continue;
        };
        let Some(handler) = scheduler.handler(record.kind) else {
            tracing::debug!(
                id = id.get(),
                kind = record.kind.as_str(),
                "no handler registered; job stays queued"
            );
            continue;
        };

        let Ok(permit) = scheduler.semaphore.clone().acquire_owned().await else {
            break;
        };

        let running = match scheduler.set_state(id, JobState::Running { progress: None }) {
            Ok(record) => record,
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not start job");
                continue;
            }
        };

        let state = match handler.run(&running).await {
            Ok(()) => JobState::Succeeded,
            Err(failure) => JobState::Failed(failure),
        };

        if let Err(error) = scheduler.set_state(id, state) {
            tracing::warn!(%error, id = id.get(), "could not finish job");
        }
        drop(permit);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use userd_core::ids::Username;
    use userd_core::job::JobKind;

    fn temp_config(label: &str) -> Config {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Config {
            state_dir: std::env::temp_dir()
                .join(format!("userd-jobs-{label}-{}-{nanos}", std::process::id())),
            ..Config::default()
        }
    }

    fn record(id: u64) -> JobRecord {
        JobRecord::new(
            JobId::new(id),
            JobKind::ModifyUser,
            "/io/github/HBlanqueto/Userd1/User/60001",
            Some(Username::new("s4rch").expect("valid name")),
        )
    }

    #[test]
    fn running_jobs_requeue_after_reopen() {
        let config = temp_config("resume");
        std::fs::create_dir_all(&config.state_dir).expect("state dir");
        let journal = Journal::open(&config.journal_path()).expect("journal");
        let mut job = record(1);
        job.set_state(JobState::Running { progress: None });
        journal.put(&job).expect("persist");
        drop(journal);

        let scheduler = Scheduler::open(&config).expect("scheduler");
        let resumed = scheduler.record(JobId::new(1)).expect("job");
        assert_eq!(resumed.state, JobState::Queued);

        std::fs::remove_dir_all(&config.state_dir).ok();
    }

    #[tokio::test]
    async fn actor_runs_registered_handler() {
        struct Done;
        impl Handler for Done {
            fn run<'a>(
                &'a self,
                _job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }

        let config = temp_config("actor");
        let scheduler = Scheduler::open(&config).expect("scheduler");
        scheduler.register(JobKind::ModifyUser, Arc::new(Done));
        scheduler.enqueue(record(2)).await.expect("enqueue");

        for _ in 0..100 {
            if scheduler
                .record(JobId::new(2))
                .is_some_and(|job| matches!(job.state, JobState::Succeeded))
            {
                std::fs::remove_dir_all(&config.state_dir).ok();
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job did not finish");
    }

    #[tokio::test]
    async fn cancel_only_queued_or_blocked() {
        let config = temp_config("cancel");
        let scheduler = Scheduler::open(&config).expect("scheduler");
        scheduler.enqueue(record(3)).await.expect("enqueue");

        let cancelled = scheduler.cancel(JobId::new(3)).expect("cancel");
        assert_eq!(cancelled.state, JobState::Cancelled);
        assert!(matches!(
            scheduler.cancel(JobId::new(3)),
            Err(Error::JobNotCancelable(_))
        ));

        std::fs::remove_dir_all(&config.state_dir).ok();
    }

    #[tokio::test]
    async fn global_semaphore_limits_concurrency() {
        struct Track {
            current: std::sync::atomic::AtomicUsize,
            peak: std::sync::atomic::AtomicUsize,
        }
        impl Handler for Track {
            fn run<'a>(
                &'a self,
                _job: &'a JobRecord,
            ) -> Pin<Box<dyn Future<Output = Result<(), JobFailure>> + Send + 'a>> {
                Box::pin(async move {
                    let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;
                    self.peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    self.current.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
            }
        }

        let config = Config {
            max_builds: 1,
            ..temp_config("semaphore")
        };
        let scheduler = Scheduler::open(&config).expect("scheduler");
        let track = Arc::new(Track {
            current: std::sync::atomic::AtomicUsize::new(0),
            peak: std::sync::atomic::AtomicUsize::new(0),
        });
        scheduler.register(JobKind::ModifyUser, track.clone());

        for id in 10..13 {
            scheduler.enqueue(record(id)).await.expect("enqueue");
        }
        for _ in 0..100 {
            let done = scheduler
                .records()
                .iter()
                .all(|job| matches!(job.state, JobState::Succeeded));
            if done {
                assert_eq!(track.peak.load(Ordering::SeqCst), 1);
                std::fs::remove_dir_all(&config.state_dir).ok();
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("jobs did not finish");
    }
}
