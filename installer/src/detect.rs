//! Environment detection used for the default hostname and the disk chooser.

use serde::Deserialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Runs a command and returns its trimmed stdout, or `None` if it failed.
pub fn run_trim(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// `uname -m`, e.g. `x86_64` or `aarch64`.
pub fn arch() -> Option<String> {
    run_trim("uname", &["-m"])
}

/// Nix system string (`<arch>-linux`) derived from [`arch`].
pub fn nix_system() -> Option<String> {
    let arch = arch()?;
    let arch = match arch.as_str() {
        "amd64" => "x86_64",
        other => other,
    };
    Some(format!("{arch}-linux"))
}

/// Default hostname (also the `nixosConfigurations` attribute) for an
/// architecture. aarch64 is assumed to be the UTM VM, x86_64 the ThinkPad.
pub fn default_hostname(arch: &str) -> String {
    match arch {
        "aarch64" | "arm64" => "UTM-aarch64".to_string(),
        "x86_64" | "amd64" => "ThinkPad-L14-Gen2".to_string(),
        other => other.to_string(),
    }
}

/// The two machine classes the flake declares (`hosts/<profile>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Thinkpad,
    Utm,
}

impl Profile {
    /// Detected from the architecture, corroborated by `systemd-detect-virt`.
    pub fn detect() -> Profile {
        let arch = arch().unwrap_or_default();
        if arch == "aarch64" || arch == "arm64" {
            Profile::Utm
        } else if arch == "x86_64" || arch == "amd64" {
            Profile::Thinkpad
        } else {
            // Unknown arch: a qemu/utm guest is the VM profile, else the
            // physical one.
            match run_trim("systemd-detect-virt", &[]).as_deref() {
                Some("qemu") | Some("kvm") => Profile::Utm,
                _ => Profile::Thinkpad,
            }
        }
    }

    /// Directory name under `hosts/`, and the flake output name of the profile.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Thinkpad => "thinkpad",
            Profile::Utm => "utm",
        }
    }

    pub fn system(self) -> &'static str {
        match self {
            Profile::Thinkpad => "x86_64-linux",
            Profile::Utm => "aarch64-linux",
        }
    }

    pub fn default_hostname(self) -> &'static str {
        match self {
            Profile::Thinkpad => "ThinkPad-L14-Gen2",
            Profile::Utm => "UTM-aarch64",
        }
    }
}

/// Derives a username from a display name: strip accents, lowercase, keep
/// `a-z0-9`, prefix `u` when it would start with a digit, cap at 32 chars.
pub fn generate_username(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        let base = deaccent(ch);
        if base.is_ascii_alphanumeric() {
            out.push(base.to_ascii_lowercase());
        }
    }
    if out.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        out.insert(0, 'u');
    }
    out.truncate(32);
    out
}

/// Maps the common accented Latin letters to their unaccented ASCII form
/// (a dependency-free stand-in for NFD + strip-combining-marks).
fn deaccent(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => 'e',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' | 'ī' | 'ĩ' => 'i',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ō' => 'o',
        'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => 'O',
        'ú' | 'ù' | 'û' | 'ü' | 'ū' | 'ũ' => 'u',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ñ' => 'n',
        'Ñ' => 'N',
        'ç' => 'c',
        'Ç' => 'C',
        'ý' | 'ÿ' => 'y',
        'Ý' => 'Y',
        _ => ch,
    }
}

/// `systemd-detect-virt`. Maps `qemu` to the friendlier «UTM (qemu)» label the
/// installer shows when it runs inside a UTM/QEMU guest.
pub fn environment() -> Option<String> {
    let virt = run_trim("systemd-detect-virt", &[])?;
    match virt.as_str() {
        "qemu" => Some("UTM (qemu)".to_string()),
        _ => Some(virt),
    }
}

