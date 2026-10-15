//! The actual installation: a sequence of steps that shell out to `disko`,
//! `nixos-generate-config`, `sbctl`, `nix` and `nixos-install`, streaming their
//! output to the UI.
//!
//! In dry-run the same steps run, but every command is only *displayed* (with
//! simulated output) and all writes go to the working copy inside a temporary
//! directory.

use crate::detect;
use crate::runner::{self, Cmd, Stream};
use crate::settings::Settings;
use color_eyre::eyre::{eyre, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;
use zeroize::Zeroizing;

/// The install steps shown in the UI, in order. The many real sub-steps are
/// grouped into these four phases; `arranque` covers the bootloader (systemd-boot
/// on utm, lanzaboote/sbctl keys on thinkpad), never a "secure boot" step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// A level-0 nix message line; shown in red.
    Error(String),
    Progress { built: u64, total: u64 },
    /// A child process finished; the UI should wipe and fully redraw.
    Repaint,
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
    /// Whether the System HD partition is wrapped in LUKS2.
    pub encrypt: bool,
    /// Passphrase for the LUKS-encrypted root (the account password, or the
    /// one typed on the Cifrado screen). Never logged, only fed over stdin.
    pub luks_passphrase: Zeroizing<String>,
    /// Recovery key enrolled as a second LUKS slot. Wiped after use.
    pub recovery_key: Zeroizing<String>,
    /// Enroll TPM2 (PCR 7) auto-unlock in addition to the passphrase.
    pub luks_tpm: bool,
    pub device: String,
    pub keep_uutils: bool,
    pub dry_run: bool,
    /// Machine class: decides the secure-boot step and the uutils option.
    pub profile: crate::detect::Profile,
    /// Wi-Fi connection to persist into /persist, if any.
    pub wifi: Option<WifiSave>,
    /// First step to run. On a retry this is the step that failed, so the disk
    /// is reattached instead of reformatted.
    pub resume_from: Step,
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

    /// Git identity for the installer's commits: the Git screen's when set,
    /// else a throwaway one.
    fn git_identity(&self) -> (String, String) {
        (
            self.settings
                .git_name
                .clone()
                .unwrap_or_else(|| "nixdots-installer".to_string()),
            self.settings
                .git_email
                .clone()
                .unwrap_or_else(|| "installer@localhost".to_string()),
        )
    }
}

/// A `git` invocation rooted at `repo`, with `safe.directory` set per call (the
/// repo belongs to the user but the installer runs as root).
fn git_cmd(repo: &Path, args: &[&str]) -> Cmd {
    let mut cmd = Cmd::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", repo.display()))
        .arg("-C")
        .arg_path(repo);
    for arg in args {
        cmd = cmd.arg(*arg);
    }
    cmd
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
        self.write(&line);
    }

    /// Writes a line to the on-disk log only (raw nix output, unfiltered).
    fn raw(&mut self, line: &str) {
        self.write(line);
    }

    /// Sends a filtered display line to the UI, without touching the file.
    fn emit(&mut self, line: String, is_error: bool) {
        let msg = if is_error { Msg::Error(line) } else { Msg::Log(line) };
        let _ = self.tx.send(msg);
    }

    fn write(&mut self, line: &str) {
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

    fn repaint(&self) {
        let _ = self.tx.send(Msg::Repaint);
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

    // The build's scratch dir lives on the target disk; drop it when done.
    if !plan.dry_run {
        let _ = fs::remove_dir_all(plan.root.join("tmp"));
    }

    if let Some(file) = report.file.as_mut() {
        let _ = file.flush();
    }
}

fn run_steps(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let resume = plan.resume_from;

    // Disk is only formatted when it is the (re)starting point. When resuming
    // from a later step the prepared disk is reattached, never reformatted.
    let luks = if resume <= Step::Disk {
        report.step(Step::Disk);
        step_disk(plan, report)?
    } else {
        ensure_disk_ready(plan, report)?;
        if plan.encrypt {
            Some(detect::luks_partition(&plan.device).ok_or_else(|| {
                eyre!("no se encontró la partición cifrada en {}", plan.device)
            })?)
        } else {
            None
        }
    };

    // Scratch space on the target disk (not the ISO's RAM store) for the build.
    prepare_target_tmp(plan, report)?;

    if resume <= Step::Settings {
        report.step(Step::Settings);
        step_repo(plan, report)?;
        step_settings(plan, report)?;
        step_hardware(plan, report, luks.as_ref())?;
        step_persist(plan, report)?;
    }

    if resume <= Step::System {
        ensure_target_space(plan)?;
        // Kept alive for the whole system build; dropped (swapoff + delete) when
        // this block ends, so it is also recreated on every retry.
        let _swap = ensure_build_memory(plan, report)?;
        report.step(Step::System);
        step_system(plan, report)?;
    }

    if resume <= Step::Boot {
        report.step(Step::Boot);
        step_boot(plan, report)?;
    }
    Ok(())
}

/// Whether `path` is a mount point, per `/proc/mounts` (its second field).
fn is_mounted(path: &str) -> bool {
    fs::read_to_string("/proc/mounts")
        .map(|mounts| {
            mounts
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(path))
        })
        .unwrap_or(false)
}

/// Creates `<root>/tmp` (mode 1777) and points `TMPDIR` at it, so every child
/// process (`nix`, `nixos-install`, …) keeps its scratch data on the target
/// disk instead of the live ISO's RAM-backed `/tmp`.
fn prepare_target_tmp(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let tmp = plan.root.join("tmp");
    if !plan.dry_run {
        fs::create_dir_all(&tmp)?;
        set_mode_dir(&tmp, 0o1777)?;
    }
    std::env::set_var("TMPDIR", &tmp);
    // The root-aware password assertion (modules/common/user.nix) checks
    // `${NIXDOTS_ROOT}${hashedPasswordFile}`; point it at the target.
    std::env::set_var("NIXDOTS_ROOT", &plan.root);
    report.log(format!("$ TMPDIR={}", tmp.display()));
    Ok(())
}

