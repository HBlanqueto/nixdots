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
    /// The chosen hostname. Not stored in settings.nix anymore (the machine's
    /// name lives in machine.json); kept here as the installer's current
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
    /// `hashedPasswordFile`, derived from the username. `None` when there is no
    /// account yet, in which case the key is not written.
    pub fn password_file(&self) -> Option<String> {
        self.username
            .as_deref()
            .map(|username| format!("/persist/secrets/{username}.hash"))
    }

    /// A copy without the user fields: the installed `settings.nix` does not
    /// carry them (they live in `user.json`).
    pub fn without_user(&self) -> Self {
        let mut copy = self.clone();
        copy.username = None;
        copy.userdescription = None;
        copy
    }

    /// Values used when `settings.nix` cannot be evaluated (e.g. the simulation
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
        let mut settings: Self = serde_json::from_slice(&output.stdout)
            .wrap_err("settings.nix no tiene el formato esperado")?;
        // An empty string counts as absent, so `settings.username or null` stays
        // null (otherwise the first-boot assistant would never open).
        settings.username = settings.username.filter(|value| !value.is_empty());
        settings.userdescription = settings.userdescription.filter(|value| !value.is_empty());
        Ok(settings)
    }

    /// Renders the template. `username`/`userdescription`/`hashedPasswordFile`
    /// are omitted when there is no account, so `first-boot.nix` can detect it.
    pub fn render(&self) -> String {
        let pretty = if self.pretty_host_name.is_empty() {
            String::new()
        } else {
            format!(
                "    prettyHostName = {};\n",
                nix_str(&self.pretty_host_name)
            )
        };
        let user = match (&self.username, self.password_file()) {
            (Some(username), Some(password_file)) => format!(
                "    username = {};\n    userdescription = {};\n{pretty}    hashedPasswordFile = {};\n",
                nix_str(username),
                nix_str(self.userdescription.as_deref().unwrap_or("")),
                nix_str(&password_file),
            ),
            _ => pretty,
        };
        format!(
            "{{\n    stateVersion = {state_version};\n\n    timeZone = {time_zone};\n    defaultLocale = {default_locale};\n    keyboardLayout = {keyboard_layout};\n    keyboardVariant = {keyboard_variant};\n\n{user}\n    gitName = {git_name};\n    gitEmail = {git_email};\n}}\n",
            state_version = nix_str(&self.state_version),
            time_zone = nix_str(&self.time_zone),
            keyboard_layout = nix_str(&self.keyboard_layout),
            keyboard_variant = nix_str(&self.keyboard_variant),
            default_locale = nix_str(&self.default_locale),
            user = user,
            git_name = nix_opt(&self.git_name),
            git_email = nix_opt(&self.git_email),
        )
    }

    /// Writes `settings.nix` atomically: a sibling temporary file is fully
    /// written and synced, then renamed over the original, so an interrupted
    /// install never leaves a truncated file the flake cannot evaluate.
    pub fn write(&self, repo: &Path) -> Result<()> {
        let target = repo.join("settings.nix");
        let tmp = repo.join(".settings.nix.tmp");
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&tmp)
                .wrap_err("no se pudo crear el temporal de settings.nix")?;
            file.write_all(self.render().as_bytes())
                .and_then(|()| file.sync_all())
                .wrap_err("no se pudo escribir settings.nix")?;
        }
        std::fs::rename(&tmp, &target).wrap_err("no se pudo reemplazar settings.nix")?;
        Ok(())
    }
}

/// A Nix double-quoted string literal. Besides `\` and `"`, `${` must be
/// escaped: unescaped it starts an interpolation, so a value like
/// `${builtins.readFile /etc/shadow}` would be evaluated as Nix code.
fn nix_str(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${");
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
            username: Some("humbe".into()),
            userdescription: Some("H. Blanqueto".into()),
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
        s.username = Some("ana".into());
        assert_eq!(
            s.password_file().as_deref(),
            Some("/persist/secrets/ana.hash")
        );
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