/// The machine's commercial model, macOS-style: «Máquina virtual», the DMI
/// product name (or Lenovo's product version), the ARM device-tree model, or
/// «Laptop»/«PC» as a last resort. Detected in the background at startup.
pub fn machine_model() -> String {
    let arch = arch().unwrap_or_default();
    let is_arm = arch.starts_with("arm") || arch == "aarch64";

    if let Some(virt) = run_trim("systemd-detect-virt", &[]) {
        if virt != "none" {
            return "Máquina virtual".to_string();
        }
    }

    let mut vendor = String::new();
    let mut candidate = None;
    if !is_arm {
        vendor = dmi_field("sys_vendor").unwrap_or_default();
        let product = dmi_field("product_name");
        let version = dmi_field("product_version");
        // Lenovo's product_name is a code (20X5…); the version is the real name.
        candidate = if vendor.to_lowercase().contains("lenovo") {
            version.or(product)
        } else {
            product.or(version)
        };
    }
    if candidate.is_none() {
        candidate = device_tree_model();
    }

    let model = candidate.unwrap_or_else(|| {
        if battery_present() {
            "Laptop".to_string()
        } else {
            "PC".to_string()
        }
    });
    simplify_model(&model, &vendor)
}

fn dmi_field(name: &str) -> Option<String> {
    let text = std::fs::read_to_string(format!("/sys/class/dmi/id/{name}")).ok()?;
    let value = text.trim();
    if value.is_empty() || is_placeholder(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn device_tree_model() -> Option<String> {
    let text = std::fs::read_to_string("/proc/device-tree/model").ok()?;
    let value = text.trim_end_matches('\0').trim();
    if value.is_empty() || is_placeholder(value) {
        None
    } else {
        Some(value.to_string())
    }
}

/// Firmware placeholders that are not a real model name.
fn is_placeholder(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "to be filled by o.e.m."
            | "system product name"
            | "default string"
            | "none"
            | "not applicable"
    )
}

fn battery_present() -> bool {
    std::fs::read_dir("/sys/class/power_supply")
        .map(|entries| {
            entries.flatten().any(|entry| {
                entry.file_name().to_string_lossy().starts_with("BAT")
            })
        })
        .unwrap_or(false)
}

/// Shortens a model the way macOS does: drop a trailing generation, drop the
/// vendor when it repeats at the start, and collapse whitespace.
fn simplify_model(model: &str, vendor: &str) -> String {
    let mut model = model.trim().to_string();
    if let Some(idx) = model.rfind(" Gen ") {
        let tail = &model[idx + 5..];
        if !tail.is_empty() && tail.chars().all(|c| c.is_alphanumeric()) {
            model.truncate(idx);
        }
    }
    if !vendor.is_empty() && model.to_lowercase().starts_with(&vendor.to_lowercase()) {
        let rest = model[vendor.len()..].trim_start();
        if !rest.is_empty() {
            model = rest.to_string();
        }
    }
    model.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// True when something answers on `cache.nixos.org:443`.
pub fn network_online() -> bool {
    let Ok(addrs) = ("cache.nixos.org", 443).to_socket_addrs() else {
        return false;
    };
    addrs.into_iter().any(|addr| {
        TcpStream::connect_timeout(&addr, Duration::from_secs(4)).is_ok()
    })
}

/// The NixOS ISO drops `/etc/NIXOS`; a normal install does not have it.
pub fn is_live_environment() -> bool {
    Path::new("/etc/NIXOS").exists() || Path::new("/iso").exists()
}

/// Effective UID, read straight from `/proc` so no extra crate is needed.
pub fn euid() -> u32 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(real) = rest.split_whitespace().nth(1) {
                    return real.parse().unwrap_or(1);
                }
            }
        }
    }
    1
}

pub fn is_root() -> bool {
    euid() == 0
}

/// One partition of a disk, as reported by `lsblk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub size: u64,
    pub fstype: String,
    pub label: String,
    /// GPT partition type GUID (or MBR type), from `PARTTYPE`.
    pub parttype: String,
    pub mountpoints: Vec<String>,
}

impl Partition {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    /// The EFI System Partition (GPT type GUID).
    pub fn is_efi(&self) -> bool {
        self.parttype.eq_ignore_ascii_case("c12a7328-f81f-11d2-ba4b-00a0c93ec93b")
    }

    pub fn is_ntfs(&self) -> bool {
        self.fstype.eq_ignore_ascii_case("ntfs")
    }

