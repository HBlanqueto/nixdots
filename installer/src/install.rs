//! The actual installation: a sequence of steps that shell out to `disko`,
//! `nixos-generate-config`, `sbctl`, `nix` and `nixos-install`, streaming their
//! output to the UI.
//!
//! In dry-run the same steps run, but every command is only *displayed* (with
//! simulated output) and all writes go to the working copy inside a temporary
//! directory.

use crate::runner::{self, Cmd, Stream};
use crate::settings::Settings;
use color_eyre::eyre::{eyre, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

/// The install steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Disk,
    Repo,
    Settings,
    Hardware,
    SecureBoot,
    System,
}

impl Step {
    pub const ALL: [Step; 6] = [
        Step::Disk,
        Step::Repo,
        Step::Settings,
        Step::Hardware,
        Step::SecureBoot,
        Step::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Step::Disk => "disco",
            Step::Repo => "repo",
            Step::Settings => "ajustes",
            Step::Hardware => "hardware",
            Step::SecureBoot => "secure boot",
            Step::System => "sistema",
        }
    }
}

/// Messages sent from the install thread to the UI.
#[derive(Debug, Clone)]
pub enum Msg {
    Step(Step),
    Note(String),
    Log(String),
    Progress { built: u64, total: u64 },
    Finished { ok: bool, error: Option<String>, log_path: PathBuf },
}

/// Everything the installer needs to run.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Source repo (already cloned/available), read-only.
    pub source: PathBuf,
    /// Mount root (`/mnt` for real, a temp dir in dry-run).
    pub root: PathBuf,
    pub settings: Settings,
    pub password_hash: String,
    pub device: String,
    pub keep_uutils: bool,
    pub dry_run: bool,
}

impl Plan {
    pub fn installed_repo(&self) -> PathBuf {
        self.root
            .join("home")
            .join(&self.settings.username)
            .join("nixdots")
    }

    pub fn log_path(&self) -> PathBuf {
        PathBuf::from("/tmp/nixdots-install.log")
    }
}

/// Starts the install on a worker thread.
pub fn spawn(plan: Plan, tx: Sender<Msg>) -> thread::JoinHandle<()> {
    thread::spawn(move || run(plan, tx))
}

struct Reporter {
    tx: Sender<Msg>,
    file: Option<File>,
}

impl Reporter {
    fn log(&mut self, line: impl Into<String>) {
        let line = line.into();
        let _ = self.tx.send(Msg::Log(line.clone()));
        if let Some(file) = self.file.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }

    fn note(&mut self, line: impl Into<String>) {
        let line = line.into();
        let _ = self.tx.send(Msg::Note(line));
    }

    fn step(&self, step: Step) {
        let _ = self.tx.send(Msg::Step(step));
    }

    fn progress(&self, built: u64, total: u64) {
        let _ = self.tx.send(Msg::Progress { built, total });
    }
}

fn run(plan: Plan, tx: Sender<Msg>) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(plan.log_path())
        .ok();
    let mut report = Reporter { tx: tx.clone(), file };

    report.log("== nixdots installer ==");
    report.log(format!(
        "modo: {}",
        if plan.dry_run { "simulación" } else { "real" }
    ));
    report.log(format!("host: {}", plan.settings.host_name));
    report.log(format!("usuario: {}", plan.settings.username));
    report.log(format!("dispositivo: {}", plan.device));
    report.log(format!("repo destino: {}", plan.installed_repo().display()));

    match run_steps(&plan, &mut report) {
        Ok(()) => {
            let log_path = plan.log_path();
            if !plan.dry_run {
                copy_log_to_persist(&plan, &mut report);
            }
            report.log("== instalación completada ==");
            let _ = tx.send(Msg::Finished {
                ok: true,
                error: None,
                log_path,
            });
        }
        Err(err) => {
            report.log(format!("error: {err}"));
            let _ = tx.send(Msg::Finished {
                ok: false,
                error: Some(err.to_string()),
                log_path: plan.log_path(),
            });
        }
    }

    if let Some(file) = report.file.as_mut() {
        let _ = file.flush();
    }
}

