//! Validación de nombre de usuario, compartida por el instalador y el asistente
//! de primer arranque.

/// Usuarios que ya existen en el sistema.
pub const RESERVED_USERS: &[&str] = &[
    "root",
    "nixos",
    "bin",
    "daemon",
    "nobody",
    "adm",
    "lp",
    "sync",
    "shutdown",
    "halt",
    "mail",
    "operator",
    "games",
    "ftp",
    "http",
    "systemd-network",
    "systemd-resolve",
    "systemd-timesync",
    "messagebus",
    "polkitd",
    "sshd",
    "dbus",
    "nscd",
    "tss",
];

/// Por qué `name` no es un nombre de usuario válido, si no lo es. Un `name`
/// vacío es inválido (el instalador decide antes si usa el generado).
pub fn username_error(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        Some(c) if c.is_ascii_digit() => return Some("no puede empezar por número"),
        _ => return Some("solo a-z, 0-9, - y _"),
    }
    if name.chars().count() > 32 {
        return Some("máximo 32 caracteres");
    }
    if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return Some("solo a-z, 0-9, - y _");
    }
    if RESERVED_USERS.contains(&name) {
        return Some("nombre reservado");
    }
    None
}
