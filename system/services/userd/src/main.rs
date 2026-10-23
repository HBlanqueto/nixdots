use std::path::PathBuf;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;
use userd_core::ids::{JobId, Username};
use userd_core::job::{JobKind, JobRecord, JobState};
use userd_core::names::ROOT_PATH;
use userd_daemon::config::Config;
use userd_daemon::jobs::Journal;
use userd_daemon::{Error, run};

const DEFAULT_CONFIG: &str = "/etc/nixos-userd/config.json";

fn main() -> Result<(), Error> {
    init_tracing();

    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|argument| argument == "seed-job")
    {
        arguments.remove(0);
        return seed_job(&arguments);
    }

    let config = load_config(&arguments)?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run(config))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let result = match tracing_journald::layer() {
        Ok(journald) => tracing_subscriber::registry()
            .with(filter)
            .with(journald)
            .try_init(),
        Err(_) => tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init(),
    };
    if result.is_err() {
        eprintln!("could not install the tracing subscriber");
    }
}

fn load_config(arguments: &[String]) -> Result<Config, Error> {
    let mut path = PathBuf::from(DEFAULT_CONFIG);
    let mut state_dir = None;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => path = value(arguments, &mut index, "--config")?.into(),
            "--state-dir" => state_dir = Some(value(arguments, &mut index, "--state-dir")?.into()),
            other => return Err(Error::Config(format!("unknown argument `{other}`"))),
        }
        index += 1;
    }

    let mut config = Config::load(&path)?;
    if let Some(state_dir) = state_dir {
        config.state_dir = state_dir;
    }
    Ok(config)
}

fn value<'a>(arguments: &'a [String], index: &mut usize, flag: &str) -> Result<&'a str, Error> {
    *index += 1;
    arguments
        .get(*index)
        .map(String::as_str)
        .ok_or_else(|| Error::Config(format!("{flag} requires a value")))
}

/// Engancha un job de prueba en el journal. Es el mínimo necesario para el
/// nixosTest de F0; desaparece cuando `Manager.CreateUser` cree jobs de verdad.
fn seed_job(arguments: &[String]) -> Result<(), Error> {
    let mut path = PathBuf::from(DEFAULT_CONFIG);
    let mut state_dir = None;
    let mut user = None;
    let mut running = false;

    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => path = value(arguments, &mut index, "--config")?.into(),
            "--state-dir" => state_dir = Some(value(arguments, &mut index, "--state-dir")?.into()),
            "--user" => user = Some(value(arguments, &mut index, "--user")?.to_owned()),
            "--running" => running = true,
            other => return Err(Error::Config(format!("unknown argument `{other}`"))),
        }
        index += 1;
    }

    let mut config = Config::load(&path)?;
    if let Some(state_dir) = state_dir {
        config.state_dir = state_dir;
    }

    let user = user.ok_or_else(|| Error::Config("--user is required".into()))?;
    let user = Username::new(user).map_err(userd_core::Error::from)?;
    std::fs::create_dir_all(&config.state_dir)?;

    let journal = Journal::open(&config.journal_path())?;
    let id = journal
        .all()?
        .iter()
        .map(|record| record.id.get())
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    let mut record = JobRecord::new(
        JobId::new(id),
        JobKind::ModifyUser,
        format!("{ROOT_PATH}/User/{user}"),
        Some(user),
    );
    if running {
        record.set_state(JobState::Running { progress: None });
    }
    journal.put(&record)?;

    println!("{id}");
    Ok(())
}
