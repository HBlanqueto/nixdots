//! Disk helpers: enumerating candidate disks and evaluating the disko layout.

use color_eyre::eyre::{eyre, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;


/// One partition of the disko layout (from evaluating `disko.nix`).
#[derive(Debug, Clone, Deserialize)]
pub struct Partition {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub mountpoint: Option<String>,
    #[serde(default)]
    pub subvolumes: BTreeMap<String, Subvolume>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Subvolume {
    #[serde(default)]
    pub mountpoint: Option<String>,
}

impl Subvolume {
    /// Only `@rootfs` is restored on every boot.
    pub fn is_ephemeral(&self, name: &str) -> bool {
        name == "@rootfs"
    }

    pub fn is_persistent(&self, name: &str) -> bool {
        name == "@persist"
    }
}

/// A named partition (`ESP`, `btrfs`, ...).
#[derive(Debug, Clone)]
pub struct NamedPartition {
    pub name: String,
    pub partition: Partition,
}

/// Evaluates `disko.nix` for `device` and returns its partitions, ESP first.
pub fn layout(repo: &Path, device: &str) -> Result<Vec<NamedPartition>> {
    if !is_safe_device(device) {
        return Err(eyre!("dispositivo inesperado: {device}"));
    }

    let expr = r#"
{ deviceArg }:
let
  cfg = import ./disko.nix { device = deviceArg; };
  parts = cfg.disko.devices.disk.main.content.partitions;
in
builtins.mapAttrs (name: part:
  let
    c = part.content or {};
    # The data partition is wrapped in a LUKS container: look one level down
    # so the btrfs subvolumes (and their mountpoints) still show up.
    inner = if (c.type or "") == "luks" then (c.content or {}) else c;
  in
  {
    # Report the inner filesystem type (btrfs/filesystem), so the disk screen
    # keeps drawing the btrfs subvolumes instead of a bare LUKS leaf.
    type = inner.type or (c.type or null);
    size = part.size or null;
    format = inner.format or null;
    mountpoint = inner.mountpoint or null;
    subvolumes =
      if (inner.type or "") == "btrfs"
      then builtins.mapAttrs (sv: s: { mountpoint = s.mountpoint or null; }) inner.subvolumes
      else {};
  }
) parts
"#;

    let output = std::process::Command::new("nix-instantiate")
        .arg("--extra-experimental-features")
        .arg("nix-command flakes")
        .arg("--eval")
        .arg("--json")
        .arg("--expr")
        .arg(expr)
        .arg("--argstr")
        .arg("deviceArg")
        .arg(device)
        .env("NIX_CONFIG", crate::runner::nix_config())
        .current_dir(repo)
        .stdin(std::process::Stdio::null())
        .output()
        .wrap_err("no se pudo ejecutar nix-instantiate")?;

    if !output.status.success() {
        return Err(eyre!(
            "no se pudo evaluar disko.nix: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let map: BTreeMap<String, Partition> = serde_json::from_slice(&output.stdout)
        .wrap_err("disko.nix no produjo el layout esperado")?;

    let mut parts: Vec<NamedPartition> = map
        .into_iter()
        .map(|(name, partition)| NamedPartition { name, partition })
        .collect();

    // ESP first, then by declared size, then by name.
    parts.sort_by(|a, b| {
        let a_boot = a.partition.mountpoint.as_deref() == Some("/boot");
        let b_boot = b.partition.mountpoint.as_deref() == Some("/boot");
        b_boot
            .cmp(&a_boot)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(parts)
}

/// Static mirror of `disko.nix`, used when it cannot be evaluated.
pub fn fallback_layout() -> Vec<NamedPartition> {
    let mut subvolumes = BTreeMap::new();
    for (name, mountpoint) in [
        ("@rootfs", "/"),
        ("@nix", "/nix"),
        ("@home", "/home"),
        ("@persist", "/persist"),
        ("@log", "/var/log"),
    ] {
        subvolumes.insert(
            name.to_string(),
            Subvolume {
                mountpoint: Some(mountpoint.to_string()),
            },
        );
    }
    vec![
        NamedPartition {
            name: "ESP".into(),
            partition: Partition {
                kind: Some("filesystem".into()),
                size: Some("1G".into()),
                format: Some("vfat".into()),
                mountpoint: Some("/boot".into()),
                subvolumes: BTreeMap::new(),
            },
        },
        NamedPartition {
            name: "btrfs".into(),
            partition: Partition {
                kind: Some("btrfs".into()),
                size: Some("100%".into()),
                format: Some("btrfs".into()),
                mountpoint: None,
                subvolumes,
            },
        },
    ]
}

fn is_safe_device(device: &str) -> bool {
    device.starts_with("/dev/")
        && device
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
}

/// Byte size of a disko size string (`1G`, `500M`, `100%`).
pub fn parse_size(size: &str, total: u64) -> u64 {
    let size = size.trim();
    if let Some(percent) = size.strip_suffix('%') {
        let pct: f64 = percent.parse().unwrap_or(100.0);
        return ((total as f64) * pct / 100.0) as u64;
    }
    let (digits, suffix) = size.split_at(size.find(|c: char| !c.is_ascii_digit()).unwrap_or(size.len()));
    let value: f64 = digits.parse().unwrap_or(0.0);
    let multiplier = match suffix.trim().to_ascii_uppercase().as_str() {
        "K" | "KB" | "KIB" => 1024_u64,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };
    (value * multiplier as f64) as u64
}

/// Proportional block widths (in cells) for a partition bar of `width` cells.
pub fn block_widths(sizes: &[u64], width: usize) -> Vec<usize> {
    let total: u64 = sizes.iter().sum();
    if total == 0 || width == 0 {
        return vec![0; sizes.len()];
    }
    let mut widths: Vec<usize> = sizes
        .iter()
        .map(|&s| ((s as f64 / total as f64) * width as f64).floor() as usize)
        .collect();

    // Hand out the leftover cells to the largest fractions.
    let used: usize = widths.iter().sum();
    let mut leftover = width.saturating_sub(used);
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| sizes[b].cmp(&sizes[a]));
    let mut i = 0;
    while leftover > 0 && !order.is_empty() {
        widths[order[i % order.len()]] += 1;
        leftover -= 1;
        i += 1;
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_strings() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(parse_size("1G", gib * 10), gib);
        assert_eq!(parse_size("500M", gib * 10), 500 * 1024 * 1024);
        assert_eq!(parse_size("100%", gib * 10), gib * 10);
    }

    #[test]
    fn widths_sum_to_total() {
        let widths = block_widths(&[1024, 3072], 40);
        assert_eq!(widths.iter().sum::<usize>(), 40);
        assert!(widths[1] > widths[0]);
    }

    #[test]
    fn device_validation() {
        assert!(is_safe_device("/dev/nvme0n1"));
        assert!(is_safe_device("/dev/disk/by-id/foo_bar"));
        assert!(!is_safe_device("/dev/sda; rm -rf /"));
        assert!(!is_safe_device("sda"));
    }
}
