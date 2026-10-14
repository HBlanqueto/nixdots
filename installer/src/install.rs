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

/// The install steps shown in the UI, in order. The many real sub-steps are
/// grouped into these four phases; `arranque` covers the bootloader (systemd-boot
/// on utm, lanzaboote/sbctl keys on thinkpad), never a "secure boot" step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Disk,
    Settings,
    System,
    Boot,
}

impl Step {
    pub const ALL: [Step; 4] = [Step::Disk, Step::Settings, Step::System, Step::Boot];

    pub fn label(self) -> &'static str {
        match self {
            Step::Disk => "disco",
            Step::Settings => "ajustes",
            Step::System => "sistema",
            Step::Boot => "arranque",
        }
    }
}

/// The Wi-Fi connection to persist into the installed system.
#[derive(Debug, Clone)]
pub struct WifiSave {
    pub ssid: String,
    /// `true` when NetworkManager made the connection (then its profile can be
    /// copied); otherwise the user must reconnect with iwd/wpa_supplicant.
    pub networkmanager: bool,
}

/// Messages sent from the install thread to the UI.
#[derive(Debug, Clone)]
pub enum Msg {
    Step(Step),
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
    /// Machine class: decides the secure-boot step and the uutils option.
    pub profile: crate::detect::Profile,
    /// Wi-Fi connection to persist into /persist, if any.
    pub wifi: Option<WifiSave>,
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

    report.step(Step::Settings);
    step_repo(plan, report)?;
    step_settings(plan, report)?;
    step_hardware(plan, report)?;
    step_persist(plan, report)?;

    report.step(Step::System);
    step_system(plan, report)?;

    report.step(Step::Boot);
    step_boot(plan, report)?;

    Ok(())
}

/// The final `arranque` phase: lanzaboote's sbctl keys on the ThinkPad; plain
/// systemd-boot (already installed with the system) on the UTM VM.
fn step_boot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    if plan.profile == crate::detect::Profile::Thinkpad {
        step_secure_boot(plan, report)
    } else {
        report.log("systemd-boot: instalado junto con el sistema");
        Ok(())
    }
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
    // Enough of a repo for the later (simulated) steps to operate on, in the
    // hosts/modules layout.
    fs::create_dir_all(dest.join("hosts"))?;
    fs::create_dir_all(dest.join("modules/common"))?;
    fs::create_dir_all(dest.join("modules/hardware"))?;
    fs::write(dest.join("settings.nix"), plan.settings.render())?;
    fs::write(
        dest.join("modules/common/default.nix"),
        "{\n    imports = [\n        ./user.nix\n        ./uutils.nix\n    ];\n}\n",
    )?;
    fs::write(dest.join("modules/hardware/snapshot.sh"), SNAPSHOT_TEMPLATE)?;
    Ok(())
}

const SNAPSHOT_TEMPLATE: &str = "#!/bin/sh\nmount -t btrfs -o subvolid=5 /dev/disk/by-uuid/9ddcd09a-903e-4863-94db-bb1ccb5d3236 /btrfs_tmp\n";

