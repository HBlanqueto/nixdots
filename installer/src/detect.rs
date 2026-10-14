//! Environment detection used by the welcome screen and the disk chooser.

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

    /// Parenthesised description shown on the Equipo screen.
    pub fn description(self) -> &'static str {
        match self {
            Profile::Thinkpad => "x86_64",
            Profile::Utm => "máquina virtual aarch64",
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

/// One physical disk, as reported by `lsblk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub name: String,
    pub size: u64,
    pub model: String,
    /// What is on the disk: "Windows 11", "vacío", "datos"…
    pub note: String,
    /// `false` for the install medium (or anything else not selectable).
    pub available: bool,
}

impl Disk {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn size_human(&self) -> String {
        human_size(self.size)
    }

    pub fn is_windows(&self) -> bool {
        self.note.starts_with("Windows")
    }
}

/// `lsblk -J -b -d -o NAME,SIZE,MODEL,TYPE`, minus the disk the live ISO boots
/// from (and the virtual/no-install devices).
pub fn disks() -> Vec<Disk> {
    let Some(json) = run_trim(
        "lsblk",
        &["-J", "-b", "-o", "NAME,SIZE,MODEL,TYPE,MOUNTPOINT,PKNAME,FSTYPE"],
    ) else {
        return Vec::new();
    };
    parse_disks(&json)
}

/// Whether an NTFS filesystem (a likely Windows install) is present.
pub fn windows_present() -> bool {
    let Some(json) = run_trim("lsblk", &["-J", "-o", "FSTYPE"]) else {
        return false;
    };
    let lower = json.to_lowercase();
    lower.contains("\"ntfs\"")
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
    fstype: Option<String>,
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
        .map(|d| Disk {
            note: disk_note(&d),
            available: !holds_live_medium(&d),
            model: d.model.clone().unwrap_or_default().trim().to_string(),
            size: d.size.unwrap_or(0),
            name: d.name.clone(),
        })
        .collect()
}

/// zram/loop/sr/ram and friends are never install targets.
fn is_virtual(name: &str) -> bool {
    ["zram", "loop", "sr", "ram", "fd", "dm-"]
        .iter()
        .any(|p| name.starts_with(p))
}

fn disk_note(node: &Node) -> String {
    fn has_ntfs(node: &Node) -> bool {
        node.fstype.as_deref() == Some("ntfs")
            || node.children.iter().any(|c| c.fstype.as_deref() == Some("ntfs"))
    }
    if has_ntfs(node) {
        "Windows 11".to_string()
    } else if node.children.is_empty() {
        "vacío".to_string()
    } else {
        "datos".to_string()
    }
}

fn holds_live_medium(node: &Node) -> bool {
    if matches!(
        node.mountpoint.as_deref(),
        Some("/") | Some("/iso") | Some("/run/iso") | Some("/nix")
    ) {
        return true;
    }
    node.children.iter().any(holds_live_medium)
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

/// Human-readable size, matching `lsblk`'s style closely enough.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
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
