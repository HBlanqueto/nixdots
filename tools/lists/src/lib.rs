//! Tablas embebidas que comparten el instalador y el asistente gráfico:
//! idiomas, teclados y zonas horarias.
//!
//! El build necesita las variables que fija el flake:
//! `NIXDOTS_LOCALES`, `NIXDOTS_LOCALE_DIR`, `NIXDOTS_XKB_BASE_LST` y
//! `NIXDOTS_ZONE1970_TAB`. Ni `core` ni el CLI `setup` dependen de este crate.

pub mod keyboard;
pub mod locale;
pub mod timezone;
