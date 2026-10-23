use std::collections::HashMap;
use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use tokio::sync::{Semaphore, mpsc};
use userd_core::ids::JobId;
use userd_core::job::{JobFailure, JobKind, JobRecord, JobState};
use zbus::Connection;

use crate::Error;
use crate::bus::Job;
use crate::config::Config;

const JOBS: TableDefinition<u64, &[u8]> = TableDefinition::new("jobs");

/// Journal de jobs sobre redb.
pub struct Journal {
    database: Database,
}

impl Journal {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let database = Database::create(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

        let transaction = database.begin_write()?;
        {
            transaction.open_table(JOBS)?;
        }
        transaction.commit()?;

        Ok(Self { database })
    }

    pub fn put(&self, record: &JobRecord) -> Result<(), Error> {
        let bytes = serde_json::to_vec(record)?;
        let transaction = self.database.begin_write()?;
        {
            let mut table = transaction.open_table(JOBS)?;
            table.insert(record.id.get(), bytes.as_slice())?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn all(&self) -> Result<Vec<JobRecord>, Error> {
        let transaction = self.database.begin_read()?;
        let table = transaction.open_table(JOBS)?;
        let mut records = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            records.push(serde_json::from_slice(value.value())?);
        }
        Ok(records)
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
    handlers: RwLock<HashMap<JobKind, Arc<dyn Handler>>>,
    next_id: AtomicU64,
    connection: RwLock<Option<Connection>>,
}

impl Scheduler {
    /// Abre el journal, reanuda los jobs no terminales y despacha los `Queued`.
    pub async fn open(config: &Config) -> Result<Arc<Self>, Error> {
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

        let queued: Vec<JobId> = records
            .iter()
            .filter(|record| matches!(record.state, JobState::Queued))
            .map(|record| record.id)
            .collect();

        let scheduler = Arc::new(Self {
            journal,
            jobs: RwLock::new(records.into_iter().map(|job| (job.id, job)).collect()),
            actors: tokio::sync::Mutex::new(HashMap::new()),
            semaphore: Arc::new(Semaphore::new(config.max_builds.max(1))),
            handlers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(max_id.saturating_add(1)),
            connection: RwLock::new(None),
        });

        for id in queued {
            scheduler.dispatch(id).await?;
        }

        Ok(scheduler)
    }

    /// Conecta el scheduler con el bus para registrar los objetos `Job` nuevos.
    pub fn set_connection(&self, connection: Connection) -> Result<(), Error> {
        *write_lock(&self.connection, "connection")? = Some(connection);
        Ok(())
    }

    /// Persiste antes de encolar y registra el objeto D-Bus en el mismo momento.
    pub async fn enqueue(self: &Arc<Self>, record: JobRecord) -> Result<(), Error> {
        self.journal.put(&record)?;
        self.next_id
            .fetch_max(record.id.get().saturating_add(1), Ordering::Relaxed);
        write_lock(&self.jobs, "jobs")?.insert(record.id, record.clone());
        self.register_object(record.id).await?;
        self.dispatch(record.id).await
    }

    /// Registra un handler y reintenta los jobs `Queued` de su mismo tipo.
    pub async fn register(
        self: &Arc<Self>,
        kind: JobKind,
        handler: Arc<dyn Handler>,
    ) -> Result<(), Error> {
        write_lock(&self.handlers, "handlers")?.insert(kind, handler);

        let queued: Vec<JobId> = self
            .records()?
            .into_iter()
            .filter(|record| record.kind == kind && matches!(record.state, JobState::Queued))
            .map(|record| record.id)
            .collect();

        for id in queued {
            self.dispatch(id).await?;
        }
        Ok(())
    }

    #[must_use]
    pub fn next_id(&self) -> JobId {
        JobId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    pub fn records(&self) -> Result<Vec<JobRecord>, Error> {
        let jobs = read_lock(&self.jobs, "jobs")?;
        let mut records: Vec<_> = jobs.values().cloned().collect();
        records.sort_by_key(|record| record.id);
        Ok(records)
    }

    pub fn record(&self, id: JobId) -> Result<Option<JobRecord>, Error> {
        Ok(read_lock(&self.jobs, "jobs")?.get(&id).cloned())
    }

    /// Solo se cancelan jobs que aún no se están ejecutando.
    pub fn cancel(&self, id: JobId) -> Result<JobRecord, Error> {
        let mut record = self.record(id)?.ok_or(Error::JobNotFound(id))?;
        match record.state {
            JobState::Queued | JobState::Blocked(_) => record.set_state(JobState::Cancelled),
            _ => return Err(Error::JobNotCancelable(id)),
        }
        self.journal.put(&record)?;
        write_lock(&self.jobs, "jobs")?.insert(id, record.clone());
        Ok(record)
    }

    fn set_state(&self, id: JobId, state: JobState) -> Result<JobRecord, Error> {
        let mut record = self.record(id)?.ok_or(Error::JobNotFound(id))?;
        record.set_state(state);
        self.journal.put(&record)?;
        write_lock(&self.jobs, "jobs")?.insert(id, record.clone());
        Ok(record)
    }

    fn handler(&self, kind: JobKind) -> Result<Option<Arc<dyn Handler>>, Error> {
        Ok(read_lock(&self.handlers, "handlers")?.get(&kind).cloned())
    }

    async fn register_object(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let connection = read_lock(&self.connection, "connection")?.clone();
        let Some(connection) = connection else {
            return Ok(());
        };
        connection
            .object_server()
            .at(
                userd_core::names::job_path(id),
                Job::new(Arc::clone(self), id),
            )
            .await?;
        Ok(())
    }

    async fn dispatch(self: &Arc<Self>, id: JobId) -> Result<(), Error> {
        let Some(record) = self.record(id)? else {
            return Err(Error::JobNotFound(id));
        };
        if !matches!(record.state, JobState::Queued) {
            return Ok(());
        }

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
        let record = match scheduler.record(id) {
            Ok(Some(record)) => record,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not read job");
                continue;
            }
        };
        if !matches!(record.state, JobState::Queued) {
            continue;
        }

        let handler = match scheduler.handler(record.kind) {
            Ok(Some(handler)) => handler,
            Ok(None) => {
                tracing::debug!(
                    id = id.get(),
                    kind = record.kind.as_str(),
                    "no handler registered; job stays queued"
                );
                continue;
            }
            Err(error) => {
                tracing::warn!(%error, id = id.get(), "could not read handlers");
                continue;
            }
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

fn read_lock<'a, T>(lock: &'a RwLock<T>, name: &str) -> Result<RwLockReadGuard<'a, T>, Error> {
    lock.read()
        .map_err(|_| Error::LockPoisoned(name.to_owned()))
}

fn write_lock<'a, T>(lock: &'a RwLock<T>, name: &str) -> Result<RwLockWriteGuard<'a, T>, Error> {
    lock.write()
        .map_err(|_| Error::LockPoisoned(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use userd_core::ids::Username;

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
        record_for(id, "s4rch")
    }

    fn record_for(id: u64, name: &str) -> JobRecord {
        JobRecord::new(
            JobId::new(id),
            JobKind::ModifyUser,
            "/io/github/HBlanqueto/Userd1/User/60001",
            Some(Username::new(name).expect("valid name")),
        )
    }

    #[tokio::test]
    async fn running_jobs_requeue_after_reopen() {
        let config = temp_config("resume");
        std::fs::create_dir_all(&config.state_dir).expect("state dir");
        let journal = Journal::open(&config.journal_path()).expect("journal");
        let mut job = record(1);
        job.set_state(JobState::Running { progress: None });
        journal.put(&job).expect("persist");
        drop(journal);

        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let resumed = scheduler.record(JobId::new(1)).expect("lock").expect("job");
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
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        scheduler
            .register(JobKind::ModifyUser, Arc::new(Done))
            .await
            .expect("register");
        scheduler.enqueue(record(2)).await.expect("enqueue");

        for _ in 0..100 {
            let finished = scheduler
                .record(JobId::new(2))
                .expect("lock")
                .is_some_and(|job| matches!(job.state, JobState::Succeeded));
            if finished {
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
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
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
        let scheduler = Scheduler::open(&config).await.expect("scheduler");
        let track = Arc::new(Track {
            current: std::sync::atomic::AtomicUsize::new(0),
            peak: std::sync::atomic::AtomicUsize::new(0),
        });
        scheduler
            .register(JobKind::ModifyUser, track.clone())
            .await
            .expect("register");

        for (id, name) in [(10, "u10"), (11, "u11"), (12, "u12")] {
            scheduler
                .enqueue(record_for(id, name))
                .await
                .expect("enqueue");
        }
        for _ in 0..100 {
            let done = scheduler
                .records()
                .expect("lock")
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
