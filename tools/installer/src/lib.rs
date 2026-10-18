//! Library for the nixdots installer (`nixdots-installer`).

pub mod app;
pub mod detect;
pub mod disk;
pub mod install;
pub mod keyboard;
pub mod logo;
pub mod net;
pub mod screens;
pub mod theme;
pub mod widgets;

// Shared, TUI-free logic lives in nixdots-common (also used by the first-boot
// assistant); re-exported so `crate::settings`/`runner`/`crypto` keep resolving.
pub use nixdots_common::{crypto, runner, settings};