    pub fn is_linux(&self) -> bool {
        matches!(
            self.fstype.as_str(),
            "ext4" | "btrfs" | "xfs" | "ext3" | "f2fs" | "crypto_LUKS"
        )
    }
}

/// One physical disk, as reported by `lsblk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub name: String,
    pub size: u64,
    pub model: String,
    /// What is on the disk: "Windows 11", "vacío", "Linux", "datos"…
    pub note: String,
    /// `false` for the install medium (or anything else not selectable).
    pub available: bool,
    /// A Windows install was detected (NTFS, or an EFI with `EFI/Microsoft`).
    pub windows: bool,
    /// Unallocated space (approximate: disk size minus its partitions).
    pub free: u64,
    pub partitions: Vec<Partition>,
}

impl Disk {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn size_human(&self) -> String {
        human_size(self.size)
    }

    pub fn is_windows(&self) -> bool {
        self.windows
    }

    /// "SAMSUNG 256 GB": the first model word plus the nominal capacity.
    pub fn short_name(&self) -> String {
        let model = self
            .model
            .split_whitespace()
            .next()
            .unwrap_or("Disco")
            .chars()
            .take(11)
            .collect::<String>();
        format!("{model} {} GB", nominal_gb(self.size))
    }

    /// Nominal capacity in GB (marketing size), e.g. 238.5 GiB → 256.
    pub fn nominal_gb(&self) -> u64 {
        nominal_gb(self.size)
    }

    /// The EFI System Partition of the disk, if any.
    pub fn efi(&self) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.is_efi())
    }

    /// The NTFS partitions, largest first.
    pub fn ntfs(&self) -> Vec<&Partition> {
        let mut ntfs: Vec<&Partition> = self.partitions.iter().filter(|p| p.is_ntfs()).collect();
        ntfs.sort_by_key(|p| std::cmp::Reverse(p.size));
        ntfs
    }

    /// Why this disk cannot be used, if it cannot: the install medium, a disk
    /// in use by the running system, or one too small for nixdots.
    pub fn unusable_reason(&self) -> Option<&'static str> {
        if self
            .partitions
            .iter()
            .any(|p| p.mountpoints.iter().any(|m| is_live_mount(m)))
        {
            return Some("Es el medio de instalación");
        }
        if !self.available {
            return Some("En uso por el sistema actual");
        }
        if self.size < crate::disk::MIN_GIB * crate::disk::GIB {
            return Some("Demasiado pequeño (mín. 40 GB)");
        }
        None
    }
}

/// A partition that is mounted by the running system (`/boot` or swap).
fn partition_in_use(partition: &Partition) -> bool {
    partition.fstype == "swap"
        || partition
            .mountpoints
            .iter()
            .any(|m| m == "/boot" || m.starts_with("/boot/"))
}

/// `lsblk -J -b -o NAME,SIZE,MODEL,TYPE,FSTYPE,PARTTYPE,LABEL,MOUNTPOINTS`,
/// minus the disk the live ISO boots from (and the virtual/no-install devices).
pub fn disks() -> Vec<Disk> {
    let Some(json) = run_trim(
        "lsblk",
        &[
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,MODEL,TYPE,FSTYPE,PARTTYPE,LABEL,MOUNTPOINTS",
        ],
    ) else {
        return Vec::new();
    };
    parse_disks(&json)
}

/// Whether an NTFS filesystem (a likely Windows install) is present.
pub fn windows_present() -> bool {
    disks().iter().any(|d| d.windows)
}

#[derive(Deserialize)]
struct Lsblk {
    #[serde(default)]
    blockdevices: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    model: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    mountpoint: Option<String>,
    #[serde(default)]
    mountpoints: Option<Vec<String>>,
    #[serde(default)]
    fstype: Option<String>,
    #[serde(default)]
    parttype: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    children: Vec<Node>,
}

