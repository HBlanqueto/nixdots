//! Runs external commands and streams their output line by line.
//!
//! The installer never reimplements what these tools do; it only orchestrates
//! them. Every command is described by a [`Cmd`], which keeps a human-readable
//! [`Cmd::display`] string for the command panel and the install log.

use color_eyre::eyre::{Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// A command that produces no output for this long is assumed to be stuck and
/// is killed, so the TUI never freezes waiting on a step that went quiet.
const IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// One command to run.
#[derive(Debug, Clone)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    /// Full shell-style string shown in the UI.
    pub display: String,
}

impl Cmd {
    pub fn new(program: impl Into<String>) -> Self {
        let program = program.into();
        Self {
            display: program.clone(),
            program,
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        let arg = arg.into();
        self.display.push(' ');
        self.display.push_str(&arg);
        self.args.push(arg);
        self
    }

    /// Adds a path argument (converted lossily to a string).
    pub fn arg_path(self, path: impl AsRef<std::path::Path>) -> Self {
        self.arg(path.as_ref().to_string_lossy().to_string())
    }

    /// Adds several arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for arg in args {
            self = self.arg(arg);
        }
        self
    }

    /// Overrides the displayed string (useful when the real invocation is more
    /// verbose than what a human wants to read).
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
}

/// A streamed piece of a command's output.
#[derive(Debug, Clone)]
pub enum Stream {
    Stdout(String),
    Stderr(String),
}

/// Experimental features every Nix invocation needs. The live ISO may not have
/// them in `nix.conf`, and `nix eval`/`nix build`/`nix flake` refuse to run
/// without them, so every child gets them through `NIX_CONFIG`. An existing
/// value is kept and extended with a newline instead of replaced.
pub fn nix_config() -> String {
    let ours = "experimental-features = nix-command flakes";
    match std::env::var("NIX_CONFIG") {
        Ok(existing) if !existing.trim().is_empty() => format!("{existing}\n{ours}"),
        _ => ours.to_string(),
    }
}

/// Runs `cmd` to completion, invoking `sink` for every output line, and returns
/// the exit code (`-1` when it was killed by a signal, `-2` when it was killed
/// because it went silent). No command ever inherits the installer's stdin, so
/// nothing can stop to ask for input and hang the TUI.
pub fn run(cmd: &Cmd, sink: &mut dyn FnMut(Stream)) -> i32 {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .env("NIX_CONFIG", nix_config())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn_and_stream(command, &cmd.program, None, sink)
}

/// Like [`run`], but feeds `input` on the child's stdin. Used for secrets
/// (LUKS passphrases, recovery keys) that must never appear as an argument, in
/// the display string or in the log. The data is sent from a helper thread and
/// the pipe is closed right after, so the child sees EOF.
pub fn run_with_stdin(cmd: &Cmd, input: &[u8], sink: &mut dyn FnMut(Stream)) -> i32 {
    let mut command = Command::new(&cmd.program);
    command
        .args(&cmd.args)
        .env("NIX_CONFIG", nix_config())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn_and_stream(command, &cmd.program, Some(input), sink)
}

fn spawn_and_stream(
    mut command: Command,
    program: &str,
    input: Option<&[u8]>,
    sink: &mut dyn FnMut(Stream),
) -> i32 {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            sink(Stream::Stderr(format!(
                "no se pudo ejecutar {program}: {err}"
            )));
            return 127;
        }
    };

    if let Some(input) = input {
        if let Some(mut stdin) = child.stdin.take() {
            let data = input.to_vec();
            thread::spawn(move || {
                let _ = stdin.write_all(&data);
                let _ = stdin.flush();
            });
        }
    }

    let (tx, rx) = mpsc::channel::<(bool, String)>();
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || read_lines(stdout, false, &tx)));
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = tx.clone();
        readers.push(thread::spawn(move || read_lines(stderr, true, &tx)));
    }
    drop(tx);

    // Forward every line as it arrives, aborting if the command goes silent.
    let mut timed_out = false;
    loop {
        match rx.recv_timeout(IDLE_TIMEOUT) {
            Ok((is_stderr, line)) => {
                if is_stderr {
                    sink(Stream::Stderr(line));
                } else {
                    sink(Stream::Stdout(line));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                timed_out = true;
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if timed_out {
        let _ = child.kill();
        sink(Stream::Stderr(format!(
            "{} no produjo salida en {} s; se cancela el comando",
            program,
            IDLE_TIMEOUT.as_secs()
        )));
    }

    for reader in readers {
        let _ = reader.join();
    }

    if timed_out {
        let _ = child.wait();
        return -2;
    }

    match child.wait() {
        Ok(status) => status.code().unwrap_or(-1),
        Err(err) => {
            sink(Stream::Stderr(format!(
                "error esperando a {program}: {err}"
            )));
            -1
        }
    }
}

fn read_lines<R: Read>(reader: R, is_stderr: bool, tx: &mpsc::Sender<(bool, String)>) {
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        if tx.send((is_stderr, line)).is_err() {
            break;
        }
    }
}

/// Runs `cmd` and captures its full output (for short, non-interactive probes).
pub fn capture(cmd: &Cmd) -> Result<Output> {
    Command::new(&cmd.program)
        .args(&cmd.args)
        .env("NIX_CONFIG", nix_config())
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("no se pudo ejecutar {}", cmd.program))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_shell_like() {
        let cmd = Cmd::new("disko")
            .arg("--mode")
            .arg("destroy,format,mount")
            .arg("disko.nix");
        assert_eq!(cmd.display, "disko --mode destroy,format,mount disko.nix");
        assert_eq!(cmd.args.len(), 3);
    }

    #[test]
    fn capture_reads_stdout() {
        let out = capture(&Cmd::new("sh").arg("-c").arg("echo hola")).unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hola");
    }

    #[test]
    fn run_streams_and_returns_code() {
        let mut lines = Vec::new();
        let code = run(
            &Cmd::new("sh")
                .arg("-c")
                .arg("echo uno; echo dos >&2; exit 3"),
            &mut |out| match out {
                Stream::Stdout(l) => lines.push(format!("out:{l}")),
                Stream::Stderr(l) => lines.push(format!("err:{l}")),
            },
        );
        assert_eq!(code, 3);
        assert!(lines.iter().any(|l| l == "out:uno"));
        assert!(lines.iter().any(|l| l == "err:dos"));
    }
}
