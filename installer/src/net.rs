//! Network probing and Wi-Fi association.
//!
//! The NixOS minimal ISO enables `networking.networkmanager` (verified in
//! `nixos/modules/profiles/installation-device.nix`), so `nmcli` is what is
//! actually present. The other backends are tried only if `nmcli` is missing.

use color_eyre::eyre::{eyre, Result};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Nmcli,
    Iwd,
    WpaSupplicant,
    None,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Backend::Nmcli => "nmcli",
            Backend::Iwd => "iwctl",
            Backend::WpaSupplicant => "wpa_cli",
            Backend::None => "ninguno",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wifi {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
}

impl Wifi {
    /// Four-step signal bars.
    pub fn bars(&self) -> &'static str {
        match self.signal {
            76..=100 => "▂▄▆█",
            51..=75 => "▂▄▆ ",
            26..=50 => "▂▄  ",
            _ => "▂   ",
        }
    }
}

/// First available backend, in priority order.
pub fn detect_backend() -> Backend {
    for (cmd, backend) in [
        ("nmcli", Backend::Nmcli),
        ("iwctl", Backend::Iwd),
        ("wpa_cli", Backend::WpaSupplicant),
    ] {
        if which(cmd) {
            return backend;
        }
    }
    Backend::None
}

fn which(cmd: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(cmd);
        candidate.is_file()
    })
}

/// A wired interface that is up and has carrier.
pub fn ethernet_connected() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/net") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        let base = entry.path();
        let wireless = base.join("wireless").exists() || base.join("phy80211").exists();
        if wireless {
            continue;
        }
        let operstate = std::fs::read_to_string(base.join("operstate")).unwrap_or_default();
        let carrier = std::fs::read_to_string(base.join("carrier")).unwrap_or_default();
        if operstate.trim() == "up" && carrier.trim() == "1" {
            return true;
        }
    }
    false
}

pub fn online() -> bool {
    crate::detect::network_online()
}

/// Scans for Wi-Fi networks.
pub fn scan(backend: Backend) -> Vec<Wifi> {
    match backend {
        Backend::Nmcli => scan_nmcli(),
        Backend::Iwd => scan_iwctl(),
        Backend::WpaSupplicant => scan_wpa_cli(),
        Backend::None => Vec::new(),
    }
}

fn scan_nmcli() -> Vec<Wifi> {
    // -t escapes ':' inside fields as '\:'.
    let Ok(output) = Command::new("nmcli")
        .args(["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut networks: Vec<Wifi> = Vec::new();
    for line in text.lines() {
        let fields = split_escaped(line);
        if fields.len() < 3 {
            continue;
        }
        let ssid = fields[0].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        let signal = fields[1].trim().parse::<u8>().unwrap_or(0);
        let security = fields[2].trim();
        let secured = !security.is_empty() && security != "--";
        push_unique(&mut networks, Wifi { ssid, signal, secured });
    }
    networks.sort_by_key(|n| std::cmp::Reverse(n.signal));
    networks
}

fn split_escaped(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ':' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn scan_iwctl() -> Vec<Wifi> {
    let Some(device) = wireless_device() else {
        return Vec::new();
    };
    let _ = Command::new("iwctl").args(["station", &device, "scan"]).output();
    let Ok(output) = Command::new("iwctl")
        .args(["station", &device, "get-networks"])
        .output()
    else {
        return Vec::new();
    };
    let mut networks = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("---") || trimmed.contains("Network name") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(ssid) = parts.next() else { continue };
        let rest: Vec<&str> = parts.collect();
        let security = rest.first().copied().unwrap_or("");
        let secured = security == "psk" || security == "8021x";
        push_unique(
            &mut networks,
            Wifi {
                ssid: ssid.to_string(),
                signal: 70,
                secured,
            },
        );
    }
    networks
}

fn scan_wpa_cli() -> Vec<Wifi> {
    let _ = Command::new("wpa_cli").arg("scan").output();
    let Ok(output) = Command::new("wpa_cli").arg("scan_results").output() else {
        return Vec::new();
    };
    let mut networks = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 5 {
            continue;
        }
        let signal = parts[2].trim().parse::<i32>().unwrap_or(-100);
        let signal = ((signal + 100).clamp(0, 100)) as u8;
        let secured = parts[3].contains("WPA") || parts[3].contains("WEP");
        let ssid = parts[4].trim().to_string();
        if ssid.is_empty() {
            continue;
        }
        push_unique(&mut networks, Wifi { ssid, signal, secured });
    }
    networks.sort_by_key(|n| std::cmp::Reverse(n.signal));
    networks
}

fn push_unique(networks: &mut Vec<Wifi>, wifi: Wifi) {
    if !networks.iter().any(|n| n.ssid == wifi.ssid) {
        networks.push(wifi);
    }
}

fn wireless_device() -> Option<String> {
    let entries = std::fs::read_dir("/sys/class/net").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if entry.path().join("wireless").exists() {
            return Some(name);
        }
    }
    None
}

/// Connects to a Wi-Fi network. `password` is empty for open networks.
pub fn connect(backend: Backend, ssid: &str, password: &str) -> Result<()> {
    match backend {
        Backend::Nmcli => {
            let mut cmd = Command::new("nmcli");
            cmd.args(["device", "wifi", "connect", ssid]);
            if !password.is_empty() {
                cmd.args(["password", password]);
            }
            run_connect(cmd)
        }
        Backend::Iwd => {
            let Some(device) = wireless_device() else {
                return Err(eyre!("no hay interfaz inalámbrica"));
            };
            let mut cmd = Command::new("iwctl");
            if !password.is_empty() {
                cmd.args(["--passphrase", password]);
            }
            cmd.args(["station", &device, "connect", ssid]);
            run_connect(cmd)
        }
        Backend::WpaSupplicant => Err(eyre!(
            "wpa_cli no permite asociarse sin escribir en la configuración; usa nmcli"
        )),
        Backend::None => Err(eyre!("no hay backend de red disponible")),
    }
}

fn run_connect(mut cmd: Command) -> Result<()> {
    let output = cmd.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(eyre!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        ))
    }
}

/// Sample networks for the dry-run when no backend is present.
pub fn sample_networks() -> Vec<Wifi> {
    vec![
        Wifi {
            ssid: "nixdots-lab".into(),
            signal: 90,
            secured: true,
        },
        Wifi {
            ssid: "Casa-2.4G".into(),
            signal: 62,
            secured: true,
        },
        Wifi {
            ssid: "Cafe-Invitados".into(),
            signal: 34,
            secured: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_reflect_signal() {
        assert_eq!(Wifi { ssid: "x".into(), signal: 90, secured: false }.bars(), "▂▄▆█");
        assert_eq!(Wifi { ssid: "x".into(), signal: 10, secured: false }.bars(), "▂   ");
    }

    #[test]
    fn nmcli_unescapes_colons() {
        let fields = split_escaped(r"Casa\:2.4G:80:WPA2");
        assert_eq!(fields[0], "Casa:2.4G");
        assert_eq!(fields[1], "80");
    }

    #[test]
    fn which_finds_common_tools() {
        // `sh` is always present in the test environment.
        assert!(which("sh"));
        assert!(!which("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn sample_networks_are_marked() {
        let nets = sample_networks();
        assert!(!nets.is_empty());
        assert!(nets.iter().any(|n| !n.secured));
    }

    #[test]
    fn path_probe_never_panics() {
        let _ = ethernet_connected();
        let _ = std::path::Path::new("/sys/class/net").exists();
    }
}
