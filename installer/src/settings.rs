//! Reads and rewrites the flake's `settings.nix`.
//!
//! The file is regenerated from a template that mirrors the original layout
//! (same key order, 4-space indentation). `settings.nix` is never edited with
//! `sed`.

use color_eyre::eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

/// Mirror of `settings.nix`. `hashedPasswordFile` is intentionally absent: it
/// is always derived from `username` when writing, and ignored when reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub state_version: String,
    /// The chosen hostname. Not stored in settings.nix anymore (hosts live in
    /// hosts/<name>/); kept here as the installer's current selection.
    #[serde(default)]
    pub host_name: String,
    pub time_zone: String,
    pub default_locale: String,
    pub username: String,
    pub userdescription: String,
    /// macOS-style visible name («ThinkPad L14 de Humberto»). Empty on old
    /// settings.nix files, in which case the line is not written.
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
    /// `hashedPasswordFile`, derived from the username.
    pub fn password_file(&self) -> String {
        format!("/persist/secrets/{}.hash", self.username)
    }

    /// Values used when `settings.nix` cannot be evaluated (e.g. the simulation
    /// run on a machine without nix). Mirrors the repo's defaults.
    pub fn fallback() -> Self {
        Self {
            state_version: "26.05".into(),
            host_name: "nixos".into(),
            time_zone: "America/Merida".into(),
            default_locale: "es_MX.UTF-8".into(),
            username: "humbe".into(),
            userdescription: "H. Blanqueto".into(),
            pretty_host_name: String::new(),
            keyboard_layout: "us".into(),
            keyboard_variant: String::new(),
            git_name: Some("H. Blanqueto".into()),
            git_email: Some("mc4w6wmkrv@privaterelay.appleid.com".into()),
        }
    }

    /// `nix eval --json -f settings.nix`.
    pub fn read(repo: &Path) -> Result<Self> {
        let file = repo.join("settings.nix");
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
                "`nix eval -f settings.nix` falló: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .wrap_err("settings.nix no tiene el formato esperado")
    }

    /// Renders the template.
    pub fn render(&self) -> String {
        let pretty = if self.pretty_host_name.is_empty() {
            String::new()
        } else {
            format!("    prettyHostName = {};\n", nix_str(&self.pretty_host_name))
        };
        format!(
            "{{\n    stateVersion = {state_version};\n\n    timeZone = {time_zone};\n    defaultLocale = {default_locale};\n    keyboardLayout = {keyboard_layout};\n    keyboardVariant = {keyboard_variant};\n\n    username = {username};\n    userdescription = {userdescription};\n{pretty}    hashedPasswordFile = {password_file};\n\n    gitName = {git_name};\n    gitEmail = {git_email};\n}}\n",
            state_version = nix_str(&self.state_version),
            time_zone = nix_str(&self.time_zone),
            keyboard_layout = nix_str(&self.keyboard_layout),
            keyboard_variant = nix_str(&self.keyboard_variant),
            default_locale = nix_str(&self.default_locale),
            username = nix_str(&self.username),
            userdescription = nix_str(&self.userdescription),
            pretty = pretty,
            password_file = nix_str(&self.password_file()),
            git_name = nix_opt(&self.git_name),
            git_email = nix_opt(&self.git_email),
        )
    }

    pub fn write(&self, repo: &Path) -> Result<()> {
        std::fs::write(repo.join("settings.nix"), self.render())
            .wrap_err("no se pudo escribir settings.nix")?;
        Ok(())
    }
}

fn nix_str(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn nix_opt(value: &Option<String>) -> String {
    match value {
        Some(value) => nix_str(value),
        None => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Settings {
        Settings {
            state_version: "26.05".into(),
            host_name: "nixos".into(),
            time_zone: "America/Merida".into(),
            default_locale: "es_MX.UTF-8".into(),
            username: "humbe".into(),
            userdescription: "H. Blanqueto".into(),
            pretty_host_name: String::new(),
            keyboard_layout: "us".into(),
            keyboard_variant: String::new(),
            git_name: Some("H. Blanqueto".into()),
            git_email: Some("a@b.c".into()),
        }
    }

    #[test]
    fn render_matches_expected_layout() {
        let rendered = sample().render();
        assert!(rendered.contains("    hashedPasswordFile = \"/persist/secrets/humbe.hash\";"));
        assert!(rendered.contains("    stateVersion = \"26.05\";"));
        // 4-space indentation, no tabs.
        assert!(!rendered.contains('\t'));
    }

    #[test]
    fn password_file_follows_username() {
        let mut s = sample();
        s.username = "ana".into();
        assert_eq!(s.password_file(), "/persist/secrets/ana.hash");
    }

    #[test]
    fn render_null_git_when_omitted() {
        let mut s = sample();
        s.git_name = None;
        s.git_email = None;
        let rendered = s.render();
        assert!(rendered.contains("    gitName = null;"));
        assert!(rendered.contains("    gitEmail = null;"));
    }

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