fn run_steps(plan: &Plan, report: &mut Reporter) -> Result<()> {
    report.step(Step::Disk);
    step_disk(plan, report)?;

    report.step(Step::Repo);
    step_repo(plan, report)?;

    report.step(Step::Settings);
    step_settings(plan, report)?;

    report.step(Step::Hardware);
    step_hardware(plan, report)?;

    report.step(Step::SecureBoot);
    step_secure_boot(plan, report)?;

    report.step(Step::System);
    step_system(plan, report)?;

    Ok(())
}

fn step_disk(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let cmd = Cmd::new("disko")
        .arg("--mode")
        .arg("destroy,format,mount")
        .arg("disko.nix")
        .arg("--argstr")
        .arg("device")
        .arg(&plan.device)
        .with_display(format!(
            "disko --mode destroy,format,mount disko.nix --argstr device {}",
            plan.device
        ));

    let code = exec(plan, report, &cmd, &plan.source)?;
    ensure_ok(code, "disko falló")
}

fn step_repo(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let dest = plan.installed_repo();
    if plan.dry_run {
        report.log(format!("$ cp -a {} {}", plan.source.display(), dest.display()));
        fs::create_dir_all(&dest)?;
        write_minimum_repo(plan, &dest)?;
        report.log("(simulación) repositorio copiado");
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let cmd = Cmd::new("cp")
        .arg("-a")
        .arg_path(&plan.source)
        .arg_path(&dest)
        .with_display(format!("cp -a {} {}", plan.source.display(), dest.display()));
    let code = exec_in(plan, report, &cmd, plan.source.parent().unwrap_or(&plan.root))?;
    ensure_ok(code, "no se pudo copiar el repositorio")?;
    Ok(())
}

fn write_minimum_repo(plan: &Plan, dest: &Path) -> Result<()> {
    // Enough of a repo for the later (simulated) steps to operate on.
    fs::create_dir_all(dest.join("etc"))?;
    fs::create_dir_all(dest.join("boot"))?;
    fs::write(dest.join("settings.nix"), plan.settings.render())?;
    fs::write(
        dest.join("etc/default.nix"),
        "{\n    imports = [\n        ./global.nix\n        ./uutils.nix\n    ];\n}\n",
    )?;
    fs::write(dest.join("boot/snapshot.sh"), SNAPSHOT_TEMPLATE)?;
    Ok(())
}

const SNAPSHOT_TEMPLATE: &str = "#!/bin/sh\nmount -t btrfs -o subvolid=5 /dev/disk/by-uuid/9ddcd09a-903e-4863-94db-bb1ccb5d3236 /btrfs_tmp\n";

fn step_settings(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    report.log(format!("$ escribir {}/settings.nix", repo.display()));
    plan.settings.write(&repo)?;

    // Password hash -> /persist/secrets/<user>.hash (0700 dir / 0600 file).
    let secrets = plan.root.join("persist/secrets");
    let hash_path = secrets.join(format!("{}.hash", plan.settings.username));
    report.log(format!("$ escribir {}", hash_path.display()));
    if !plan.dry_run {
        fs::create_dir_all(&secrets)?;
        set_mode_dir(&secrets, 0o700)?;
        fs::write(&hash_path, format!("{}\n", plan.password_hash))?;
        set_mode_file(&hash_path, 0o600)?;
    } else {
        fs::create_dir_all(&secrets)?;
        fs::write(&hash_path, "(simulación)\n")?;
    }

    // Optionally drop the uutils replacement.
    let default_nix = repo.join("etc/default.nix");
    if default_nix.exists() {
        if plan.keep_uutils {
            report.log("uutils: se conserva (experimental)");
        } else {
            report.log("uutils: se quita de etc/default.nix");
            let text = fs::read_to_string(&default_nix)?;
            let filtered: String = text
                .lines()
                .filter(|line| !line.contains("uutils.nix"))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&default_nix, format!("{filtered}\n"))?;
        }
    }

    // Repoint the boot rollback at the label instead of a fixed UUID.
    let snapshot = repo.join("boot/snapshot.sh");
    if snapshot.exists() {
        let text = fs::read_to_string(&snapshot)?;
        let replaced = replace_by_uuid_with_label(&text);
        if replaced != text {
            report.log("boot/snapshot.sh: UUID → /dev/disk/by-label/nixos");
            fs::write(&snapshot, replaced)?;
        }
    }

    Ok(())
}

