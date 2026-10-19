//! Reconstrucción de la versión instalada con la misma protección que
//! `nixdots-rebuild`: primero se construye, luego se comprueba que ningún
//! usuario normal vaya a desaparecer, y solo entonces se activa.
//!
//! Lo usan `nixdots-userd` (y, en el futuro, el binario `nixdots`). Nunca
//! reconstruye la copia de trabajo del repo: siempre el source instalado en
//! `/etc/nixdots/flake-ref`.

use color_eyre::eyre::{bail, eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::runner;

/// Modo de `nixos-rebuild`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Switch,
    Boot,
    Build,
}

impl Mode {
    fn arg(self) -> &'static str {
        match self {
            Mode::Switch => "switch",
            Mode::Boot => "boot",
            Mode::Build => "build",
        }
    }
}

/// Resultado de una reconstrucción que llegó a activar.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Se activó (switch/boot). `false` en modo build.
    pub activated: bool,
    /// Unidades que quedaron en fallo tras una activación con aviso (código 4
    /// de `nixos-rebuild`).
    pub failed_units: Vec<String>,
}

/// `/etc/nixdots/flake-ref` y `/etc/nixdots/rev`, validados.
pub fn system_ref() -> Result<(PathBuf, String)> {
    let raw = std::fs::read_to_string("/etc/nixdots/flake-ref")
        .wrap_err("falta /etc/nixdots/flake-ref")?
        .trim()
        .to_string();
    let rev = std::fs::read_to_string("/etc/nixdots/rev")
        .wrap_err("falta /etc/nixdots/rev")?
        .trim()
        .to_string();
    let path = PathBuf::from(&raw);
    if raw.is_empty() || !path.is_dir() || !path.join("flake.nix").is_file() {
        bail!("referencia del sistema no válida: {raw}");
    }
    if rev.is_empty() {
        bail!("revisión del sistema vacía");
    }
    Ok((path, rev))
}

/// Ejecuta la reconstrucción desde `flake_ref`. `allow_missing` son los
/// usuarios que pueden desaparecer en esta operación (p. ej. el que se borra).
pub fn run(flake_ref: &Path, mode: Mode, allow_missing: &[String]) -> Result<Outcome> {
    let ref_arg = format!("path:{}#nixdots", flake_ref.display());

    let build = Command::new("nixos-rebuild")
        .args(["build", "--impure", "--flake", &ref_arg])
        .env("NIX_CONFIG", runner::nix_config())
        .output()
        .wrap_err("no se pudo ejecutar nixos-rebuild")?;
    if !build.status.success() {
        bail!("nixos-rebuild build falló: {}", tail(&build.stderr));
    }

    let disappearing = disappearing_users(flake_ref, allow_missing)?;
    if !disappearing.is_empty() {
        bail!(
            "estos usuarios normales desaparecerían de la configuración: {}",
            disappearing.join(", ")
        );
    }

    if mode == Mode::Build {
        return Ok(Outcome::default());
    }

    let activate = Command::new("nixos-rebuild")
        .args([mode.arg(), "--impure", "--flake", &ref_arg])
        .env("NIX_CONFIG", runner::nix_config())
        .output()
        .wrap_err("no se pudo ejecutar nixos-rebuild")?;
    match activate.status.code() {
        Some(0) => Ok(Outcome {
            activated: true,
            failed_units: Vec::new(),
        }),
        // 4: activado, pero alguna unidad falló. No es motivo para revertir.
        Some(4) => Ok(Outcome {
            activated: true,
            failed_units: failed_units(),
        }),
        code => bail!(
            "nixos-rebuild {} falló (código {}): {}",
            mode.arg(),
            code.map(|c| c.to_string()).unwrap_or_else(|| "señal".into()),
            tail(&activate.stderr)
        ),
    }
}

/// Usuarios normales del sistema que no están en la configuración nueva y no
/// están en `allow_missing`.
fn disappearing_users(flake_ref: &Path, allow_missing: &[String]) -> Result<Vec<String>> {
    let ref_arg = format!("path:{}#nixdots", flake_ref.display());
    let apply = "u: builtins.filter (n: u.${n}.isNormalUser or false) (builtins.attrNames u)";
    let output = Command::new("nix")
        .args([
            "eval",
            "--impure",
            "--json",
            &ref_arg,
            "--apply",
            apply,
        ])
        .env("NIX_CONFIG", runner::nix_config())
        .output()
        .wrap_err("no se pudo evaluar la configuración nueva")?;
    if !output.status.success() {
        bail!("nix eval falló: {}", tail(&output.stderr));
    }
    let new_users: Vec<String> = serde_json::from_slice(&output.stdout)
        .map_err(|e| eyre!("salida de nix eval ilegible: {e}"))?;

    Ok(running_users()
        .into_iter()
        .filter(|u| !new_users.contains(u) && !allow_missing.contains(u))
        .collect())
}

/// Usuarios normales del sistema en marcha (uid >= 1000 y < 60000).
fn running_users() -> Vec<String> {
    let Ok(passwd) = std::fs::read_to_string("/etc/passwd") else {
        return Vec::new();
    };
    passwd
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let uid: u32 = fields.nth(1)?.parse().ok()?;
            if (1000..60000).contains(&uid) && !name.starts_with("nixbld") && name != "nobody" {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Unidades systemd en fallo (para el aviso del código 4).
fn failed_units() -> Vec<String> {
    let Ok(output) = Command::new("systemctl")
        .args(["--failed", "--no-legend", "--plain"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().next().map(str::to_string))
        .collect()
}

fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}