fn parse_disks(json: &str) -> Vec<Disk> {
    let Ok(tree) = serde_json::from_str::<Lsblk>(json) else {
        return Vec::new();
    };
    tree.blockdevices
        .into_iter()
        .filter(|d| d.kind.as_deref() == Some("disk"))
        .filter(|d| !is_virtual(&d.name))
        .map(|d| {
            let partitions: Vec<Partition> = d
                .children
                .iter()
                .filter(|c| c.kind.as_deref() == Some("part"))
                .map(node_partition)
                .collect();
            let used: u64 = partitions.iter().map(|p| p.size).sum();
            let size = d.size.unwrap_or(0);
            let windows = partitions.iter().any(Partition::is_ntfs);
            let note = disk_note(&partitions);
            Disk {
                note,
                available: !holds_live_medium(&d)
                    && !partitions.iter().any(partition_in_use),
                windows,
                // GPT reserves ~1 MiB; ignore it in the estimate.
                free: size.saturating_sub(used).saturating_sub(1024 * 1024),
                model: d.model.clone().unwrap_or_default().trim().to_string(),
                size,
                name: d.name.clone(),
                partitions,
            }
        })
        .collect()
}

fn node_partition(node: &Node) -> Partition {
    let mut mountpoints = node.mountpoints.clone().unwrap_or_default();
    if let Some(mp) = &node.mountpoint {
        if !mp.is_empty() && !mountpoints.contains(mp) {
            mountpoints.push(mp.clone());
        }
    }
    Partition {
        name: node.name.clone(),
        size: node.size.unwrap_or(0),
        fstype: node.fstype.clone().unwrap_or_default(),
        label: node.label.clone().unwrap_or_default(),
        parttype: node.parttype.clone().unwrap_or_default(),
        mountpoints,
    }
}

/// zram/loop/sr/ram and friends are never install targets.
fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "sr", "ram", "fd", "dm-"]
        .iter()
        .any(|p| name.starts_with(p))
}

fn disk_note(partitions: &[Partition]) -> String {
    if partitions.iter().any(Partition::is_ntfs) {
        "Windows 11".to_string()
    } else if partitions.is_empty() {
        "vacío".to_string()
    } else if partitions.iter().any(Partition::is_linux) {
        "Linux".to_string()
    } else {
        "datos".to_string()
    }
}

fn holds_live_medium(node: &Node) -> bool {
    if node
        .mountpoints
        .as_ref()
        .is_some_and(|m| m.iter().any(|p| is_live_mount(p)))
    {
        return true;
    }
    if matches!(
        node.mountpoint.as_deref(),
        Some("/") | Some("/iso") | Some("/run/iso") | Some("/nix")
    ) {
        return true;
    }
    node.children.iter().any(holds_live_medium)
}

fn is_live_mount(path: &str) -> bool {
    matches!(path, "/" | "/iso" | "/run/iso" | "/nix")
}

/// Result of probing an NTFS partition before resizing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtfsInfo {
    /// Minimal volume size `ntfsresize` allows, in bytes (0 when unknown).
    pub min_bytes: u64,
    /// The volume is BitLocker-encrypted (cannot be resized).
    pub bitlocker: bool,
    /// The volume is dirty (hibernated or Fast Startup).
    pub dirty: bool,
}

/// Probes an NTFS partition with `blkid` and `ntfsresize --info --force`.
pub fn ntfs_info(path: &str) -> NtfsInfo {
    let bitlocker = run_trim("blkid", &["-o", "value", "-s", "TYPE", path]).as_deref()
        == Some("BitLocker");
    let output = Command::new("ntfsresize")
        .args(["--info", "--force", path])
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let dirty = text.to_lowercase().contains("scheduled for check")
                || text.to_lowercase().contains("hibernat");
            NtfsInfo {
                min_bytes: parse_ntfs_min(&text).unwrap_or(0),
                bitlocker,
                dirty,
            }
        }
        // A failing `--info` means a dirty/hibernated volume.
        _ => NtfsInfo {
            min_bytes: 0,
            bitlocker,
            dirty: true,
        },
    }
}

/// The `You might resize at N bytes` value from `ntfsresize --info`.
fn parse_ntfs_min(text: &str) -> Option<u64> {
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find("resize at") {
            let rest = &line[pos + "resize at".len()..];
            let digits: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                return Some(value);
            }
        }
    }
    None
}