fn step_hardware(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let cmd = Cmd::new("nixos-generate-config")
        .arg("--root")
        .arg_path(&plan.root)
        .arg("--show-hardware-config")
        .with_display("nixos-generate-config --root /mnt --show-hardware-config");

    let mut captured = String::new();
    let code = if plan.dry_run {
        report.log(format!("$ {}", cmd.display));
        captured.push_str(&dry_hardware_config(&plan.settings.system));
        0
    } else {
        exec_capture(plan, report, &cmd, &plan.source, &mut captured)?
    };
    ensure_ok(code, "nixos-generate-config falló")?;

    let repo = plan.installed_repo();
    let hosts = repo.join("hosts").join(&plan.settings.host_name);
    let target = if repo.join("hosts").is_dir() {
        fs::create_dir_all(&hosts)?;
        hosts.join("hardware-configuration.nix")
    } else {
        report.note("no hay hosts/; se escribe ./hardware-configuration.nix");
        repo.join("hardware-configuration.nix")
    };
    report.log(format!("$ escribir {}", target.display()));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, captured)?;
    Ok(())
}

fn step_secure_boot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    // lanzaboote (systemd-boot) only targets x86_64.
    let system = if plan.settings.system.is_empty() {
        "x86_64-linux"
    } else {
        plan.settings.system.as_str()
    };
    if !system.starts_with("x86_64") {
        return Err(eyre!(
            "lanzaboote no soporta {system}; secure boot se detiene en lugar de improvisar"
        ));
    }

    let keys = plan.root.join("persist/var/lib/sbctl");
    let keys_dir = keys.join("keys");
    if keys_dir.is_dir() {
        report.log("sbctl: llaves ya presentes");
        return Ok(());
    }

    report.log("sbctl: no hay llaves, se generan en /mnt/persist/var/lib/sbctl");
    let cmd = Cmd::new("sbctl")
        .arg("create-keys")
        .arg("--directory")
        .arg_path(&keys)
        .with_display("sbctl create-keys --directory /mnt/persist/var/lib/sbctl");
    let code = exec(plan, report, &cmd, &plan.source)?;
    ensure_ok(code, "sbctl create-keys falló")?;
    Ok(())
}

fn step_system(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    let attr = format!(
        "{}#nixosConfigurations.{}.config.system.build.toplevel",
        repo.display(),
        plan.settings.host_name
    );

    let build = Cmd::new("nix")
        .arg("build")
        .arg(&attr)
        .arg("--store")
        .arg_path(&plan.root)
        .arg("--impure")
        .arg("--log-format")
        .arg("internal-json")
        .arg("-v")
        .with_display(format!(
            "nix build {attr} --store /mnt --log-format internal-json -v"
        ));

    let system_path = if plan.dry_run {
        report.log(format!("$ {}", build.display));
        simulate_build(report);
        format!("/nix/store/{}-nixos-system-{}", "0".repeat(32), plan.settings.host_name)
    } else {
        build_system(plan, report, &build)?
    };

    let cmd = Cmd::new("nixos-install")
        .arg("--system")
        .arg(&system_path)
        .arg("--no-root-passwd")
        .with_display(format!("nixos-install --system {system_path} --no-root-passwd"));
    let code = exec(plan, report, &cmd, &repo)?;
    ensure_ok(code, "nixos-install falló")?;
    Ok(())
}

fn build_system(plan: &Plan, report: &mut Reporter, build: &Cmd) -> Result<String> {
    report.log(format!("$ {}", build.display));
    let mut progress = NixProgress::default();
    let mut out_path: Option<String> = None;

    let last_code = {
        let report = &mut *report;
        let mut sink = |out: Stream| {
            let line = match out {
                Stream::Stdout(l) => l,
                Stream::Stderr(l) => l,
            };
            // nix build prints the resulting store path on stdout at the end.
            let trimmed = line.trim();
            if trimmed.starts_with("/nix/store/") && trimmed.contains("-nixos-system") {
                out_path = Some(trimmed.to_string());
            }
            if let Some((built, total)) = progress.feed(trimmed) {
                report.progress(built, total);
            }
            report.log(line);
        };
        runner::run(build, &mut sink)
    };

    if last_code != 0 {
        report.log("la evaluación o la construcción falló; revisa el log");
        return Err(eyre!("nix build terminó con código {last_code}"));
    }

    if let Some(path) = out_path {
        return Ok(path);
    }
    // Fall back to asking nix for the path via `nix path-info` semantics.
    let query = Cmd::new("nix")
        .arg("path-info")
        .arg("--store")
        .arg_path(&plan.root)
        .arg(format!(
            "{}#nixosConfigurations.{}.config.system.build.toplevel",
            plan.installed_repo().display(),
            plan.settings.host_name
        ));
    match runner::capture(&query) {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                Err(eyre!("no se pudo determinar la ruta del sistema"))
            } else {
                Ok(path)
            }
        }
        _ => Err(eyre!("no se pudo determinar la ruta del sistema")),
    }
}

