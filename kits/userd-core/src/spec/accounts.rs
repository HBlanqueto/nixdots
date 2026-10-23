/// Overlay cosmético compatible con `org.freedesktop.Accounts`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountsSpec {
    pub icon: Option<String>,
    pub language: Option<String>,
    pub session: Option<String>,
    pub automatic_login: bool,
}