/// Free bytes on the disk's EFI System Partition, by mounting it read-only.
/// `None` when it cannot be determined.
pub fn efi_free(efi: &Partition) -> Option<u64> {
    if let Some(mp) = efi.mountpoints.iter().find(|m| !m.is_empty()) {
        return df_free(mp);
    }
    let dir = std::env::temp_dir().join("nixdots-efi");
    let _ = std::fs::create_dir_all(&dir);
    let mounted = Command::new("mount")
        .args(["-o", "ro"])
        .arg(efi.path())
        .arg(&dir)
        .stdin(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !mounted {
        return None;
    }
    let free = df_free(&dir.to_string_lossy());
    let _ = Command::new("umount").arg(&dir).stdin(Stdio::null()).status();
    free
}

fn df_free(path: &str) -> Option<u64> {
    let output = Command::new("df")
        .args(["-B1", "--output=avail"])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .and_then(|line| line.trim().parse::<u64>().ok())
}

/// The LUKS2 partition disko created inside the target disk, with its UUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuksPartition {
    /// Real device node, e.g. `/dev/vda2` (never a label: udev escapes spaces
    /// and disko defaults the GPT label to `disk-<disk>-<name>`).
    pub path: String,
    /// LUKS UUID, for the installed system's `boot.initrd.luks.devices`.
    pub uuid: String,
}

/// Finds the `crypto_LUKS` partition of `disk` and returns its real device path
/// and UUID. `disk` is the install target as picked on the Disco screen.
pub fn luks_partition(disk: &str) -> Option<LuksPartition> {
    let json = run_trim("lsblk", &["-J", "-o", "NAME,TYPE,FSTYPE,UUID", disk])?;
    let mut part = parse_luks(&json)?;
    if part.uuid.is_empty() {
        part.uuid = run_trim("blkid", &["-s", "UUID", "-o", "value", &part.path])?;
    }
    Some(part)
}

fn parse_luks(json: &str) -> Option<LuksPartition> {
    let tree = serde_json::from_str::<Lsblk>(json).ok()?;
    let node = tree.blockdevices.into_iter().next()?;
    let part = node
        .children
        .into_iter()
        .find(|c| c.fstype.as_deref() == Some("crypto_LUKS"))?;
    Some(LuksPartition {
        path: format!("/dev/{}", part.name),
        uuid: part.uuid.unwrap_or_default(),
    })
}

/// True when a TPM 2.0 chip is present (`/sys/class/tpm/tpm0`, major version 2).
pub fn tpm2_present() -> bool {
    let dir = Path::new("/sys/class/tpm/tpm0");
    if !dir.exists() {
        return false;
    }
    match std::fs::read_to_string(dir.join("tpm_version_major")) {
        Ok(version) => version.trim() == "2",
        // The attribute is not on every kernel; a tpm0 device is close enough.
        Err(_) => true,
    }
}

/// A mountable removable medium offered by the Cifrado screen's "guardar en
/// USB" list.
#[derive(Debug, Clone)]
pub struct RemovableMedium {
    pub path: String,
    pub label: String,
}

