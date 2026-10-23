//! Núcleo de `keel-userd`: identificadores, especificación KDL, jobs y
//! selección de backends. No habla D-Bus ni toca el sistema de archivos.

#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod backend;
pub mod error;
pub mod ids;
pub mod job;
pub mod kdl;
pub mod names;
pub mod spec;

pub use error::Error;
