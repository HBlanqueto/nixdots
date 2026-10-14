//! Environment detection used by the welcome screen and the disk chooser.

use serde::Deserialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Runs a command and returns its trimmed stdout, or `None` if it failed.
pub fn run_trim(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
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
}

impl Disk {
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }

    pub fn size_human(&self) -> String {
        human_size(self.size)
    }
}

/// `lsblk -J -b -d -o NAME,SIZE,MODEL,TYPE`, minus the disk the live ISO boots
/// from.
pub fn disks() -> Vec<Disk> {
    let Some(json) = run_trim(
        "lsblk",
        &["-J", "-b", "-o", "NAME,SIZE,MODEL,TYPE,MOUNTPOINT,PKNAME"],
    ) else {
        return Vec::new();
    };
    parse_disks(&json)
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
    children: Vec<Node>,
}

fn parse_disks(json: &str) -> Vec<Disk> {
    let Ok(tree) = serde_json::from_str::<Lsblk>(json) else {
        return Vec::new();
    };
    tree.blockdevices
        .into_iter()
        .filter(|d| d.kind.as_deref() == Some("disk"))
        .filter(|d| !holds_live_medium(d))
        .map(|d| Disk {
            model: d.model.unwrap_or_default().trim().to_string(),
            size: d.size.unwrap_or(0),
            name: d.name,
        })
        .collect()
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
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].name, "nvme0n1");
    }
}
