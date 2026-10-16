//! Ejecuta el CLI `nixdots-setup apply` con `gio::Subprocess`: sin hilos,
//! lectura asíncrona línea a línea. La contraseña va por stdin y se cierra.

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib::Priority;

/// Escribe el JSON en un directorio temporal único (0700) y devuelve su ruta
/// (archivo 0600).
fn write_json(json: &str) -> std::io::Result<std::path::PathBuf> {
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

/// Lanza `nixdots-setup apply` y llama a `on_line` por cada línea y a `on_done`
/// una vez, con `true` si terminó bien.
pub fn start<F, D>(
    json: &str,
    repo: &str,
    dry_run: bool,
    password: &str,
    on_line: F,
    on_done: D,
) -> Result<(), String>
where
    F: Fn(&str) + Clone + 'static,
    D: Fn(bool) + 'static,
{
    let path = write_json(json).map_err(|e| format!("no se pudo escribir los datos: {e}"))?;

    let mut argv: Vec<std::ffi::OsString> = vec![cli(), "apply".into(), path.into_os_string()];
    if !repo.is_empty() {
        argv.push("--repo".into());
        argv.push(repo.into());
    }
    if dry_run {
        argv.push("--dry-run".into());
    }
    let argv_ref: Vec<&std::ffi::OsStr> = argv.iter().map(|s| s.as_os_str()).collect();

    let launcher = gio::SubprocessLauncher::new(
        gio::SubprocessFlags::STDIN_PIPE
            | gio::SubprocessFlags::STDOUT_PIPE
            | gio::SubprocessFlags::STDERR_PIPE,
    );
    let subprocess = launcher
        .spawn(&argv_ref)
        .map_err(|e| format!("no se pudo ejecutar el CLI: {e}"))?;

    // Contraseña por stdin y cierre inmediato (el CLI lee hasta EOF).
    let stdin = subprocess.stdin_pipe().ok_or("sin stdin")?;
    let data = password.as_bytes();
    let mut written = 0;
    while written < data.len() {
        let n = stdin
            .write(&data[written..], gio::Cancellable::NONE)
            .map_err(|e| e.to_string())?;
        written += n as usize;
    }
    stdin.close(gio::Cancellable::NONE).map_err(|e| e.to_string())?;

    // Salida línea a línea.
    if let Some(out) = subprocess.stdout_pipe() {
        pump(gio::DataInputStream::new(&out), on_line.clone());
    }
    if let Some(err) = subprocess.stderr_pipe() {
        pump(gio::DataInputStream::new(&err), on_line);
    }

    // Fin: éxito = salió 0.
    let done = subprocess.clone();
    subprocess.wait_async(gio::Cancellable::NONE, move |res| {
        on_done(res.is_ok() && done.is_successful());
    });
    Ok(())
}

/// Lee líneas de un stream de forma asíncrona, encadenando una lectura con la
/// siguiente (sin hilos).
fn pump<F: Fn(&str) + Clone + 'static>(stream: gio::DataInputStream, on_line: F) {
    let next = stream.clone();
    stream.read_line_utf8_async(Priority::DEFAULT, gio::Cancellable::NONE, move |res| {
        if let Ok(Some(line)) = res {
            on_line(&line);
            pump(next.clone(), on_line.clone());
        }
    });
}
