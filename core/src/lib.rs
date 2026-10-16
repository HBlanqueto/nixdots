//! Lógica compartida y libre de TUI del instalador de nixdots y del asistente
//! de primer arranque: leer/escribir `settings.nix`, secretos (clave de
//! recuperación y hash de contraseña) y ejecución de comandos con registro.

pub mod crypto;
pub mod runner;
pub mod settings;
pub mod user;
pub mod user_data;
