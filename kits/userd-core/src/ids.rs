use std::fmt;
use std::str::FromStr;

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

/// Nombre de usuario válido según `[a-z_][a-z0-9_-]{0,31}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Username(String);

/// Error de validación de los identificadores.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdError {
    #[error("invalid user name `{0}`: expected [a-z_][a-z0-9_-]{{0,31}}")]
    InvalidUsername(String),
}

impl Username {
    /// Longitud máxima, sin contar el terminador.
    pub const MAX_LEN: usize = 32;

    /// Valida y construye un nombre de usuario.
    pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let invalid = || IdError::InvalidUsername(value.clone());

        if value.len() > Self::MAX_LEN {
            return Err(invalid());
        }

        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return Err(invalid());
        };

        if !(first.is_ascii_lowercase() || first == '_') {
            return Err(invalid());
        }

        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return Err(invalid());
        }

        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl FromStr for Username {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for Username {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Username {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// UID del sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uid(u32);

/// GID del sistema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Gid(u32);

/// Identificador de un job persistido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(u64);

macro_rules! numeric_id {
    ($name:ident, $inner:ty) => {
        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

numeric_id!(Uid, u32);
numeric_id!(Gid, u32);
numeric_id!(JobId, u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        for name in [
            "s4rch",
            "_service",
            "a",
            "user-name_1",
            "x".repeat(32).as_str(),
        ] {
            assert!(Username::new(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_invalid_names() {
        for name in [
            "",
            "Sergio",
            "1sergio",
            "-sergio",
            "sergio!",
            "ñ",
            "x".repeat(33).as_str(),
        ] {
            assert!(Username::new(name).is_err(), "{name}");
        }
    }

    #[test]
    fn username_serde_round_trip() {
        let name = Username::new("s4rch").expect("valid name");
        let json = serde_json::to_string(&name).expect("serializable");
        assert_eq!(json, "\"s4rch\"");
        let decoded: Username = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(decoded, name);
        assert!(serde_json::from_str::<Username>("\"Sergio\"").is_err());
    }
}