/// Minimum free space on the target before building the system: the closure is
/// large (kernel, desktop, …) and is realised in the target store.
const MIN_TARGET_GIB: u64 = 25;

/// Fails early, in Spanish, when the target has less than [`MIN_TARGET_GIB`] of
/// free space, instead of letting `nixos-install` die with `ENOSPC` halfway.
fn ensure_target_space(plan: &Plan) -> Result<()> {
    if plan.dry_run {
        return Ok(());
    }
    let free = target_free_gib(&plan.root)?;
    if free < MIN_TARGET_GIB {
        return Err(eyre!(
            "espacio insuficiente en {}: hay {} GiB libres y se necesitan al menos {} GiB para instalar el sistema",
            plan.root.display(),
            free,
            MIN_TARGET_GIB
        ));
    }
    Ok(())
}

/// Free space (GiB) on the filesystem holding `root`, via `df`.
fn target_free_gib(root: &Path) -> Result<u64> {
    let out = runner::capture(&Cmd::new("df").arg("-Pk").arg_path(root))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let avail_kb: u64 = text
        .lines()
        .last()
        .and_then(|line| line.split_whitespace().nth(3))
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| eyre!("no se pudo leer el espacio libre de {}", root.display()))?;
    Ok(avail_kb / 1024 / 1024)
}

/// Total RAM in GiB, read from `MemTotal` in `/proc/meminfo`. `None` when it
/// cannot be read, which callers treat as "little RAM".
fn mem_total_gib() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = text
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    Some(kb / 1024 / 1024)
}

/// Below this much RAM the build is throttled and a temporary swap is enabled.
const LOW_MEM_GIB: u64 = 12;

/// A temporary 8 GiB swapfile on the target disk, alive only while the system is
/// built. `swapoff` and deletion happen on drop, so it never reaches the
/// installed system whether the step succeeds or fails.
struct TempSwap {
    path: PathBuf,
}

impl TempSwap {
    fn create(plan: &Plan, report: &mut Reporter) -> Result<Self> {
        let path = plan.root.join("persist/.nixdots-install.swap");
        // Clear any leftover from an attempt that was cut short; it may still be
        // active. Failures are ignored (nothing to clean on a first run).
        let mut sink = |_: Stream| {};
        let _ = runner::run(&Cmd::new("swapoff").arg_path(&path), &mut sink);
        let _ = fs::remove_file(&path);

        let mkswap = Cmd::new("btrfs")
            .arg("filesystem")
            .arg("mkswapfile")
            .arg("--size")
            .arg("8g")
            .arg_path(&path);
        ensure_ok(
            exec(plan, report, &mkswap, &plan.source)?,
            "no se pudo crear el swap temporal",
        )?;

        let swapon = Cmd::new("swapon").arg_path(&path);
        ensure_ok(
            exec(plan, report, &swapon, &plan.source)?,
            "no se pudo activar el swap temporal",
        )?;

        Ok(Self { path })
    }
}