// -- command execution helpers ------------------------------------------------

fn exec(plan: &Plan, report: &mut Reporter, cmd: &Cmd, cwd: &Path) -> Result<i32> {
    exec_in(plan, report, cmd, cwd)
}

fn exec_in(plan: &Plan, report: &mut Reporter, cmd: &Cmd, cwd: &Path) -> Result<i32> {
    report.log(format!("$ {}", cmd.display));
    if plan.dry_run {
        // Simulated, deterministic output; nothing is touched.
        report.log("(simulación) comando no ejecutado");
        thread::sleep(Duration::from_millis(120));
        return Ok(0);
    }

    // `runner::run` has no working-directory support, so cd inside a shell.
    let wrapped = Cmd::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && exec {}",
            shell_quote(cwd),
            shell_command(cmd)
        ))
        .with_display(cmd.display.clone());
    let mut sink = |out: Stream| match out {
        Stream::Stdout(l) => report.log(l),
        Stream::Stderr(l) => report.log(l),
    };
    Ok(runner::run(&wrapped, &mut sink))
}

fn exec_capture(
    plan: &Plan,
    report: &mut Reporter,
    cmd: &Cmd,
    cwd: &Path,
    captured: &mut String,
) -> Result<i32> {
    report.log(format!("$ {}", cmd.display));
    if plan.dry_run {
        report.log("(simulación) comando no ejecutado");
        return Ok(0);
    }
    let wrapped = Cmd::new("sh")
        .arg("-c")
        .arg(format!("cd {} && exec {}", shell_quote(cwd), shell_command(cmd)));
    let mut sink = |out: Stream| match out {
        Stream::Stdout(l) => {
            captured.push_str(&l);
            captured.push('\n');
            report.log(l);
        }
        Stream::Stderr(l) => report.log(l),
    };
    Ok(runner::run(&wrapped, &mut sink))
}

fn ensure_ok(code: i32, what: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(eyre!("{what} (código {code})"))
    }
}

