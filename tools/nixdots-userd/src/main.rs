//! `nixdots-userd` — servicio de sistema para gestionar cuentas de nixdots.
//!
//! Habla por D-Bus del sistema (`org.nixdots.Users1`) y exige la acción polkit
//! `org.nixdots.users.manage` para cualquier cambio. Los datos se escriben en
//! `/persist/nixdots/users/<usuario>.toml` y el hash en `/persist/secrets`; el
//! sistema se reconstruye **desde la referencia instalada** (`/etc/nixdots/`),
//! nunca desde el repo de trabajo.

use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{bail, Context, Result};
use nixdots_common::runner;
use nixdots_common::user;
use nixdots_common::user_data::{self, UserData};
use tokio::sync::Mutex;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;
use zbus::{fdo, interface, proxy, Connection};
use zeroize::Zeroizing;

const BUS_NAME: &str = "org.nixdots.Users1";
const OBJECT_PATH: &str = "/org/nixdots/Users1";
const ACTION: &str = "org.nixdots.users.manage";
const SECRETS_DIR: &str = "/persist/secrets";

#[proxy(
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority",
    interface = "org.freedesktop.PolicyKit1.Authority"
)]
trait PolicyKit1 {
    fn check_authorization(
        &self,
        subject: (String, HashMap<String, OwnedValue>),
        action_id: &str,
        details: HashMap<String, String>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

#[proxy(
    default_service = "org.freedesktop.DBus",
    default_path = "/org/freedesktop/DBus",
    interface = "org.freedesktop.DBus"
)]
trait DBus {
    fn get_connection_unix_user(&self, name: &str) -> zbus::Result<u32>;
}

#[derive(Default)]
struct Users {
    /// Una sola operación de gestión a la vez.
    lock: Mutex<()>,
}

