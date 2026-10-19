//! Reads the flake's `system/lib/defaults.nix`.
//!
//! The installer only needs `stateVersion` from there (the rest of the system
//! data comes from `machine.toml`); the file is never rewritten.

use color_eyre::eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

/// Mirror of `system/lib/defaults.nix` used while reading it. `hashedPasswordFile` is
/// intentionally absent: it is derived from `username` elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub state_version: String,
    /// The chosen hostname. Not stored in system/lib/defaults.nix anymore (the machine's
    /// name lives in machine.toml); kept here as the installer's current
    /// selection.
    #[serde(default)]
    pub host_name: String,
    pub time_zone: String,
    pub default_locale: String,
    /// Optional: the first-boot assistant runs before the account exists. `None`
    /// means the key is not written at all, which is how `first-boot.nix` knows
    /// there is no account yet (`settings.username or null`).
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub userdescription: Option<String>,
    /// macOS-style visible name («ThinkPad L14 de Humberto»). Empty on old
    /// system/lib/defaults.nix files, in which case the line is not written.
    #[serde(default)]
    pub pretty_host_name: String,
    #[serde(default = "default_keyboard")]
    pub keyboard_layout: String,
    #[serde(default)]
    pub keyboard_variant: String,
    /// `null` when git is left unconfigured by the installer.
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
}

fn default_keyboard() -> String {
    "us".to_string()
}

impl Settings {
    /// Values used when `system/lib/defaults.nix` cannot be evaluated (e.g. the simulation
    /// run on a machine without nix). Mirrors the repo's defaults.
    ///
    /// Personal fields stay empty: the installer and the first-boot assistant
    /// start every user field blank, and the public repo must not carry anyone's
    /// name or e-mail.
    pub fn fallback() -> Self {
        Self {
            state_version: "26.05".into(),
            host_name: "nixos".into(),
            time_zone: "America/Merida".into(),
            default_locale: "es_MX.UTF-8".into(),
            username: None,
            userdescription: None,
            pretty_host_name: String::new(),
            keyboard_layout: "es".into(),
            keyboard_variant: String::new(),
            git_name: None,
            git_email: None,
        }
    }

    /// `nix eval --json -f system/lib/defaults.nix`.
    pub fn read(repo: &Path) -> Result<Self> {
        let file = repo.join("system/lib/defaults.nix");
        let output = Command::new("nix")
            .arg("eval")
            .arg("--extra-experimental-features")
            .arg("nix-command flakes")
            .arg("--json")
            .arg("-f")
            .arg(&file)
            .env("NIX_CONFIG", crate::runner::nix_config())
            .stdin(Stdio::null())
            .output()
            .wrap_err("no se pudo ejecutar `nix eval`")?;
        if !output.status.success() {
            return Err(eyre!(
                "`nix eval -f system/lib/defaults.nix` falló: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let mut settings: Self = serde_json::from_slice(&output.stdout)
            .wrap_err("system/lib/defaults.nix no tiene el formato esperado")?;
        // An empty string counts as absent, so `settings.username or null` stays
        // null (otherwise the first-boot assistant would never open).
        settings.username = settings.username.filter(|value| !value.is_empty());
        settings.userdescription = settings.userdescription.filter(|value| !value.is_empty());
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_null_git_parses() {
        let json = r#"{ "system": "x86_64-linux", "stateVersion": "26.05", "hostName": "h",
            "timeZone": "UTC", "defaultLocale": "en_US.UTF-8", "username": "u",
            "userdescription": "U", "hashedPasswordFile": "/p/u.hash", "gitName": null, "gitEmail": null }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.git_name, None);
    }

    #[test]
    fn json_camel_case_round_trips() {
        let json = r#"{
            "system": "aarch64-linux",
            "stateVersion": "26.05",
            "hostName": "utm",
            "timeZone": "America/Merida",
            "defaultLocale": "es_MX.UTF-8",
            "username": "humbe",
            "userdescription": "H. Blanqueto",
            "hashedPasswordFile": "/persist/secrets/humbe.hash",
            "gitName": "H. Blanqueto",
            "gitEmail": "a@b.c"
        }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.host_name, "utm");
        assert_eq!(s.state_version, "26.05");
    }
}
