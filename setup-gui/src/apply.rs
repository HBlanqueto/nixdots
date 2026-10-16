//! Ejecuta el CLI `nixdots-setup apply`, con la contraseña por stdin.
//!
//! Fase 4a: siempre `--dry-run`. En 4b se quitará para aplicar de verdad. La
//! salida se recoge completa y se muestra en «Mostrar detalles» (no se
//! transmite en vivo; ver shortcut).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Escribe el JSON en un directorio temporal único (0700) con el archivo 0600.
fn write_json(json: &str) -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("nixdots-setup-gui-{}-{nanos}", std::process::id()));
    std::fs::create_dir(&dir)?;
    set_mode(&dir, 0o700)?;
    let path = dir.join("datos.json");
    std::fs::write(&path, json)?;
    set_mode(&path, 0o600)?;
    Ok(path)
}

fn set_mode(path: &std::path::Path, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

/// El binario del CLI: hermano del propio ejecutable (cargo run / paquete) o,
/// si no, `nixdots-setup` del PATH.
fn cli() -> std::ffi::OsString {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("nixdots-setup");
            if sibling.exists() {
                return sibling.into_os_string();
            }
        }
    }
    "nixdots-setup".into()
}

/// Lanza `nixdots-setup apply <json>` y devuelve `(éxito, salida)`.
///
/// shortcut: no transmite la salida línea a línea; en 4b, si hace falta el
/// progreso en vivo, leer stdout/stderr en hilos mientras corre.
pub fn run(
    json: &str,
    repo: &str,
    dry_run: bool,
    password: &str,
) -> Result<(bool, String), String> {
    let path = write_json(json).map_err(|e| format!("no se pudo escribir los datos: {e}"))?;

    let mut cmd = Command::new(cli());
    cmd.arg("apply").arg(&path);
    if !repo.is_empty() {
        cmd.arg("--repo").arg(repo);
    }
    if dry_run {
        cmd.arg("--dry-run");
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo ejecutar el CLI: {e}"))?;

    // Contraseña por stdin y cierre inmediato (el CLI lee hasta EOF).
    {
        let mut stdin = child.stdin.take().ok_or("sin stdin del CLI")?;
        stdin
            .write_all(password.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), text))
}
