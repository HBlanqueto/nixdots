//! `nixdots-setup` — CLI del asistente de primer arranque: aplica los datos del
//! usuario (JSON), escribe el hash y `settings.nix`, y deja la nueva generación
//! para el próximo arranque.
//!
//!   nixdots-setup apply <datos.json> [--repo <ruta>] [--dry-run]
//!
//! La contraseña se lee de stdin (nunca del JSON ni de argv).

use color_eyre::eyre::{bail, eyre, Context, Result};
use nixdots_core::runner::{self, Cmd, Stream};
use nixdots_core::settings::Settings;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const HELP: &str = "nixdots-setup apply <datos.json> [--repo <ruta>] [--dry-run]

  datos.json  fullName, username, locale, keyboardLayout, keyboardVariant,
              timeZone y, opcionalmente, gitName y gitEmail.
  --repo      repositorio a reconstruir (defecto /persist/nixdots)
  --dry-run   copia temporal del repo; solo imprime los comandos

La contraseña se lee de stdin.";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    full_name: String,
    username: String,
    locale: String,
    #[serde(default)]
    keyboard_layout: String,
    #[serde(default)]
    keyboard_variant: String,
    time_zone: String,
    #[serde(default)]
    git_name: Option<String>,
    #[serde(default)]
    git_email: Option<String>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("apply") => apply(&args[1..]),
        Some("-h") | Some("--help") | None => {
            println!("{HELP}");
            Ok(())
        }
        Some(other) => bail!("orden desconocida: {other}\n\n{HELP}"),
    }
}

fn apply(args: &[String]) -> Result<()> {
    let mut data_path = None;
    let mut repo = PathBuf::from("/persist/nixdots");
    let mut dry_run = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--repo" => {
                repo = PathBuf::from(it.next().ok_or_else(|| eyre!("--repo necesita un valor"))?)
            }
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                println!("{HELP}");
                return Ok(());
            }
            other if other.starts_with("--") => bail!("opción desconocida: {other}"),
            other => data_path = Some(PathBuf::from(other)),
        }
    }
    let data_path = data_path.ok_or_else(|| eyre!("falta el archivo de datos"))?;
    let data = load_data(&data_path)?;
    validate(&data)?;
    let password = read_password()?;

    let repo = if dry_run { copy_to_temp(&repo)? } else { repo };

    // 1. Hash (0600) antes de settings.nix: user.nix exige que exista.
    let hash = nixdots_core::crypto::hash_password(&password)?;
    let secrets = Path::new("/persist/secrets");
    let hash_path = secrets.join(format!("{}.hash", data.username));
    println!("$ escribir {} (0600)", hash_path.display());
    if !dry_run {
        std::fs::create_dir_all(secrets)?;
        set_mode(secrets, 0o700)?;
        std::fs::write(&hash_path, hash.as_bytes())?;
        set_mode(&hash_path, 0o600)?;
    }

    // 2. settings.nix: se parte del que ya hay (el instalador pudo escribir
    //    prettyHostName, stateVersion, etc.) y solo se reemplazan los campos
    //    que trae el JSON.
    let mut settings = Settings::read(&repo)?;
    settings.username = Some(data.username);
    settings.userdescription = Some(data.full_name);
    settings.default_locale = data.locale;
    settings.keyboard_layout = data.keyboard_layout;
    settings.keyboard_variant = data.keyboard_variant;
    settings.time_zone = data.time_zone;
    if let Some(name) = data.git_name {
        settings.git_name = Some(name);
    }
    if let Some(email) = data.git_email {
        settings.git_email = Some(email);
    }
    println!("$ escribir {}/settings.nix", repo.display());
    settings.write(&repo)?;

    // 3. git add: el flake solo ve lo que git conoce.
    run_step(&git_cmd(&repo, &["add", "-A"]), dry_run, "git add")?;

    // 4. boot, no switch: activar en caliente apagaría greetd y mataría esta
    //    misma sesión a mitad de la activación. El bucle de sesión reinicia.
    let host = hostname()?;
    let rebuild = Cmd::new("nixos-rebuild")
        .args(["boot", "--impure", "--flake"])
        .arg(format!("{}#{host}", repo.display()));
    run_step(&rebuild, dry_run, "nixos-rebuild")
}

fn load_data(path: &Path) -> Result<Data> {
    let text = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("no se pudo leer {}", path.display()))?;
    serde_json::from_str(&text).wrap_err("los datos no tienen el formato esperado")
}

fn validate(data: &Data) -> Result<()> {
    if data.full_name.trim().is_empty() {
        bail!("fullName está vacío");
    }
    if let Some(err) = nixdots_core::user::username_error(&data.username) {
        bail!("username inválido: {err}");
    }
    if data.locale.trim().is_empty() {
        bail!("locale está vacío");
    }
    if data.time_zone.trim().is_empty() {
        bail!("timeZone está vacío");
    }
    if data.keyboard_layout.trim().is_empty() {
        bail!("keyboardLayout está vacío");
    }
    if let Some(email) = &data.git_email {
        if !email.trim().is_empty() && !(email.contains('@') && email.contains('.')) {
            bail!("gitEmail no parece un correo");
        }
    }
    Ok(())
}

/// Lee toda la entrada estándar y quita el salto de línea final. El búfer y la
/// contraseña viven en `Zeroizing`, así que se borran al salir de ámbito.
fn read_password() -> Result<Zeroizing<String>> {
    let mut buf = Zeroizing::new(String::new());
    std::io::stdin()
        .read_to_string(&mut buf)
        .wrap_err("no se pudo leer la contraseña de stdin")?;
    let password = buf.trim_end_matches(['\n', '\r']).to_string();
    if password.is_empty() {
        bail!("la contraseña está vacía");
    }
    Ok(Zeroizing::new(password))
}

fn hostname() -> Result<String> {
    let name = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .wrap_err("no se pudo leer el hostname del sistema")?;
    let name = name.trim().to_string();
    if name.is_empty() {
        bail!("el hostname está vacío");
    }
    Ok(name)
}

fn git_cmd(repo: &Path, args: &[&str]) -> Cmd {
    let mut cmd = Cmd::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", repo.display()))
        .arg("-C")
        .arg_path(repo);
    for a in args {
        cmd = cmd.arg(*a);
    }
    cmd
}

/// Imprime el comando; en real lo ejecuta y transmite el registro.
fn run_step(cmd: &Cmd, dry_run: bool, what: &str) -> Result<()> {
    println!("$ {}", cmd.display);
    if dry_run {
        return Ok(());
    }
    let mut sink = |stream: Stream| match stream {
        Stream::Stdout(l) => println!("{l}"),
        Stream::Stderr(l) => eprintln!("{l}"),
    };
    let code = runner::run(cmd, &mut sink);
    if code != 0 {
        bail!("{what} falló (código {code})");
    }
    Ok(())
}

/// Copia el repo a un directorio temporal único (`<tmp>/nixdots-setup-<pid>-<ns>`).
fn copy_to_temp(repo: &Path) -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("nixdots-setup-{}-{nanos}", std::process::id()));
    // create_dir falls over if the path already exists, so a previous run's
    // directory is never reused.
    std::fs::create_dir(&tmp).wrap_err_with(|| format!("no se pudo crear {}", tmp.display()))?;
    set_mode(&tmp, 0o700)?;
    let status = std::process::Command::new("cp")
        .arg("-a")
        .arg(repo.join("."))
        .arg(&tmp)
        .status()?;
    if !status.success() {
        bail!("no se pudo copiar {} al temporal", repo.display());
    }
    Ok(tmp)
}

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}