/// Removable/USB partitions that can hold the recovery key, excluding the
/// install target and the live ISO.
pub fn removable_media(target: &str) -> Vec<RemovableMedium> {
    let target_name = target.rsplit('/').next().unwrap_or("").to_string();
    let Some(json) = run_trim(
        "lsblk",
        &["-J", "-o", "NAME,PATH,RM,TRAN,TYPE,FSTYPE,LABEL,SIZE,PKNAME"],
    ) else {
        return Vec::new();
    };
    let Ok(tree) = serde_json::from_str::<serde_json::Value>(&json) else {
        return Vec::new();
    };

    let truthy = |value: &serde_json::Value| {
        value.as_bool().unwrap_or_else(|| value.as_str() == Some("1"))
    };

    let mut out = Vec::new();
    let Some(disks) = tree.get("blockdevices").and_then(|d| d.as_array()) else {
        return out;
    };
    for disk in disks {
        let disk_name = disk.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if disk_name == target_name {
            continue;
        }
        let removable = disk
            .get("rm")
            .map(&truthy)
            .unwrap_or(false)
            || disk.get("tran").and_then(|v| v.as_str()) == Some("usb");
        if !removable {
            continue;
        }
        let Some(parts) = disk.get("children").and_then(|c| c.as_array()) else {
            continue;
        };
        for part in parts {
            if part.get("type").and_then(|v| v.as_str()) != Some("part") {
                continue;
            }
            let path = part.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if path.is_empty() {
                continue;
            }
            let Some(fstype) = part.get("fstype").and_then(|v| v.as_str()) else {
                continue;
            };
            if fstype.is_empty() || fstype == "iso9660" {
                continue;
            }
            let name = part.get("name").and_then(|v| v.as_str()).unwrap_or(path);
            let label = part
                .get("label")
                .and_then(|v| v.as_str())
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .unwrap_or_else(|| name.to_string());
            let size = part
                .get("size")
                .and_then(|v| v.as_u64())
                .map(human_size)
                .unwrap_or_default();
            out.push(RemovableMedium {
                path: path.to_string(),
                label: format!("{label}  {size}"),
            });
        }
    }
    out
}

/// Nominal capacity in GB (marketing size), e.g. 238.5 GiB → 256.
pub fn nominal_gb(bytes: u64) -> u64 {
    (bytes as f64 / 1_000_000_000.0).round() as u64
}

/// Human-readable size, matching `lsblk`'s style closely enough.
pub fn human_size(bytes: u64) -> String {    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1024), "1.0 K");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0 G");
    }

    #[test]
    fn hostname_defaults_per_arch() {
        assert_eq!(default_hostname("aarch64"), "UTM-aarch64");
        assert_eq!(default_hostname("arm64"), "UTM-aarch64");
        assert_eq!(default_hostname("x86_64"), "ThinkPad-L14-Gen2");
        assert_eq!(default_hostname("amd64"), "ThinkPad-L14-Gen2");
        assert_eq!(default_hostname("riscv64"), "riscv64");
    }

    #[test]
    fn disks_skip_live_medium() {
        let json = r#"{
            "blockdevices": [
                { "name": "sda", "size": 8000000000, "model": "USB", "type": "disk",
                  "mountpoint": null, "pkname": null,
                  "children": [ { "name": "sda1", "size": 7000000000, "type": "part",
                                  "mountpoint": "/iso", "pkname": "sda", "model": null,
                                  "children": [] } ] },
                { "name": "nvme0n1", "size": 500000000000, "model": "NVMe", "type": "disk",
                  "mountpoint": null, "pkname": null, "children": [] }
            ]
        }"#;
        let disks = parse_disks(json);
        // The live medium is kept but flagged as not selectable.
        assert_eq!(disks.len(), 2);
        let live = disks.iter().find(|d| d.name == "sda").unwrap();
        assert!(!live.available);
        let target = disks.iter().find(|d| d.name == "nvme0n1").unwrap();
        assert!(target.available);
    }

    #[test]
    fn finds_crypto_luks_partition_by_type() {
        let json = r#"{
            "blockdevices": [
                { "name": "vda", "type": "disk", "fstype": null, "uuid": null,
                  "children": [
                    { "name": "vda1", "type": "part", "fstype": "vfat", "uuid": "B7C1-02A5",
                      "children": [] },
                    { "name": "vda2", "type": "part", "fstype": "crypto_LUKS",
                      "uuid": "1234abcd-0000-0000-0000-000000000000", "children": [] }
                  ] }
            ]
        }"#;
        let part = parse_luks(json).unwrap();
        assert_eq!(part.path, "/dev/vda2");
        assert_eq!(part.uuid, "1234abcd-0000-0000-0000-000000000000");
    }

    #[test]
    fn no_luks_partition_is_none() {
        let json = r#"{
            "blockdevices": [
                { "name": "vda", "type": "disk", "children": [
                    { "name": "vda1", "type": "part", "fstype": "vfat", "children": [] } ] }
            ]
        }"#;
        assert!(parse_luks(json).is_none());
    }
}
