use crate::backend::IdentityBackendKind;
use crate::ids::Uid;

/// Identidad del usuario. Los campos marcados como privilegiados solo los
/// puede cambiar quien tenga `users.manage`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentitySpec {
    /// Backend elegido al crear el usuario; no cambia solo.
    pub backend: Option<IdentityBackendKind>,
    pub uid: Option<Uid>,
    pub real_name: Option<String>,
    pub shell: Option<String>,
    pub groups: Vec<String>,
    pub admin: bool,
}