impl Drop for TempSwap {
    fn drop(&mut self) {
        let _ = std::process::Command::new("swapoff")
            .arg(&self.path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let _ = fs::remove_file(&self.path);
    }
}

/// Frees memory for the build on small machines. On the live ISO the store and
/// `/tmp` live in RAM and `nixos-install` unpacks many paths at once, so the OOM
/// killer can hit it (exit 137). Under [`LOW_MEM_GIB`] we turn on a temporary
/// swapfile on the target disk; with plenty of RAM nothing changes.
fn ensure_build_memory(plan: &Plan, report: &mut Reporter) -> Result<Option<TempSwap>> {
    if plan.dry_run {
        return Ok(None);
    }
    let ram = mem_total_gib();
    if ram.is_some_and(|g| g >= LOW_MEM_GIB) {
        return Ok(None);
    }
    let ram_label = ram
        .map(|g| format!("{g} GiB"))
        .unwrap_or_else(|| "desconocida".to_string());

    // The swapfile and the build's closure both live on the target disk.
    let needed = MIN_TARGET_GIB + 8;
    let free = target_free_gib(&plan.root)?;
    if free < needed {
        report.log(format!(
            "RAM: {ram_label} — sin espacio en {} para el swap temporal ({} GiB libres, se necesitan {needed} GiB); se continúa sin él",
            plan.root.display(),
            free
        ));
        return Ok(None);
    }

    report.log(format!(
        "RAM: {ram_label} — se activa un swap temporal de 8 GiB en el disco destino para la instalación"
    ));
    Ok(Some(TempSwap::create(plan, report)?))
}

/// Reattaches the already-prepared disk for a retry: if `/mnt` is not mounted
/// (or `cryptroot` is closed) it re-opens and mounts the existing filesystems
/// with `disko --mode mount`, which never formats. The passphrase comes from the
/// in-memory plan, so the disk is left exactly as the disk step left it.
fn ensure_disk_ready(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let encrypted_open = !plan.encrypt || Path::new("/dev/mapper/cryptroot").exists();
    if is_mounted("/mnt") && encrypted_open {
        report.log("disco ya montado; se continúa sin reformatear");
        return Ok(());
    }

    if plan.dry_run {
        report.log(format!(
            "$ disko --mode mount disko.nix --argstr device {}{}",
            plan.device,
            if plan.encrypt {
                " --argstr encrypt true"
            } else {
                ""
            }
        ));
        report.log("(simulación) disco re-montado sin formatear");
        return Ok(());
    }

    let mut cmd = Cmd::new("disko")
        .arg("--mode")
        .arg("mount")
        .arg("disko.nix")
        .arg("--argstr")
        .arg("device")
        .arg(&plan.device);
    if plan.encrypt {
        cmd = cmd.arg("--argstr").arg("encrypt").arg("true");
    }

    let pass = if plan.encrypt {
        Some(SecretFile::write("luks.pass", &plan.luks_passphrase)?)
    } else {
        None
    };
    if let Some(pass) = &pass {
        cmd = cmd.arg("--argstr").arg("luksPasswordFile").arg_path(pass.path());
    }

    ensure_ok(
        exec(plan, report, &cmd, &plan.source)?,
        "no se pudo volver a montar el disco",
    )
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

fn step_disk(plan: &Plan, report: &mut Reporter) -> Result<Option<detect::LuksPartition>> {
    // A retry can find /mnt still mounted and `cryptroot` open from the run that
    // failed; clear everything so disko starts from a clean slate instead of
    // choking on a half-prepared disk.
    if !plan.dry_run {
        clean_target(plan, report);
    }

    // The passphrase is written once, verbatim (no trailing newline), and reused
    // by disko, the slot check, luksAddKey and the TPM enroll. Nothing else
    // writes it; `SecretFile` shreds it when this function returns.
    let pass = if plan.encrypt && !plan.dry_run {
        Some(SecretFile::write("luks.pass", &plan.luks_passphrase)?)
    } else {
        None
    };

    let mut cmd = Cmd::new("disko")
        .arg("--mode")
        .arg("destroy,format,mount")
        .arg("--yes-wipe-all-disks")
        .arg("disko.nix")
        .arg("--argstr")
        .arg("device")
        .arg(&plan.device);
    if plan.encrypt {
        cmd = cmd.arg("--argstr").arg("encrypt").arg("true");
        if let Some(pass) = &pass {
            cmd = cmd.arg("--argstr").arg("luksPasswordFile").arg_path(pass.path());
        }
        cmd = cmd.with_display(format!(
            "disko --mode destroy,format,mount --yes-wipe-all-disks disko.nix --argstr device {} --argstr encrypt true",
            plan.device
        ));
    } else {
        cmd = cmd.with_display(format!(
            "disko --mode destroy,format,mount --yes-wipe-all-disks disko.nix --argstr device {}",
            plan.device
        ));
    }

    // disko reads the passphrase from `luksPasswordFile` and runs luksFormat
    // itself; the installer never formats the container.
    ensure_ok(
        exec(plan, report, &cmd, &plan.source)?,
        if plan.encrypt {
            "no se pudo cifrar y formatear el disco"
        } else {
            "no se pudo formatear el disco"
        },
    )?;

    if !plan.encrypt {
        return Ok(None);
    }

    let luks = if plan.dry_run {
        detect::LuksPartition {
            path: format!("{}2", plan.device),
            uuid: "00000000-0000-0000-0000-000000000000".to_string(),
        }
    } else {
        // Identify the LUKS partition by its filesystem type, never by a label:
        // udev escapes spaces in `by-partlabel` (`System\x20HD`) and disko names
        // the GPT label `disk-<disk>-<name>` by default.
        detect::luks_partition(&plan.device)
            .ok_or_else(|| eyre!("no se encontró la partición cifrada en {}", plan.device))?
    };
    report.log(format!(
        "$ partición cifrada: {} (UUID {})",
        luks.path, luks.uuid
    ));

    let pass_path = pass.as_ref().map(SecretFile::path);
    if let Some(pass_path) = pass_path {
        // Bail out before touching any slot if disko's key and ours disagree.
        verify_key(
            report,
            &luks.path,
            pass_path,
            "la frase del disco no coincide con la configurada",
        )?;
    }

    step_recovery_key(plan, report, &luks.path, pass_path)?;
    if plan.luks_tpm {
        step_tpm_enroll(plan, report, &luks.path, pass_path)?;
    }
    Ok(Some(luks))
}

/// Unmounts the target, closes the LUKS mapper and wipes any LUKS header left
/// behind by a previous attempt. Failures are ignored: on a first run there is
/// nothing to clean.
fn clean_target(plan: &Plan, report: &mut Reporter) {
    report.log(
        "$ umount -R /mnt; cryptsetup close cryptroot; wipefs -a <partición>   (limpieza previa)",
    );
    let mut sink = |_: Stream| {};
    let _ = runner::run(&Cmd::new("umount").arg("-R").arg("/mnt"), &mut sink);
    let _ = runner::run(&Cmd::new("cryptsetup").arg("close").arg("cryptroot"), &mut sink);
    if let Some(luks) = detect::luks_partition(&plan.device) {
        let _ = runner::run(&Cmd::new("wipefs").arg("-a").arg(&luks.path), &mut sink);
    }
    // Drop stale /run secrets; the next step rewrites the ones it needs.
    let _ = fs::remove_dir_all("/run/nixdots");
}

/// Runs `cryptsetup open --test-passphrase` with `key_file`, so a key that does
/// not match the header is caught before any slot is touched. `what` is the
/// failure message.
fn verify_key(
    report: &mut Reporter,
    device: &str,
    key_file: &Path,
    what: &str,
) -> Result<()> {
    let cmd = Cmd::new("cryptsetup")
        .arg("open")
        .arg("--test-passphrase")
        .arg("--key-file")
        .arg_path(key_file)
        .arg(device)
        .with_display(format!(
            "cryptsetup open --test-passphrase --key-file {} {device}",
            key_file.display()
        ));
    report.log(format!("$ {}", cmd.display));
    let mut sink = |_: Stream| {};
    let code = runner::run(&cmd, &mut sink);
    report.repaint();
    ensure_ok(code, what)
}

/// A root-only secret written to `/run` (tmpfs) and `shred -u`d the moment it
/// goes out of scope, so it never survives on disk even when the step fails.
///
/// The bytes are written *verbatim*: `cryptsetup --key-file` consumes the whole
/// file, so a trailing newline would become part of the passphrase and no longer
/// match what disko's `cat`-based key file (or the user at boot) uses.
struct SecretFile {
    path: PathBuf,
}

impl SecretFile {
    fn write(name: &str, secret: &str) -> Result<Self> {
        let dir = Path::new("/run/nixdots");
        fs::create_dir_all(dir)?;
        set_mode_dir(dir, 0o700)?;
        let path = dir.join(name);
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|err| eyre!("no se pudo crear {}: {err}", path.display()))?;
        file.write_all(secret.as_bytes())?;
        file.flush()?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SecretFile {
    fn drop(&mut self) {
        let _ = std::process::Command::new("shred")
            .arg("-u")
            .arg("--")
            .arg(&self.path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

/// Whether this cryptsetup supports `--new-keyfile` for `luksAddKey` (2.7+).
fn cryptsetup_has_new_keyfile() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| match std::process::Command::new("cryptsetup")
        .arg("luksAddKey")
        .arg("--help")
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(out) => {
            String::from_utf8_lossy(&out.stdout).contains("--new-keyfile")
                || String::from_utf8_lossy(&out.stderr).contains("--new-keyfile")
        }
        Err(_) => false,
    })
}

/// Adds the recovery key generated on the Cifrado screen as a second LUKS slot
/// on `device`, unlocking with the shared `pass_path` file. The new key is the
/// exact text shown on screen (no trailing newline), so what the user types at
/// boot matches. Both files are short-lived `/run` files removed by
/// [`SecretFile`] on the way out. Nothing is logged.
fn step_recovery_key(
    plan: &Plan,
    report: &mut Reporter,
    device: &str,
    pass_path: Option<&Path>,
) -> Result<()> {
    if plan.dry_run {
        report.log(format!(
            "(simulación) cryptsetup luksAddKey {device}  (clave de recuperación)"
        ));
        thread::sleep(Duration::from_millis(300));
        return Ok(());
    }

    let pass_path = pass_path.ok_or_else(|| eyre!("falta el archivo de la frase del disco"))?;
    let recovery = SecretFile::write("luks.recovery", &plan.recovery_key)?;

    let mut cmd = Cmd::new("cryptsetup").arg("luksAddKey");
    if cryptsetup_has_new_keyfile() {
        cmd = cmd
            .arg("--key-file")
            .arg_path(pass_path)
            .arg("--new-keyfile")
            .arg_path(recovery.path())
            .arg(device);
    } else {
        // Older cryptsetup: the new key is the trailing positional argument.
        cmd = cmd
            .arg("--key-file")
            .arg_path(pass_path)
            .arg(device)
            .arg_path(recovery.path());
    }
    let cmd = cmd.with_display(format!(
        "cryptsetup luksAddKey --key-file {} {} {}",
        pass_path.display(),
        device,
        recovery.path().display()
    ));
    report.log(format!("$ {}", cmd.display));

    let mut nixlog = NixLog::default();
    let mut sink = |out: Stream| {
        let l = match out {
            Stream::Stdout(l) => l,
            Stream::Stderr(l) => l,
        };
        report.raw(&l);
        for (is_err, d) in nixlog.feed(&l).0 {
            report.emit(d, is_err);
        }
    };
    let code = runner::run(&cmd, &mut sink);
    report.repaint();
    ensure_ok(
        code,
        &format!("no se pudo añadir la clave de recuperación a {device}"),
    )?;

    // Prove the new slot opens with the exact bytes we wrote.
    verify_key(
        report,
        device,
        recovery.path(),
        "la clave de recuperación no coincide con la mostrada",
    )
}

/// Enrolls TPM2 auto-unlock (PCR 7 = Secure Boot state) on `device`, unlocking
/// with the shared passphrase file before it is shredded.
fn step_tpm_enroll(
    plan: &Plan,
    report: &mut Reporter,
    device: &str,
    pass_path: Option<&Path>,
) -> Result<()> {
    if plan.dry_run {
        report.log(format!(
            "(simulación) systemd-cryptenroll --tpm2-device=auto {device}"
        ));
        thread::sleep(Duration::from_millis(300));
        return Ok(());
    }

    let pass_path = pass_path.ok_or_else(|| eyre!("falta el archivo de la frase del disco"))?;
    let cmd = Cmd::new("systemd-cryptenroll")
        .arg("--tpm2-device=auto")
        .arg("--tpm2-pcrs=7")
        .arg("--unlock-key-file")
        .arg_path(pass_path)
        .arg(device)
        .with_display(format!(
            "systemd-cryptenroll --tpm2-device=auto --tpm2-pcrs=7 --unlock-key-file {} {device}",
            pass_path.display()
        ));
    report.log(format!("$ {}", cmd.display));

    let mut nixlog = NixLog::default();
    let mut sink = |out: Stream| {
        let l = match out {
            Stream::Stdout(l) => l,
            Stream::Stderr(l) => l,
        };
        report.raw(&l);
        for (is_err, d) in nixlog.feed(&l).0 {
            report.emit(d, is_err);
        }
    };
    let code = runner::run(&cmd, &mut sink);
    report.repaint();
    ensure_ok(
        code,
        &format!("no se pudo inscribir el TPM2 para el desbloqueo automático de {device}"),
    )
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
    // A retry can find the previous copy in place; replace it so `cp -a` does
    // not nest the repo inside itself.
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    let cmd = Cmd::new("cp")
        .arg("-a")
        .arg_path(&plan.source)
        .arg_path(&dest)
        .with_display(format!("cp -a {} {}", plan.source.display(), dest.display()));
    let code = exec_in(plan, report, &cmd, plan.source.parent().unwrap_or(&plan.root))?;
    ensure_ok(code, "no se pudo copiar el repositorio")?;
    ensure_repo_ready(plan, report, &dest)?;
    Ok(())
}

/// Stages everything the installer wrote into the copied repo and commits it,
/// because a flake only sees git-tracked paths: without this, `hosts/<host>` and
/// `settings.nix` would be invisible to `nix eval`/`nix build`. The identity and
/// `safe.directory` are passed per-invocation (`-c`), so nothing is written to
/// the user's git config; the Git screen's identity is used when provided.
fn git_snapshot(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();

    if plan.dry_run {
        report.log(format!(
            "$ git -C {} add -A && git commit -m \"hosts: add {}\"   (simulación)",
            repo.display(),
            plan.settings.host_name
        ));
        return Ok(());
    }

    if !repo.join(".git").exists() {
        report.log("aviso: el repo destino no es un repositorio git; se omite el commit");
        return Ok(());
    }

    let (name, email) = plan.git_identity();
    let name_arg = format!("user.name={name}");
    let email_arg = format!("user.email={email}");
    let message = format!("hosts: add {}", plan.settings.host_name);

    let add = git_cmd(&repo, &["add", "-A"])
        .with_display(format!("git -C {} add -A", repo.display()));
    ensure_ok(exec(plan, report, &add, &plan.source)?, "git add falló")?;

    // Nothing staged (e.g. a retry): HEAD already holds what the flake needs.
    let check = git_cmd(&repo, &["diff", "--cached", "--quiet"])
        .with_display(format!("git -C {} diff --cached --quiet", repo.display()));
    if exec(plan, report, &check, &plan.source)? == 0 {
        report.log("git: sin cambios que confirmar");
        return Ok(());
    }

    let commit = git_cmd(
        &repo,
        &[
            "-c",
            name_arg.as_str(),
            "-c",
            email_arg.as_str(),
            "commit",
            "-m",
            message.as_str(),
        ],
    )
    .with_display(format!(
        "git -C {} commit -m \"{message}\"",
        repo.display()
    ));
    ensure_ok(exec(plan, report, &commit, &plan.source)?, "git commit falló")
}

/// Makes the repo copied into `/mnt` usable by the flake. A snapshot copied from
/// the ISO (`/etc/nixdots-src`, a read-only store path) arrives read-only and
/// without `.git`; give it write permission and an initial commit.
fn ensure_repo_ready(plan: &Plan, report: &mut Reporter, repo: &Path) -> Result<()> {
    let chmod = Cmd::new("chmod")
        .arg("-R")
        .arg("u+w")
        .arg_path(repo)
        .with_display(format!("chmod -R u+w {}", repo.display()));
    let _ = exec(plan, report, &chmod, &plan.source);

    if repo.join(".git").exists() {
        return Ok(());
    }

    let (name, email) = plan.git_identity();
    let name_arg = format!("user.name={name}");
    let email_arg = format!("user.email={email}");
    report.log("git: repo sin .git; se crea un commit inicial");

    let init = git_cmd(repo, &["init"]);
    ensure_ok(exec(plan, report, &init, &plan.source)?, "git init falló")?;
    let add = git_cmd(repo, &["add", "-A"]);
    ensure_ok(exec(plan, report, &add, &plan.source)?, "git add falló")?;
    let commit = git_cmd(
        repo,
        &[
            "-c",
            name_arg.as_str(),
            "-c",
            email_arg.as_str(),
            "commit",
            "-m",
            "nixdots: initial import",
        ],
    );
    ensure_ok(exec(plan, report, &commit, &plan.source)?, "git commit falló")
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

const SNAPSHOT_TEMPLATE: &str = "#!/bin/sh\nmount -t btrfs -o subvolid=5 /dev/disk/by-label/nixos /btrfs_tmp\n";

fn step_settings(plan: &Plan, report: &mut Reporter) -> Result<()> {
    let repo = plan.installed_repo();
    report.log(format!("$ escribir {}/settings.nix", repo.display()));
    plan.settings.write(&repo)?;

    // Keep a copy of the license in the installed system.
    let lic_dst = plan.root.join("etc/nixdots");
    let lic_src = repo.join("LICENSE");
    report.log(format!("$ cp {} {}/LICENSE", lic_src.display(), lic_dst.display()));
    if !plan.dry_run && lic_src.exists() {
        fs::create_dir_all(&lic_dst)?;
        fs::copy(&lic_src, lic_dst.join("LICENSE"))?;
    }

    // Password hash -> /persist/secrets/<user>.hash (0700 dir / 0600 file),
    // root-owned and without a trailing newline. modules/common/user.nix checks
    // `${NIXDOTS_ROOT}${hashedPasswordFile}`, so this is the file the install
    // evaluates.
    let secrets = plan.root.join("persist/secrets");
    let hash_path = secrets.join(format!("{}.hash", plan.settings.username));
    report.log(format!("$ escribir {}", hash_path.display()));
    if !plan.dry_run {
        fs::create_dir_all(&secrets)?;
        set_mode_dir(&secrets, 0o700)?;
        fs::write(&hash_path, plan.password_hash.as_bytes())?;
        set_mode_file(&hash_path, 0o600)?;
        let written = fs::read_to_string(&hash_path).unwrap_or_default();
        if written.is_empty() || !written.starts_with("$y$") {
            return Err(eyre!(
                "el hash de contraseña no es válido en {}; vuelve a la pantalla Contraseña",
                hash_path.display()
            ));
        }
    } else {
        fs::create_dir_all(&secrets)?;
        fs::write(&hash_path, "$y$(simulación)")?;
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

fn step_hardware(
    plan: &Plan,
    report: &mut Reporter,
    luks: Option<&detect::LuksPartition>,
) -> Result<()> {
    let cmd = Cmd::new("nixos-generate-config")
        .arg("--root")
        .arg_path(&plan.root)
        .arg("--show-hardware-config")
        .with_display("nixos-generate-config --root /mnt --show-hardware-config");

    let mut captured = String::new();
    let mut err = String::new();
    let code = if plan.dry_run {
        report.log(format!("$ {}", cmd.display));
        captured.push_str(&dry_hardware_config(plan.profile.system()));
        0
    } else {
        exec_capture(plan, report, &cmd, &plan.source, &mut captured, &mut err)?
    };
    ensure_ok(code, "nixos-generate-config falló")?;

    write_host_files(plan, report, &captured, luks)?;
    Ok(())
}

/// Writes the generated hardware-configuration.nix into hosts/<hostname>/ and
/// creates default.nix + host.nix when the hostname is a new one. Existing
/// host.nix/default.nix are never overwritten. When the root is encrypted, the
/// file is checked to carry `boot.initrd.luks.devices.cryptroot` (written by
/// nixos-generate-config, or injected here from the discovered UUID).
fn write_host_files(
    plan: &Plan,
    report: &mut Reporter,
    hardware_config: &str,
    luks: Option<&detect::LuksPartition>,
) -> Result<()> {
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

    let (config, _changed) = apply_encryption_block(
        hardware_config.to_string(),
        plan.encrypt,
        luks.map(|l| l.uuid.as_str()),
        plan.luks_tpm,
    );
    if plan.encrypt {
        report.log(format!(
            "cifrado: bloque del instalador en hardware-configuration.nix (UUID {}, tpmUnlock = {})",
            luks.map(|l| l.uuid.as_str()).unwrap_or("<UUID>"),
            plan.luks_tpm
        ));
    } else {
        report.log("sin cifrado: no se escribe bloque de cifrado en hardware-configuration.nix");
    }
    fs::write(&target, config)?;

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

    // The flake only sees git-tracked paths, so stage what the installer wrote
    // (hosts/<host>, settings.nix, ...) before evaluating the config.
    git_snapshot(plan, report)?;

    let expr = format!(
        "{}#nixosConfigurations.{}.config.environment.persistence",
        repo.display(),
        plan.settings.host_name
    );
    // Impermanence declares some options (allowOther, method, …) as *removed*:
    // forcing them throws. A plain `--json` dump of the whole attrset forces
    // every option, so project each entry down to just the fields we use.
    let apply = r#"p: { "/persist" = let e = p."/persist" or {}; in { directories = map (d: { directory = d.directory; user = d.user or null; group = d.group or null; mode = d.mode or null; }) (e.directories or []); files = map (f: { file = f.file; }) (e.files or []); }; }"#;
    let cmd = Cmd::new("nix")
        .arg("eval")
        .arg("--impure")
        .arg("--extra-experimental-features")
        .arg("nix-command flakes")
        .arg("--json")
        .arg("--apply")
        .arg(apply)
        .arg(&expr)
        .with_display(format!("nix eval --impure --json --apply <filtro> {expr}"));

    if plan.dry_run {
        report.log(format!("$ {}", cmd.display));
        fs::create_dir_all(persist.join("etc/NetworkManager/system-connections"))?;
        fs::create_dir_all(persist.join("secrets"))?;
        report.log("(simulación) /persist preparado");
        persist_wifi(plan, report)?;
        return Ok(());
    }

    let mut captured = String::new();
    let mut eval_err = String::new();
    let code = exec_capture(plan, report, &cmd, &repo, &mut captured, &mut eval_err)?;
    if code != 0 {
        report_eval_failure(plan, report, &repo, &eval_err);
    }
    ensure_ok(code, "no se pudo evaluar environment.persistence")?;
    let value: serde_json::Value =
        serde_json::from_str(&captured).map_err(|e| eyre!("persistencia ilegible: {e}"))?;
    let entries = value
        .get("/persist")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    if let Some(dirs) = entries.get("directories").and_then(|d| d.as_array()) {
        let mut log_dup = false;
        for entry in dirs {
            if entry_path(entry).as_deref() == Some("/var/log") {
                log_dup = true;
            }
            create_persist_dir(plan, report, &persist, entry)?;
        }
        // Detected by evaluating the config, not hardcoded: the @log subvolume
        // already keeps /var/log, so listing it here too is redundant.
        if log_dup {
            report.log("aviso: /var/log definido dos veces: subvolumen @log e impermanence");
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
        .stdin(std::process::Stdio::null())
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
    let flake = format!("{}#{}", repo.display(), plan.settings.host_name);
    let tmp = plan.root.join("tmp");

    // Make sure the flake sees everything the installer wrote before building.
    git_snapshot(plan, report)?;

    // `nixos-install --flake` builds the system with `--store /mnt`, so the
    // closure lands on the target disk, never in the live ISO's RAM-backed
    // store. `--option build-dir` and `TMPDIR` keep the build's scratch there
    // too. No separate `nix build` runs in the ISO store.
    let cmd = Cmd::new("nixos-install")
        .arg("--flake")
        .arg(&flake)
        .arg("--root")
        .arg_path(&plan.root)
        .arg("--no-root-passwd")
        .arg("--impure")
        .arg("--option")
        .arg("build-dir")
        .arg_path(&tmp)
        .arg("--log-format")
        .arg("internal-json")
        .with_display(format!(
            "nixos-install --flake {flake} --root {} --no-root-passwd --option build-dir {}",
            plan.root.display(),
            tmp.display()
        ));

    let code = run_with_progress(plan, report, &cmd);
    ensure_ok(code, "nixos-install falló")?;
    Ok(())
}

/// Runs `cmd`, streaming its output and updating the build progress bar from
/// nix's `internal-json` log format. In dry-run nothing is executed.
fn run_with_progress(plan: &Plan, report: &mut Reporter, cmd: &Cmd) -> i32 {
    report.log(format!("$ {}", cmd.display));
    if plan.dry_run {
        simulate_build(report);
        return 0;
    }
    let mut nixlog = NixLog::default();
    let mut sink = |out: Stream| {
        let line = match out {
            Stream::Stdout(l) => l,
            Stream::Stderr(l) => l,
        };
        report.raw(&line);
        let (display, progress) = nixlog.feed(&line);
        if let Some((built, total)) = progress {
            report.progress(built, total);
        }
        for (is_err, d) in display {
            report.emit(d, is_err);
        }
    };
    let code = runner::run(cmd, &mut sink);
    report.repaint();
    code
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
    let mut nixlog = NixLog::default();
    let mut sink = |out: Stream| {
        let l = match out {
            Stream::Stdout(l) => l,
            Stream::Stderr(l) => l,
        };
        report.raw(&l);
        for (is_err, d) in nixlog.feed(&l).0 {
            report.emit(d, is_err);
        }
    };
    let code = runner::run(&wrapped, &mut sink);
    report.repaint();
    Ok(code)
}

fn exec_capture(
    plan: &Plan,
    report: &mut Reporter,
    cmd: &Cmd,
    cwd: &Path,
    captured: &mut String,
    err: &mut String,
) -> Result<i32> {
    report.log(format!("$ {}", cmd.display));
    if plan.dry_run {
        report.log("(simulación) comando no ejecutado");
        return Ok(0);
    }
    let wrapped = Cmd::new("sh")
        .arg("-c")
        .arg(format!("cd {} && exec {}", shell_quote(cwd), shell_command(cmd)));
    let mut nixlog = NixLog::default();
    let mut sink = |out: Stream| match out {
        Stream::Stdout(l) => {
            captured.push_str(&l);
            captured.push('\n');
            report.raw(&l);
            for (is_err, d) in nixlog.feed(&l).0 {
                report.emit(d, is_err);
            }
        }
        Stream::Stderr(l) => {
            err.push_str(&l);
            err.push('\n');
            report.raw(&l);
            for (is_err, d) in nixlog.feed(&l).0 {
                report.emit(d, is_err);
            }
        }
    };
    let code = runner::run(&wrapped, &mut sink);
    report.repaint();
    Ok(code)
}

/// On an evaluation failure, points at the missing file (nix reports
/// `path '…' does not exist`) and prints the installed repo's commit.
fn report_eval_failure(plan: &Plan, report: &mut Reporter, repo: &Path, err: &str) {
    match missing_path_hint(err) {
        Some(path) => report.log(format!(
            "error: falta el archivo {path} en {} (revisa el commit del repo)",
            repo.display()
        )),
        None => report.log("error: la evaluación del repo falló; revisa el log"),
    }

    let rev = git_cmd(repo, &["log", "-1", "--oneline"]);
    let mut out = String::new();
    let mut git_err = String::new();
    if exec_capture(plan, report, &rev, &plan.source, &mut out, &mut git_err).unwrap_or(1) == 0 {
        let line = out.trim();
        if !line.is_empty() {
            report.log(format!("repo en /mnt: {line}"));
        }
    }
}

/// Extracts the path from nix's `error: path '/nix/store/…' does not exist`.
fn missing_path_hint(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        if !line.contains("does not exist") {
            continue;
        }
        if let (Some(a), Some(b)) = (line.find('\''), line.rfind('\'')) {
            if b > a + 1 {
                return Some(line[a + 1..b].to_string());
            }
        }
        return Some(line.trim().to_string());
    }
    None
}

fn ensure_ok(code: i32, what: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(eyre!("{what} (código {code})"))
    }
}

/// Translates nix's `internal-json` log into clean lines plus a global build
/// progress. Activity types: 103 CopyPaths, 104 Builds, 105 Build, 108
/// Substitute. Only a `result` 105 of a Builds/CopyPaths activity is real
/// progress (fields = [done, expected]); a download's `result` 105 is bytes.
#[derive(Default)]
struct NixLog {
    activities: std::collections::HashMap<u64, u64>,
}

/// [`NixLog::feed`]'s result: display lines (each with an error flag) plus the
/// optional `(built, total)` progress.
type NixLogOut = (Vec<(bool, String)>, Option<(u64, u64)>);

impl NixLog {
    /// Returns the display lines (each with an error flag) and the progress.
    fn feed(&mut self, line: &str) -> NixLogOut {
        let line = line.trim_end();
        let Some(json) = line.strip_prefix("@nix ") else {
            return (vec![(false, strip_ansi(line))], None);
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
            return (Vec::new(), None);
        };
        let id = v.get("id").and_then(|i| i.as_u64());
        let ty = v.get("type").and_then(|t| t.as_u64()).unwrap_or(0);
        let fields = v.get("fields").and_then(|f| f.as_array());
        let field = |i: usize| -> Option<String> {
            fields
                .and_then(|f| f.get(i))
                .and_then(|s| s.as_str())
                .map(str::to_string)
        };
        match v.get("action").and_then(|a| a.as_str()) {
            Some("start") => {
                if let Some(id) = id {
                    self.activities.insert(id, ty);
                }
                let disp = match ty {
                    105 => field(0).map(|p| format!("compilando {}", store_name(&p))),
                    108 => field(0).map(|p| {
                        format!(
                            "descargando {} · {}",
                            store_name(&p),
                            field(1).unwrap_or_default()
                        )
                    }),
                    _ => None,
                };
                (disp.map(|d| vec![(false, d)]).unwrap_or_default(), None)
            }
            Some("result") if ty == 105 => {
                if let Some(id) = id {
                    if matches!(self.activities.get(&id), Some(104) | Some(103)) {
                        let done = field(0).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        let expected = field(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                        if expected > 0 {
                            return (Vec::new(), Some((done, expected)));
                        }
                    }
                }
                (Vec::new(), None)
            }
            Some("msg") => {
                // Levels: 0 error, 1 warn, 2 notice, 3 info; 4+ is noise.
                let level = v.get("level").and_then(|l| l.as_u64()).unwrap_or(3);
                if level > 3 {
                    return (Vec::new(), None);
                }
                let Some(text) = v
                    .get("msg")
                    .or_else(|| v.get("text"))
                    .and_then(|t| t.as_str())
                else {
                    return (Vec::new(), None);
                };
                let is_error = level == 0;
                let lines = text
                    .lines()
                    .map(|l| (is_error, strip_ansi(l)))
                    .collect();
                (lines, None)
            }
            _ => (Vec::new(), None),
        }
    }
}

/// The `-name` part of a `/nix/store/<hash>-<name>` path.
fn store_name(p: &str) -> String {
    let base = p.rsplit('/').next().unwrap_or(p);
    match base.split_once('-') {
        Some((hash, name)) if hash.len() == 32 => name.to_string(),
        _ => base.to_string(),
    }
}

/// Removes ANSI CSI escape sequences (`\x1b[…m` and friends).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
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

    // Exact bytes on stdin, no trailing newline: `--stdin` reads the secret
    // verbatim (EOF terminates it), so the hash matches what was typed.
    child
        .stdin
        .take()
        .ok_or_else(|| eyre!("mkpasswd no aceptó la entrada"))?
        .write_all(password.as_bytes())?;

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
    let dest_dir = plan.root.join("var/log");
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

/// Markers around the installer-managed encryption block in the generated
/// `hardware-configuration.nix`.
const ENC_BLOCK_BEGIN: &str = "# >>> nixdots-installer: cifrado (no editar a mano)";
const ENC_BLOCK_END: &str = "# <<< nixdots-installer";

/// Writes or removes the installer-managed encryption block in the generated
/// `hardware-configuration.nix`:
///
/// ```nix
/// # >>> nixdots-installer: cifrado (no editar a mano)
/// boot.initrd.luks.devices.cryptroot.device = "/dev/disk/by-uuid/<UUID>";
/// nixdots.encryption.enable = true;
/// nixdots.encryption.tpmUnlock = <true|false>;
/// # <<< nixdots-installer
/// ```
///
/// The whole block is replaced on every run so it never duplicates, and any
/// stray `cryptroot` entry left by `nixos-generate-config` is folded into it.
/// Without encryption no block is written and a previous one is removed.
/// Returns the updated text and whether it changed.
fn apply_encryption_block(
    mut text: String,
    encrypt: bool,
    uuid: Option<&str>,
    tpm: bool,
) -> (String, bool) {
    let before = text.clone();

    // Drop the block written by a previous run (markers included).
    if let Some(start) = text.find(ENC_BLOCK_BEGIN) {
        let end = match text[start..].find(ENC_BLOCK_END) {
            Some(rel) => start + rel + ENC_BLOCK_END.len(),
            None => text.len(),
        };
        // Swallow the trailing newline too, so no blank lines accumulate.
        let end = text[end..].find('\n').map(|rel| end + rel + 1).unwrap_or(end);
        text.replace_range(start..end, "");
    }

    // Fold a stray cryptroot entry into the block instead of duplicating it.
    text = remove_stray_cryptroot(text);

    if encrypt {
        let device = uuid.unwrap_or("<UUID>");
        let block = format!(
            "  {ENC_BLOCK_BEGIN}\n  boot.initrd.luks.devices.cryptroot.device = \"/dev/disk/by-uuid/{device}\";\n  nixdots.encryption.enable = true;\n  nixdots.encryption.tpmUnlock = {tpm};\n  {ENC_BLOCK_END}\n"
        );
        match text.rfind("\n}") {
            Some(pos) => text.insert_str(pos, &format!("\n{block}")),
            None => text.push_str(&block),
        }
    }

    let changed = text != before;
    (text, changed)
}

/// Removes single-line `boot.initrd.luks.devices...cryptroot... = "...";`
/// entries (e.g. written by `nixos-generate-config`), leaving everything else.
fn remove_stray_cryptroot(text: String) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        let stray = trimmed.contains("boot.initrd.luks.devices")
            && trimmed.contains("cryptroot")
            && trimmed.contains('=')
            && trimmed.ends_with(';');
        if !stray {
            out.push_str(line);
        }
    }
    out
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
    fn steps_have_labels() {
        assert_eq!(Step::ALL.len(), 4);
        assert_eq!(Step::Disk.label(), "disco");
        assert_eq!(Step::Settings.label(), "ajustes");
        assert_eq!(Step::System.label(), "sistema");
        assert_eq!(Step::Boot.label(), "arranque");
    }

    #[test]
    fn writes_encryption_block_when_encrypted() {
        let cfg = "{ config, lib, pkgs, modulesPath, ... }:\n\n{\n  imports = [ ];\n}\n";
        let (out, changed) = apply_encryption_block(cfg.to_string(), true, Some("abcd-1234"), true);
        assert!(changed);
        assert!(out.contains(ENC_BLOCK_BEGIN));
        assert!(out.contains(ENC_BLOCK_END));
        assert!(out
            .contains("boot.initrd.luks.devices.cryptroot.device = \"/dev/disk/by-uuid/abcd-1234\";"));
        assert!(out.contains("nixdots.encryption.enable = true;"));
        assert!(out.contains("nixdots.encryption.tpmUnlock = true;"));
        // Still valid: exactly one closing brace line at the end.
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn folds_stray_cryptroot_into_the_block() {
        let cfg =
            "{\n  boot.initrd.luks.devices.\"cryptroot\".device = \"/dev/disk/by-uuid/old\";\n}\n";
        let (out, changed) =
            apply_encryption_block(cfg.to_string(), true, Some("abcd-1234"), false);
        assert!(changed);
        assert!(!out.contains("by-uuid/old"));
        assert_eq!(out.matches("boot.initrd.luks.devices").count(), 1);
        assert!(out.contains("tpmUnlock = false;"));
    }

    #[test]
    fn removes_block_without_encryption() {
        let cfg = "{\n  # >>> nixdots-installer: cifrado (no editar a mano)\n  nixdots.encryption.enable = true;\n  # <<< nixdots-installer\n}\n";
        let (out, changed) = apply_encryption_block(cfg.to_string(), false, None, false);
        assert!(changed);
        assert!(!out.contains("nixdots.encryption.enable"));
        assert!(out.trim_end().ends_with('}'));
    }
}