fn step_settings(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    report.log(format!("$ escribir {}/settings.nix", repo.display()));
    plan.settings.write(&repo)?;

    // Password hash -> /persist/secrets/<user>.hash (0711 dir / 0600 file).
    // The directory is traversable (but not listable) so the unprivileged
    // `nix` evaluation can stat the hash; the file stays 0600 root-owned.
    let secrets = plan.root.join("persist/secrets");
    let hash_path = secrets.join(format!("{}.hash", plan.settings.username));
    report.log(format!("$ escribir {}", hash_path.display()));
    if !plan.dry_run {
        fs::create_dir_all(&secrets)?;
        set_mode_dir(&secrets, 0o711)?;
        fs::write(&hash_path, format!("{}\n", plan.password_hash))?;
        set_mode_file(&hash_path, 0o600)?;
    } else {
        fs::create_dir_all(&secrets)?;
        fs::write(&hash_path, "(simulación)\n")?;
    }

    // Optionally drop the uutils replacement (imported by modules/common).
    let default_nix = repo.join("modules/common/default.nix");
    if default_nix.exists() {
        if plan.keep_uutils {
            report.log("uutils: se conserva (experimental)");
        } else {
            report.log("uutils: se quita de modules/common/default.nix");
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
    let snapshot = repo.join("modules/hardware/snapshot.sh");
    if snapshot.exists() {
        let text = fs::read_to_string(&snapshot)?;
        let replaced = replace_by_uuid_with_label(&text);
        if replaced != text {
            report.log("modules/hardware/snapshot.sh: UUID → /dev/disk/by-label/nixos");
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
        captured.push_str(&dry_hardware_config(plan.profile.system()));
        0
    } else {
        exec_capture(plan, report, &cmd, &plan.source, &mut captured)?
    };
    ensure_ok(code, "nixos-generate-config falló")?;

    write_host_files(plan, report, &captured)?;
    Ok(())
}

/// Writes the generated hardware-configuration.nix into hosts/<hostname>/ and
/// creates default.nix + host.nix when the hostname is a new one. Existing
/// host.nix/default.nix are never overwritten.
fn write_host_files(plan: &Plan, report: &mut Reporter, hardware_config: &str) -> Result<()> {
    let repo = plan.installed_repo();
    let host = &plan.settings.host_name;
    let dir = repo.join("hosts").join(host);
    let target = if repo.join("hosts").is_dir() {
        fs::create_dir_all(&dir)?;
        dir.join("hardware-configuration.nix")
    } else {
        report.log("no hay hosts/; se escribe ./hardware-configuration.nix");
        repo.join("hardware-configuration.nix")
    };
    report.log(format!("$ escribir {}", target.display()));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, hardware_config)?;

    if repo.join("hosts").is_dir() {
        let host_nix = dir.join("host.nix");
        if !host_nix.exists() {
            report.log(format!("$ escribir {}", host_nix.display()));
            fs::write(
                &host_nix,
                format!(
                    "{{\n    profile = \"{}\";\n    system = \"{}\";\n}}\n",
                    plan.profile.name(),
                    plan.profile.system()
                ),
            )?;
        }
        let default_nix = dir.join("default.nix");
        if !default_nix.exists() {
            report.log(format!("$ escribir {}", default_nix.display()));
            fs::write(
                &default_nix,
                format!(
                    "{{ ... }}:\n\n{{\n    imports = [\n        ./hardware-configuration.nix\n        ../../modules/hardware/{}.nix\n    ];\n}}\n",
                    plan.profile.name()
                ),
            )?;
        }
    }
    Ok(())
}

/// Creates the /persist layout declared by `environment.persistence`, the
/// secrets directory and (when NetworkManager connected) the saved Wi-Fi
/// profile. Runs after the disk is mounted and before nixos-install.
fn step_persist(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    let persist = plan.root.join("persist");

    let expr = format!(
        "{}#nixosConfigurations.{}.config.environment.persistence",
        repo.display(),
        plan.settings.host_name
    );
    let cmd = Cmd::new("nix")
        .arg("eval")
        .arg("--impure")
        .arg("--json")
        .arg(&expr)
        .with_display(format!("nix eval --impure --json {expr}"));

    if plan.dry_run {
        report.log(format!("$ {}", cmd.display));
        fs::create_dir_all(persist.join("etc/NetworkManager/system-connections"))?;
        fs::create_dir_all(persist.join("secrets"))?;
        report.log("(simulación) /persist preparado");
        persist_wifi(plan, report)?;
        return Ok(());
    }

    let mut captured = String::new();
    let code = exec_capture(plan, report, &cmd, &repo, &mut captured)?;
    ensure_ok(code, "no se pudo evaluar environment.persistence")?;
    let value: serde_json::Value =
        serde_json::from_str(&captured).map_err(|e| eyre!("persistencia ilegible: {e}"))?;
    let entries = value
        .get("/persist")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if let Some(dirs) = entries.get("directories").and_then(|d| d.as_array()) {
        for entry in dirs {
            create_persist_dir(plan, report, &persist, entry)?;
        }
    }
    if let Some(files) = entries.get("files").and_then(|f| f.as_array()) {
        for entry in files {
            create_persist_file(plan, report, &persist, entry)?;
        }
    }

    // Secrets live next to the persisted tree. 0711 keeps the hash file
    // unreadable while still letting the unprivileged `nix` evaluation stat it.
    let secrets = persist.join("secrets");
    fs::create_dir_all(&secrets)?;
    set_mode_dir(&secrets, 0o711)?;

    persist_wifi(plan, report)?;
    Ok(())
}

fn entry_path(entry: &serde_json::Value) -> Option<String> {
    if let Some(path) = entry.as_str() {
        Some(path.to_string())
    } else {
        entry
            .get("directory")
            .or_else(|| entry.get("file"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

fn create_persist_dir(
    _plan: &Plan,
    report: &mut Reporter,
    persist: &Path,
    entry: &serde_json::Value,
) -> Result<()> {
    let Some(path) = entry_path(entry) else {
        return Ok(());
    };
    let target = persist.join(path.trim_start_matches('/'));
    fs::create_dir_all(&target)?;
    // The NetworkManager connections directory is private.
    let mode = if path.contains("NetworkManager/system-connections") {
        0o700
    } else {
        entry
            .get("mode")
            .and_then(|m| m.as_u64())
            .map(|m| m as u32)
            .unwrap_or(0o755)
    };
    set_mode_dir(&target, mode)?;

    if let Some(user) = entry.get("user").and_then(|u| u.as_str()) {
        let group = entry.get("group").and_then(|g| g.as_str()).unwrap_or(user);
        let owner = format!("{user}:{group}");
        if chown(&target, &owner).is_err() {
            report.log(format!("aviso: no se pudo fijar el dueño {owner} en {}", target.display()));
        }
    }
    report.log(format!("$ mkdir -p {}", target.display()));
    Ok(())
}

fn create_persist_file(
    _plan: &Plan,
    report: &mut Reporter,
    persist: &Path,
    entry: &serde_json::Value,
) -> Result<()> {
    let Some(path) = entry_path(entry) else {
        return Ok(());
    };
    if path.ends_with("etc/machine-id") {
        // A fresh machine-id must exist before the first boot.
        report.log("$ systemd-machine-id-setup --root=/mnt/persist");
        let code = runner::run(
            &Cmd::new("systemd-machine-id-setup")
                .arg("--root")
                .arg_path(persist)
                .with_display("systemd-machine-id-setup --root=/mnt/persist"),
            &mut |_| {},
        );
        if code != 0 {
            // Fall back to an empty file so the mount is at least present.
            let target = persist.join(path.trim_start_matches('/'));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, "")?;
        }
    } else {
        let target = persist.join(path.trim_start_matches('/'));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if !target.exists() {
            fs::write(&target, "")?;
        }
    }
    Ok(())
}

/// Copies the Wi-Fi connection into /persist when NetworkManager is in use;
/// otherwise just notes that the VM/user must reconnect.
fn persist_wifi(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let Some(wifi) = &plan.wifi else {
        return Ok(());
    };
    if !wifi.networkmanager {
        report.log(format!(
            "Wi-Fi {ssid}: guardada por iwd/wpa_supplicant, reconecta tras instalar",
            ssid = wifi.ssid
        ));
        return Ok(());
    }
    let src = PathBuf::from(format!(
        "/etc/NetworkManager/system-connections/{}.nmconnection",
        wifi.ssid
    ));
    let dst_dir = plan.root.join("persist/etc/NetworkManager/system-connections");
    fs::create_dir_all(&dst_dir)?;
    set_mode_dir(&dst_dir, 0o700)?;
    let dst = dst_dir.join(format!("{}.nmconnection", wifi.ssid));
    report.log(format!("$ cp {} {}", src.display(), dst.display()));
    if plan.dry_run {
        fs::write(&dst, "")?;
    } else {
        match fs::copy(&src, &dst) {
            Ok(_) => set_mode_file(&dst, 0o600)?,
            Err(err) => report.log(format!("aviso: no se pudo copiar la conexión Wi-Fi: {err}")),
        }
    }
    Ok(())
}

fn chown(path: &Path, owner: &str) -> std::io::Result<()> {
    let status = std::process::Command::new("chown")
        .arg("-R")
        .arg(owner)
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("chown falló"))
    }
}

fn step_secure_boot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    // lanzaboote (systemd-boot) only targets x86_64.
    let system = plan.profile.system();
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
            "nix build --impure {attr} --store /mnt --log-format internal-json -v"
        ));

    let system_path = if plan.dry_run {
        report.log(format!("$ {}", build.display));
        simulate_build(report);
        format!("/nix/store/{}-nixos-system-{}", "0".repeat(32), plan.settings.host_name)
    } else {
        build_system(plan, report, &build)?
    };

    let cmd = Cmd::new("nixos-install")
        .arg("--root")
        .arg_path(&plan.root)
        .arg("--system")
        .arg(&system_path)
        .arg("--no-root-passwd")
        .with_display(format!(
            "nixos-install --root /mnt --system {system_path} --no-root-passwd"
        ));
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
        // Simulated, deterministic output; nothing is touched. The pause keeps
        // the simulation in the ~20–30 s range so the whole screen is legible.
        report.log("(simulación) comando no ejecutado");
        thread::sleep(Duration::from_millis(500));
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
    let steps = 100u64;
    for i in 1..=steps {
        report.progress(i, steps);
        if i % 20 == 0 {
            report.log(format!(
                "building '/nix/store/{}000000000000000000000000000000000.drv'...",
                i
            ));
        }
        thread::sleep(Duration::from_millis(180));
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
        assert_eq!(Step::ALL.len(), 4);
        assert_eq!(Step::Disk.label(), "disco");
        assert_eq!(Step::Settings.label(), "ajustes");
        assert_eq!(Step::System.label(), "sistema");
        assert_eq!(Step::Boot.label(), "arranque");
    }
}
