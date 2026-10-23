//! Comprueba que un job reanudado se ejecuta cuando llega su handler.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use keel_userd::config::Config;
use keel_userd::jobs::{Handler, Journal, Scheduler};
use keel_userd_core::ids::{JobId, Username};
use keel_userd_core::job::{JobFailure, JobKind, JobRecord, JobState};

struct Flag(Arc<AtomicBool>);

impl Handler for Flag {
    fn run<'a>(
        &'a self,
        _job: &'a JobRecord,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), JobFailure>> + Send + 'a>>
    {
        let flag = Arc::clone(&self.0);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    }
}

#[tokio::test]
async fn resumed_job_runs_after_handler_registration() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let config = Config {
        state_dir: std::env::temp_dir()
            .join(format!("userd-resume-{}-{nanos}", std::process::id())),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.state_dir).expect("state dir");

    let journal = Journal::open(&config.journal_path()).expect("journal");
    let mut job = JobRecord::new(
        JobId::new(1),
        JobKind::ModifyUser,
        "/io/github/HBlanqueto/Keel/Userd1/User/60001",
        Some(Username::new("s4rch").expect("valid name")),
    );
    job.set_state(JobState::Running { progress: None });
    journal.put(&job).expect("persist");
    drop(journal);

    let scheduler = Scheduler::open(&config).await.expect("scheduler");
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        scheduler
            .record(JobId::new(1))
            .expect("lock")
            .expect("job")
            .state,
        JobState::Queued
    );

    let executed = Arc::new(AtomicBool::new(false));
    scheduler
        .register(JobKind::ModifyUser, Arc::new(Flag(Arc::clone(&executed))))
        .await
        .expect("register");

    for _ in 0..100 {
        let finished = scheduler
            .record(JobId::new(1))
            .expect("lock")
            .is_some_and(|job| matches!(job.state, JobState::Succeeded));
        if finished {
            assert!(executed.load(Ordering::SeqCst));
            std::fs::remove_dir_all(&config.state_dir).ok();
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("resumed job did not run");
}
