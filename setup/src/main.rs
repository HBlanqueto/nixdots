//! `nixdots-setup` — CLI del asistente de primer arranque: aplica los datos del
//! usuario (JSON), escribe el hash y `user.json`, y deja la nueva generación
//! para el próximo arranque.
//!
//!   nixdots-setup apply <datos.json> [--repo <ruta>] [--dry-run]
//!
//! La contraseña se lee de stdin (nunca del JSON ni de argv).

use color_eyre::eyre::{bail, eyre, Context, Result};
use nixdots_core::runner::{self, Cmd, Stream};
use nixdots_core::user_data::UserData;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const HELP: &str = "nixdots-setup apply <datos.json> [--repo <ruta>] [--dry-run]

  datos.json  fullName, username, locale, region, keyboardLayout,
              keyboardVariant, timeZone, timeZoneAuto, a11y y, opcionalmente,
              gitName y gitEmail.
  --repo      repositorio a reconstruir (defecto /persist/nixdots/repo)
  --dry-run   solo imprime lo que haría, sin escribir nada

La contraseña se lee de stdin.";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    full_name: String,
    username: String,
    locale: String,
    #[serde(default)]
    region: Option<String>,
    #[serde(default)]
    keyboard_layout: String,
    #[serde(default)]
    keyboard_variant: String,
    time_zone: String,
    #[serde(default)]
    time_zone_auto: Option<bool>,
    #[serde(default)]
    a11y: Option<nixdots_core::user_data::A11y>,
    #[serde(default)]
    git_name: Option<String>,
    #[serde(default)]
    git_email: Option<String>,
}

fn main() {
    color_eyre::install().ok();
    if let Err(err) = run() {
        // Una sola línea legible para que la GUI muestre el motivo.
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
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
    let mut repo = PathBuf::from("/persist/nixdots/repo");
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
    let sim_fail = std::env::var_os("NIXDOTS_SIM_FAIL").is_some();

    // 1. Hash (0600) antes de user.json: user.nix exige que exista.
    phase(0, dry_run);
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

    // 2. user.json, fuera del repo (el flake lo mezcla con settings.nix).
    phase(1, dry_run);
    if sim_fail {
        bail!("Fallo simulado al aplicar la configuración");
    }
    let user = UserData {
        username: data.username.clone(),
        userdescription: Some(data.full_name),
        pretty_host_name: None,
        git_name: data.git_name,
        git_email: data.git_email,
        default_locale: Some(data.locale),
        region: data.region.and_then(non_empty),
        time_zone: Some(data.time_zone),
        time_zone_auto: data.time_zone_auto,
        keyboard_layout: non_empty(data.keyboard_layout),
        keyboard_variant: non_empty(data.keyboard_variant),
        a11y: data.a11y,
    };
    println!(
        "$ escribir {} (0644)",
        nixdots_core::user_data::path().display()
    );
    if !dry_run {
        user.write()?;
    }

    // 3. boot, no switch: activar en caliente apagaría greetd y mataría esta
    //    misma sesión a mitad de la activación. El bucle de sesión reinicia.
    phase(2, dry_run);
    let host = hostname()?;
    let rebuild = Cmd::new("nixos-rebuild")
        .args(["boot", "--impure", "--flake"])
        .arg(format!("{}#{host}", repo.display()));
    run_step(&rebuild, dry_run, "nixos-rebuild")?;

    // ~<usuario>/nixdots -> repo, y el repo en manos del usuario, solo cuando el
    // rebuild ha terminado bien: si falla no se tocan permisos.
    phase(3, dry_run);
    let home_link = format!("/home/{}/nixdots", data.username);
    println!(
        "$ ln -sfn {} {home_link} && chown -R 1000:users {}",
        repo.display(),
        repo.display()
    );
    if !dry_run {
        link_home(&data.username, &repo)?;
    }
    phase(4, dry_run);
    Ok(())
}

/// Línea `PHASE n` que la GUI interpreta. En `--dry-run` (simulación) se toma
/// un respiro antes de cada fase para que se vean todas.
fn phase(n: u8, dry_run: bool) {
    if dry_run && n > 0 {
        std::thread::sleep(std::time::Duration::from_millis(1600));
    }
    println!("PHASE {n}");
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
    if !locale_known(&data.locale) {
        bail!("locale desconocido: {}", data.locale);
    }
    if let Some(region) = data.region.as_deref() {
        if !region.trim().is_empty() && !locale_known(region) {
            bail!("region desconocida: {region}");
        }
    }
    if data.time_zone.trim().is_empty() {
        bail!("timeZone está vacío");
    }
    if !timezone_known(&data.time_zone) {
        bail!("zona horaria desconocida: {}", data.time_zone);
    }
    if let Some(email) = &data.git_email {
        if !email.trim().is_empty() && !(email.contains('@') && email.contains('.')) {
            bail!("gitEmail no parece un correo");
        }
    }
    Ok(())
}

/// ¿El locale existe en la tabla de `nixdots-lists`?
fn locale_known(code: &str) -> bool {
    nixdots_lists::locale::all().iter().any(|l| l.code == code)
}

/// ¿La zona horaria existe en la tabla de `nixdots-lists`?
fn timezone_known(zone: &str) -> bool {
    nixdots_lists::timezone::zones().iter().any(|z| z == zone)
}

/// `None` para cadenas vacías: así el `//` del flake deja el valor de
/// `settings.nix` en vez de sobrescribirlo con "".
fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
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

fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

/// Deja `~<usuario>/nixdots` como enlace al repo y pone el repo en manos del
/// usuario. El uid está fijado a 1000 en `modules/common/user.nix` (el asistente
/// corre antes de que exista la cuenta).
fn link_home(username: &str, repo: &Path) -> Result<()> {
    let home = format!("/home/{username}");
    std::fs::create_dir_all(&home).wrap_err_with(|| format!("no se pudo crear {home}"))?;
    let link = format!("{home}/nixdots");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(repo, &link).wrap_err("no se pudo crear el enlace ~/nixdots")?;

    chown(&["-R", "1000:users"], repo)?;
    chown(&["1000:users"], Path::new(&home))?;
    Ok(())
}

fn chown(args: &[&str], path: &Path) -> Result<()> {
    let status = std::process::Command::new("chown")
        .args(args)
        .arg(path)
        .status()?;
    if !status.success() {
        bail!("chown falló en {}", path.display());
    }
    Ok(())
}