fn shell_quote(path: &Path) -> String {
    let s = path.to_string_lossy();
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn shell_command(cmd: &Cmd) -> String {
    let mut parts = vec![shell_quote(Path::new(&cmd.program))];
    for arg in &cmd.args {
        parts.push(format!("'{}'", arg.replace('\'', "'\\''")));
    }
    parts.join(" ")
}

/// Hashes a password with `mkpasswd -m yescrypt --stdin`.
///
/// The plaintext is only ever written to the child's stdin; it is never logged,
/// put in a display string or written to disk.
pub fn hash_password(password: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("mkpasswd")
        .arg("-m")
        .arg("yescrypt")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| eyre!("no se pudo ejecutar mkpasswd: {e}"))?;

    child
        .stdin
        .take()
        .ok_or_else(|| eyre!("mkpasswd no aceptó la entrada"))?
        .write_all(format!("{password}\n").as_bytes())?;

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(eyre!(
            "mkpasswd falló: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// -- misc ---------------------------------------------------------------------

fn set_mode_dir(path: &Path, mode: u32) -> Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms)?;
    Ok(())
}

fn set_mode_file(path: &Path, mode: u32) -> Result<()> {
    set_mode_dir(path, mode)
}

fn copy_log_to_persist(plan: &Plan, report: &mut Reporter) {
    let dest_dir = plan.root.join("persist");
    if !dest_dir.is_dir() {
        return;
    }
    let dest = dest_dir.join("nixdots-install.log");
    match fs::copy(plan.log_path(), &dest) {
        Ok(_) => report.log(format!("log copiado a {}", dest.display())),
        Err(err) => report.log(format!("no se pudo copiar el log: {err}")),
    }
}

/// Replaces the first `/dev/disk/by-uuid/<uuid>` token with the btrfs label.
fn replace_by_uuid_with_label(text: &str) -> String {
    const NEEDLE: &str = "/dev/disk/by-uuid/";
    let Some(start) = text.find(NEEDLE) else {
        return text.to_string();
    };
    let after = start + NEEDLE.len();
    let end = text[after..]
        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
        .map(|i| after + i)
        .unwrap_or(text.len());
    let uuid = &text[after..end];
    if uuid.is_empty() {
        return text.to_string();
    }
    format!(
        "{}/dev/disk/by-label/nixos{}",
        &text[..start],
        &text[end..]
    )
}

fn dry_hardware_config(system: &str) -> String {
    format!(
        "# (simulación) generado por nixos-generate-config\n{{ config, lib, pkgs, modulesPath, ... }}:\n\n{{\n  nixpkgs.hostPlatform = lib.mkDefault \"{system}\";\n}}\n"
    )
}

fn simulate_build(report: &mut Reporter) {
    let steps = 40u64;
    for i in 1..=steps {
        report.progress(i, steps);
        thread::sleep(Duration::from_millis(15));
    }
    report.log("(simulación) sistema construido");
}

/// Very small parser for `nix --log-format internal-json` build progress.
#[derive(Default)]
struct NixProgress {
    built: u64,
    total: u64,
    saw_text: bool,
}

impl NixProgress {
    /// Returns `(built, total)` when it changed.
    fn feed(&mut self, line: &str) -> Option<(u64, u64)> {
        let before = (self.built, self.total);
        let json = line.strip_prefix("@nix ");
        if let Some(json) = json {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
                if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
                    self.parse_text(text);
                }
                if let Some(fields) = value.get("fields").and_then(|f| f.as_array()) {
                    for field in fields {
                        if let Some(s) = field.as_str() {
                            self.parse_text(s);
                        }
                    }
                }
                if value.get("action").and_then(|a| a.as_str()) == Some("stop") && !self.saw_text {
                    self.built += 1;
                    if self.built > self.total {
                        self.total = self.built;
                    }
                }
            }
        } else {
            self.parse_text(line);
        }
        if (self.built, self.total) != before {
            Some((self.built, self.total))
        } else {
            None
        }
    }

    fn parse_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii_digit() {
                i += 1;
                continue;
            }
            let a_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] != b'/' {
                continue;
            }
            let a: u64 = text[a_start..i].parse().unwrap_or(0);
            i += 1;
            let b_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == b_start {
                continue;
            }
            let b: u64 = text[b_start..i].parse().unwrap_or(0);
            let tail = text[i..].trim_start();
            if tail.starts_with("built") || tail.starts_with("copied") {
                self.saw_text = true;
                self.built = self.built.max(a);
                self.total = self.total.max(b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_uuid_but_keeps_rest() {
        let input = "mount -t btrfs -o subvolid=5 /dev/disk/by-uuid/9ddcd09a-903e-4863-94db-bb1ccb5d3236 /btrfs_tmp\n";
        let out = replace_by_uuid_with_label(input);
        assert!(out.contains("/dev/disk/by-label/nixos"));
        assert!(!out.contains("by-uuid"));
        assert!(out.ends_with("/btrfs_tmp\n"));
    }

    #[test]
    fn no_uuid_is_left_alone() {
        let input = "echo hola\n";
        assert_eq!(replace_by_uuid_with_label(input), input);
    }

    #[test]
    fn progress_parses_built_ratio() {
        let mut p = NixProgress::default();
        assert_eq!(p.feed("[3/10 built, 1/2 copied]"), Some((3, 10)));
    }

    #[test]
    fn progress_parses_internal_json() {
        let mut p = NixProgress::default();
        let line = r#"@nix {"action":"msg","text":"[7/42 built]"}"#;
        assert_eq!(p.feed(line), Some((7, 42)));
    }

    #[test]
    fn steps_have_labels() {
        assert_eq!(Step::ALL.len(), 6);
        assert_eq!(Step::Disk.label(), "disco");
    }
}