#[interface(name = "org.nixdots.Users1")]
impl Users {
    async fn list_users(&self) -> Vec<(String, String, bool)> {
        UserData::list()
            .map(|users| {
                users
                    .into_iter()
                    .map(|u| (u.username, u.full_name, u.admin))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn create_user(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        full_name: String,
        admin: bool,
        password: String,
    ) -> fdo::Result<()> {
        let password = Zeroizing::new(password);
        self.authorize(&header).await?;
        validate(&username, Some(&full_name))?;
        if existing(&username) {
            return Err(fdo::Error::Failed(format!("el usuario '{username}' ya existe")));
        }
        let _guard = self.lock.lock().await;
        self.emit(&emitter, "create", 1, 10, "Validando").await;

        let hash = nixdots_common::crypto::hash_password(&password)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let user = UserData {
            version: 1,
            username: username.clone(),
            full_name,
            admin,
        };
        self.emit(&emitter, "create", 2, 30, "Escribiendo la cuenta").await;
        user.write().map_err(fail)?;
        write_hash(&username, &hash).map_err(fail)?;
        self.emit(&emitter, "create", 3, 50, "Reconstruyendo el sistema").await;
        if let Err(err) = rebuild() {
            let _ = std::fs::remove_file(user_data::path_for(&username));
            let _ = std::fs::remove_file(hash_path(&username));
            self.emit(&emitter, "create", 3, 50, &format!("Error: {err}")).await;
            return Err(fail(err));
        }
        self.emit(&emitter, "create", 4, 100, "Cuenta creada").await;
        Self::emit_finished(&emitter, "create").await;
        Ok(())
    }

    async fn delete_user(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        keep_home: bool,
    ) -> fdo::Result<()> {
        self.authorize(&header).await?;
        validate(&username, None)?;
        let _guard = self.lock.lock().await;
        let Some(user) = UserData::read(&username).map_err(fail)? else {
            return Err(fdo::Error::Failed(format!("el usuario '{username}' no existe")));
        };
        if user.admin && admin_count().map_err(fail)? <= 1 {
            return Err(fdo::Error::Failed(
                "no se puede borrar el último administrador".into(),
            ));
        }
        if !keep_home && user_has_processes(&username).map_err(fail)? {
            return Err(fdo::Error::Failed(format!(
                "'{username}' tiene procesos abiertos; cierra su sesión antes de borrar su home"
            )));
        }
        self.emit(&emitter, "delete", 1, 10, "Eliminando la cuenta").await;

        let user_bak = std::fs::read_to_string(user_data::path_for(&username)).ok();
        let hash_bak = std::fs::read(hash_path(&username)).ok();
        let prefs_bak = std::fs::read_to_string(user_data::prefs_path_for(&username)).ok();
        let _ = std::fs::remove_file(user_data::path_for(&username));
        let _ = std::fs::remove_file(hash_path(&username));
        let _ = std::fs::remove_file(user_data::prefs_path_for(&username));

        self.emit(&emitter, "delete", 2, 50, "Reconstruyendo el sistema").await;
        if let Err(err) = rebuild() {
            restore(&username, user_bak.as_deref(), hash_bak.as_deref(), prefs_bak.as_deref());
            return Err(fail(err));
        }
        if !keep_home {
            let home = Path::new("/home").join(&username);
            if home.exists() {
                std::fs::remove_dir_all(&home).map_err(fail)?;
            }
        }
        self.emit(&emitter, "delete", 3, 100, "Cuenta eliminada").await;
        Self::emit_finished(&emitter, "delete").await;
        Ok(())
    }

    async fn set_admin(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        admin: bool,
    ) -> fdo::Result<()> {
        self.authorize(&header).await?;
        let _guard = self.lock.lock().await;
        let mut user = existing_user(&username)?;
        if user.admin && !admin && admin_count().map_err(fail)? <= 1 {
            return Err(fdo::Error::Failed(
                "no se puede quitar el último administrador".into(),
            ));
        }
        user.admin = admin;
        self.apply(&emitter, "set-admin", &user).await
    }

    async fn set_full_name(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        full_name: String,
    ) -> fdo::Result<()> {
        self.authorize(&header).await?;
        validate(&username, Some(&full_name))?;
        let _guard = self.lock.lock().await;
        let mut user = existing_user(&username)?;
        user.full_name = full_name;
        self.apply(&emitter, "set-full-name", &user).await
    }

    async fn set_password(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        username: String,
        password: String,
    ) -> fdo::Result<()> {
        let password = Zeroizing::new(password);
        self.authorize(&header).await?;
        validate(&username, None)?;
        let _guard = self.lock.lock().await;
        let _ = existing_user(&username)?;
        if password.is_empty() {
            return Err(fdo::Error::Failed("la contraseña está vacía".into()));
        }
        self.emit(&emitter, "set-password", 1, 30, "Cambiando la contraseña").await;
        let hash = nixdots_common::crypto::hash_password(&password)
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let old = std::fs::read(hash_path(&username)).ok();
        write_hash(&username, &hash).map_err(fail)?;
        if let Err(err) = rebuild_emitting(&emitter, "set-password").await {
            if let Some(old) = old {
                let _ = write_hash(&username, &String::from_utf8_lossy(&old));
            }
            let _ = err;
            return Err(fdo::Error::Failed(
                "no se pudo aplicar la contraseña".into(),
            ));
        }
        Self::emit_finished(&emitter, "set-password").await;
        Ok(())
    }

    /// Borra la semilla de prefs. Solo el propio usuario (o root/admin) puede.
    async fn delete_prefs_seed(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
        username: String,
    ) -> fdo::Result<()> {
        validate(&username, None)?;
        let sender = sender(&header)?;
        if !self.same_user(connection, &sender, &username).await? {
            authorize(connection, &sender).await?;
        }
        let _ = std::fs::remove_file(user_data::prefs_path_for(&username));
        Ok(())
    }

    #[zbus(signal)]
    async fn progress(
        emitter: &SignalEmitter<'_>,
        action: &str,
        phase: u32,
        percent: u32,
        message: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn finished(emitter: &SignalEmitter<'_>, action: &str) -> zbus::Result<()>;
}

impl Users {
    async fn authorize(&self, header: &Header<'_>) -> fdo::Result<()> {
        let connection = zbus::Connection::system()
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let sender = sender(header)?;
        authorize(&connection, &sender).await
    }

    async fn same_user(
        &self,
        connection: &Connection,
        sender: &str,
        username: &str,
    ) -> fdo::Result<bool> {
        let dbus = DBusProxy::new(connection)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let uid = dbus
            .get_connection_unix_user(sender)
            .await
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        Ok(uid_of(username) == Some(uid))
    }

    async fn emit(
        &self,
        emitter: &SignalEmitter<'_>,
        action: &str,
        phase: u32,
        percent: u32,
        message: &str,
    ) {
        let _ = Self::progress(emitter, action, phase, percent, message).await;
    }

    async fn emit_finished(emitter: &SignalEmitter<'_>, action: &str) {
        let _ = Self::finished(emitter, action).await;
    }

    async fn apply(
        &self,
        emitter: &SignalEmitter<'_>,
        action: &str,
        user: &UserData,
    ) -> fdo::Result<()> {
        self.emit(emitter, action, 1, 30, "Escribiendo la cuenta").await;
        let old = std::fs::read_to_string(user_data::path_for(&user.username)).ok();
        user.write().map_err(fail)?;
        self.emit(emitter, action, 2, 50, "Reconstruyendo el sistema").await;
        if let Err(err) = rebuild() {
            restore(&user.username, old.as_deref(), None, None);
            return Err(fail(err));
        }
        self.emit(emitter, action, 3, 100, "Aplicado").await;
        Self::emit_finished(emitter, action).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().ok();
    let connection = zbus::connection::Builder::system()
        .wrap_err("no se pudo abrir el bus del sistema")?
        .name(BUS_NAME)
        .wrap_err("no se pudo tomar el nombre D-Bus")?
        .serve_at(OBJECT_PATH, Users::default())
        .wrap_err("no se pudo publicar la interfaz")?
        .build()
        .await
        .wrap_err("no se pudo conectar al bus del sistema")?;
    let _ = connection;
    std::future::pending::<()>().await;
    Ok(())
}

fn sender(header: &Header<'_>) -> fdo::Result<String> {
    header
        .sender()
        .map(|s| s.as_str().to_string())
        .ok_or_else(|| fdo::Error::Failed("llamada sin remitente".into()))
}

async fn authorize(connection: &Connection, sender: &str) -> fdo::Result<()> {
    let polkit = PolicyKit1Proxy::new(connection)
        .await
        .map_err(|e| fdo::Error::Failed(format!("polkit no disponible: {e}")))?;
    let mut subject = HashMap::new();
    subject.insert(
        "name".to_string(),
        OwnedValue::try_from(zbus::zvariant::Value::from(sender))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?,
    );
    let (authorized, _challenge, _details) = polkit
        .check_authorization(
            ("system-bus-name".to_string(), subject),
            ACTION,
            HashMap::new(),
            1,
            "",
        )
        .await
        .map_err(|e| fdo::Error::Failed(format!("polkit falló: {e}")))?;
    if authorized {
        Ok(())
    } else {
        Err(fdo::Error::AccessDenied(
            "se requiere autenticación de administrador".into(),
        ))
    }
}

fn fail(err: impl std::fmt::Display) -> fdo::Error {
    fdo::Error::Failed(err.to_string())
}

fn validate(username: &str, full_name: Option<&str>) -> fdo::Result<()> {
    if let Some(err) = user::username_error(username) {
        return Err(fdo::Error::Failed(format!("username inválido: {err}")));
    }
    if let Some(name) = full_name {
        if name.trim().is_empty() {
            return Err(fdo::Error::Failed("el nombre no puede estar vacío".into()));
        }
    }
    Ok(())
}

fn existing(username: &str) -> bool {
    user_data::path_for(username).exists() || passwd_line(username).is_some()
}

fn existing_user(username: &str) -> fdo::Result<UserData> {
    UserData::read(username)
        .map_err(fail)?
        .ok_or_else(|| fdo::Error::Failed(format!("el usuario '{username}' no existe")))
}

fn passwd_line(username: &str) -> Option<String> {
    let text = std::fs::read_to_string("/etc/passwd").ok()?;
    text.lines()
        .find(|line| line.split(':').next() == Some(username))
        .map(str::to_string)
}

fn uid_of(username: &str) -> Option<u32> {
    passwd_line(username)?
        .split(':')
        .nth(2)?
        .parse()
        .ok()
}

fn admin_count() -> Result<usize> {
    Ok(UserData::list()?.into_iter().filter(|u| u.admin).count())
}

fn user_has_processes(username: &str) -> Result<bool> {
    let Some(uid) = uid_of(username) else {
        return Ok(false);
    };
    let uid_line = format!("Uid:\t{uid}\t");
    for entry in std::fs::read_dir("/proc")? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        if !name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if let Ok(status) = std::fs::read_to_string(entry.path().join("status")) {
            if status.lines().any(|l| l.starts_with(&uid_line)) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn hash_path(username: &str) -> PathBuf {
    Path::new(SECRETS_DIR).join(format!("{username}.hash"))
}

fn write_hash(username: &str, hash: &str) -> Result<()> {
    let dir = Path::new(SECRETS_DIR);
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o711))?;
    let path = hash_path(username);
    let tmp = dir.join(format!(".{username}.hash.tmp"));
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        file.write_all(hash.as_bytes())?;
        file.sync_all()?;
    }
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

fn restore(username: &str, user: Option<&str>, hash: Option<&[u8]>, prefs: Option<&str>) {
    if let Some(text) = user {
        let _ = std::fs::write(user_data::path_for(username), text);
    }
    if let Some(bytes) = hash {
        let _ = write_hash(username, &String::from_utf8_lossy(bytes));
    }
    if let Some(text) = prefs {
        let _ = std::fs::write(user_data::prefs_path_for(username), text);
    }
}

fn system_ref() -> Result<(String, String)> {
    let flake_ref = std::fs::read_to_string("/etc/nixdots/flake-ref")
        .wrap_err("falta /etc/nixdots/flake-ref")?
        .trim()
        .to_string();
    let rev = std::fs::read_to_string("/etc/nixdots/rev")
        .wrap_err("falta /etc/nixdots/rev")?
        .trim()
        .to_string();
    if flake_ref.is_empty()
        || !Path::new(&flake_ref).is_dir()
        || !Path::new(&flake_ref).join("flake.nix").is_file()
    {
        bail!("referencia del sistema no válida: {flake_ref}");
    }
    if rev.is_empty() {
        bail!("revisión del sistema vacía");
    }
    Ok((flake_ref, rev))
}

fn rebuild() -> Result<()> {
    let (flake_ref, _rev) = system_ref()?;
    let ref_arg = format!("path:{flake_ref}#nixdots");
    let status = Command::new("nixos-rebuild")
        .args(["switch", "--impure", "--flake", &ref_arg])
        .env("NIXDOTS_ROOT", "/persist")
        .env("NIX_CONFIG", runner::nix_config())
        .status()
        .wrap_err("no se pudo ejecutar nixos-rebuild")?;
    if !status.success() {
        bail!("nixos-rebuild falló con {status}");
    }
    Ok(())
}

async fn rebuild_emitting(emitter: &SignalEmitter<'_>, action: &str) -> fdo::Result<()> {
    let _ = Users::progress(emitter, action, 2, 50, "Reconstruyendo el sistema").await;
    tokio::task::spawn_blocking(rebuild)
        .await
        .map_err(|e| fdo::Error::Failed(e.to_string()))?
        .map_err(fail)
}
