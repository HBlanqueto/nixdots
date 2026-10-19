//! Lógica compartida y libre de TUI del instalador de Keel y del asistente
//! de primer arranque: leer/escribir `system/lib/defaults.nix`, secretos (clave de
//! recuperación y hash de contraseña) y ejecución de comandos con registro.

pub mod crypto;
pub mod machine;
pub mod paths;
pub mod rebuild;
pub mod runner;
pub mod settings;
pub mod store;
pub mod user;
pub mod user_data;
