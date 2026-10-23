use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use crate::Error;
use crate::backend::PackageBackendKind;
use crate::ids::{Uid, Username};
use crate::spec::{
    AccountsSpec, HomeSpec, IdentitySpec, PackageRef, PackagesSpec, SCHEMA_VERSION, UserSpec,
};

impl UserSpec {
    /// Parsea un documento KDL v2 con el esquema de la versión 1.
    pub fn from_kdl(text: &str) -> Result<Self, Error> {
        let document = KdlDocument::parse(text)
            .map_err(|error| invalid(format!("KDL syntax error: {error}")))?;
        Self::try_from(&document)
    }

    /// Serializa el usuario a KDL v2.
    pub fn to_kdl_string(&self) -> Result<String, Error> {
        let document = KdlDocument::try_from(self)?;
        Ok(document.to_string())
    }
}

impl TryFrom<&KdlDocument> for UserSpec {
    type Error = Error;

    fn try_from(document: &KdlDocument) -> Result<Self, Self::Error> {
        let mut version = None;
        let mut user = None;

        for node in document.nodes() {
            match node.name().value() {
                "version" => {
                    if version.replace(parse_version(node)?).is_some() {
                        return Err(duplicate("version"));
                    }
                }
                "user" => {
                    if user.replace(parse_user(node)?).is_some() {
                        return Err(duplicate("user"));
                    }
                }
                other => return Err(unknown_node(other)),
            }
        }

        let version = version.ok_or_else(|| invalid("missing `version`"))?;
        if version != SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported schema version {version}, expected {SCHEMA_VERSION}"
            )));
        }

        let (name, identity, accounts, home, packages) =
            user.ok_or_else(|| invalid("missing `user`"))?;

        Ok(Self {
            version,
            name,
            identity,
            accounts,
            home,
            packages,
        })
    }
}

impl TryFrom<&UserSpec> for KdlDocument {
    type Error = Error;

    fn try_from(spec: &UserSpec) -> Result<Self, Self::Error> {
        if spec.version != SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported schema version {}, expected {SCHEMA_VERSION}",
                spec.version
            )));
        }

        let mut document = KdlDocument::new();

        let mut version = KdlNode::new("version");
        version.push(KdlValue::Integer(i128::from(spec.version)));
        document.nodes_mut().push(version);

        let mut user = KdlNode::new("user");
        user.push(KdlValue::String(spec.name.as_str().to_owned()));
        user.set_children(sections(spec)?);
        document.nodes_mut().push(user);

        Ok(document)
    }
}

type Sections = (Username, IdentitySpec, AccountsSpec, HomeSpec, PackagesSpec);

fn parse_version(node: &KdlNode) -> Result<u32, Error> {
    let value = entry_value(node, "version")?;
    let raw = value
        .as_integer()
        .ok_or_else(|| invalid("`version` must be an integer"))?;
    u32::try_from(raw).map_err(|_| invalid("`version` is out of range"))
}

fn parse_user(node: &KdlNode) -> Result<Sections, Error> {
    let name = string_entry(node, "user")?;
    let name = Username::new(name).map_err(|error| invalid(error.to_string()))?;

    let mut identity = IdentitySpec::default();
    let mut accounts = AccountsSpec::default();
    let mut home = HomeSpec::default();
    let mut packages = PackagesSpec::default();
    let mut seen_identity = false;
    let mut seen_accounts = false;
    let mut seen_home = false;
    let mut seen_packages = false;

    for child in child_nodes(node) {
        match child.name().value() {
            "identity" => {
                if std::mem::replace(&mut seen_identity, true) {
                    return Err(duplicate("identity"));
                }
                identity = parse_identity(child)?;
            }
            "accounts" => {
                if std::mem::replace(&mut seen_accounts, true) {
                    return Err(duplicate("accounts"));
                }
                accounts = parse_accounts(child)?;
            }
            "home" => {
                if std::mem::replace(&mut seen_home, true) {
                    return Err(duplicate("home"));
                }
                home = parse_home(child)?;
            }
            "packages" => {
                if std::mem::replace(&mut seen_packages, true) {
                    return Err(duplicate("packages"));
                }
                packages = parse_packages(child)?;
            }
            other => return Err(unknown_node(other)),
        }
    }

    Ok((name, identity, accounts, home, packages))
}

fn parse_identity(node: &KdlNode) -> Result<IdentitySpec, Error> {
    no_entries(node, "identity")?;
    let mut spec = IdentitySpec::default();
    let mut seen_groups = false;
    let mut seen_admin = false;

    for child in child_nodes(node) {
        match child.name().value() {
            "backend" => {
                if spec.backend.is_some() {
                    return Err(duplicate("identity.backend"));
                }
                spec.backend = Some(string_entry(child, "identity.backend")?.parse()?);
            }
            "uid" => {
                if spec.uid.is_some() {
                    return Err(duplicate("identity.uid"));
                }
                spec.uid = Some(Uid::new(u32_entry(child, "identity.uid")?));
            }
            "real-name" => {
                if spec.real_name.is_some() {
                    return Err(duplicate("identity.real-name"));
                }
                spec.real_name = Some(string_entry(child, "identity.real-name")?);
            }
            "shell" => {
                if spec.shell.is_some() {
                    return Err(duplicate("identity.shell"));
                }
                spec.shell = Some(string_entry(child, "identity.shell")?);
            }
            "groups" => {
                if std::mem::replace(&mut seen_groups, true) {
                    return Err(duplicate("identity.groups"));
                }
                spec.groups = string_args(child, "identity.groups")?;
            }
            "admin" => {
                if std::mem::replace(&mut seen_admin, true) {
                    return Err(duplicate("identity.admin"));
                }
                spec.admin = bool_entry(child, "identity.admin")?;
            }
            other => return Err(unknown_node(other)),
        }
    }

    Ok(spec)
}

fn parse_accounts(node: &KdlNode) -> Result<AccountsSpec, Error> {
    no_entries(node, "accounts")?;
    let mut spec = AccountsSpec::default();
    let mut seen_login = false;

    for child in child_nodes(node) {
        match child.name().value() {
            "icon" => {
                if spec.icon.is_some() {
                    return Err(duplicate("accounts.icon"));
                }
                spec.icon = Some(string_entry(child, "accounts.icon")?);
            }
            "language" => {
                if spec.language.is_some() {
                    return Err(duplicate("accounts.language"));
                }
                spec.language = Some(string_entry(child, "accounts.language")?);
            }
            "session" => {
                if spec.session.is_some() {
                    return Err(duplicate("accounts.session"));
                }
                spec.session = Some(string_entry(child, "accounts.session")?);
            }
            "automatic-login" => {
                if std::mem::replace(&mut seen_login, true) {
                    return Err(duplicate("accounts.automatic-login"));
                }
                spec.automatic_login = bool_entry(child, "accounts.automatic-login")?;
            }
            other => return Err(unknown_node(other)),
        }
    }

    Ok(spec)
}

fn parse_home(node: &KdlNode) -> Result<HomeSpec, Error> {
    no_entries(node, "home")?;
    let mut spec = HomeSpec::default();

    for child in child_nodes(node) {
        if child.name().value() != "option" {
            return Err(unknown_node(child.name().value()));
        }

        let entries = child.entries();
        if entries.len() != 2 || entries.iter().any(|entry| entry.name().is_some()) {
            return Err(invalid("`home.option` expects a path and a scalar value"));
        }

        let path = entries[0]
            .value()
            .as_string()
            .ok_or_else(|| invalid("`home.option` path must be a string"))?;
        if path.is_empty() {
            return Err(invalid("`home.option` path cannot be empty"));
        }

        let value = json_value(entries[1].value())?;
        if spec.options.insert(path.to_owned(), value).is_some() {
            return Err(invalid(format!("duplicate home option `{path}`")));
        }
    }

    Ok(spec)
}

fn parse_packages(node: &KdlNode) -> Result<PackagesSpec, Error> {
    no_entries(node, "packages")?;
    let mut spec = PackagesSpec::default();

    for child in child_nodes(node) {
        if child.name().value() != "package" {
            return Err(unknown_node(child.name().value()));
        }

        let entries = child.entries();
        let Some(id_entry) = entries.first() else {
            return Err(invalid("`package` expects an id"));
        };
        if id_entry.name().is_some() {
            return Err(invalid("`package` expects an id as its first value"));
        }
        let id = id_entry
            .value()
            .as_string()
            .ok_or_else(|| invalid("`package` id must be a string"))?;
        if id.is_empty() {
            return Err(invalid("`package` id cannot be empty"));
        }

        let mut backend = None;
        let mut attr = None;
        let mut remote = None;

        for entry in &entries[1..] {
            let Some(name) = entry.name() else {
                return Err(invalid("`package` only accepts one positional id"));
            };
            match name.value() {
                "backend" => {
                    if backend.is_some() {
                        return Err(duplicate("package.backend"));
                    }
                    backend = Some(parse_backend(entry, "package.backend")?);
                }
                "attr" => {
                    if attr.is_some() {
                        return Err(duplicate("package.attr"));
                    }
                    attr = Some(string_value(entry, "package.attr")?);
                }
                "remote" => {
                    if remote.is_some() {
                        return Err(duplicate("package.remote"));
                    }
                    remote = Some(string_value(entry, "package.remote")?);
                }
                other => return Err(invalid(format!("unknown package property `{other}`"))),
            }
        }

        spec.packages.push(PackageRef {
            id: id.to_owned(),
            backend: backend.ok_or_else(|| invalid("`package` requires a backend"))?,
            attr,
            remote,
        });
    }

    Ok(spec)
}

fn parse_backend(entry: &KdlEntry, context: &str) -> Result<PackageBackendKind, Error> {
    string_value(entry, context)?.parse()
}

fn sections(spec: &UserSpec) -> Result<KdlDocument, Error> {
    let mut sections = KdlDocument::new();
    sections
        .nodes_mut()
        .push(section("identity", identity_children(spec)));
    sections
        .nodes_mut()
        .push(section("accounts", accounts_children(spec)));
    sections
        .nodes_mut()
        .push(section("home", home_children(spec)?));
    sections
        .nodes_mut()
        .push(section("packages", packages_children(spec)));
    Ok(sections)
}

fn identity_children(spec: &UserSpec) -> KdlDocument {
    let mut children = KdlDocument::new();
    if let Some(backend) = spec.identity.backend {
        children
            .nodes_mut()
            .push(node_with_arg("backend", backend.as_str()));
    }
    if let Some(uid) = spec.identity.uid {
        let mut node = KdlNode::new("uid");
        node.push(KdlValue::Integer(i128::from(uid.get())));
        children.nodes_mut().push(node);
    }
    if let Some(real_name) = &spec.identity.real_name {
        children
            .nodes_mut()
            .push(node_with_arg("real-name", real_name));
    }
    if let Some(shell) = &spec.identity.shell {
        children.nodes_mut().push(node_with_arg("shell", shell));
    }
    let mut groups = KdlNode::new("groups");
    for group in &spec.identity.groups {
        groups.push(KdlValue::String(group.clone()));
    }
    children.nodes_mut().push(groups);
    let mut admin = KdlNode::new("admin");
    admin.push(KdlValue::Bool(spec.identity.admin));
    children.nodes_mut().push(admin);
    children
}

fn accounts_children(spec: &UserSpec) -> KdlDocument {
    let mut children = KdlDocument::new();
    if let Some(icon) = &spec.accounts.icon {
        children.nodes_mut().push(node_with_arg("icon", icon));
    }
    if let Some(language) = &spec.accounts.language {
        children
            .nodes_mut()
            .push(node_with_arg("language", language));
    }
    if let Some(session) = &spec.accounts.session {
        children.nodes_mut().push(node_with_arg("session", session));
    }
    let mut login = KdlNode::new("automatic-login");
    login.push(KdlValue::Bool(spec.accounts.automatic_login));
    children.nodes_mut().push(login);
    children
}

fn home_children(spec: &UserSpec) -> Result<KdlDocument, Error> {
    let mut children = KdlDocument::new();
    for (path, value) in &spec.home.options {
        let mut node = KdlNode::new("option");
        node.push(KdlValue::String(path.clone()));
        node.push(kdl_value(value)?);
        children.nodes_mut().push(node);
    }
    Ok(children)
}

fn packages_children(spec: &UserSpec) -> KdlDocument {
    let mut children = KdlDocument::new();
    for package in &spec.packages.packages {
        let mut node = KdlNode::new("package");
        node.push(KdlValue::String(package.id.clone()));
        node.insert("backend", package.backend.as_str());
        if let Some(attr) = &package.attr {
            node.insert("attr", attr.clone());
        }
        if let Some(remote) = &package.remote {
            node.insert("remote", remote.clone());
        }
        children.nodes_mut().push(node);
    }
    children
}

fn section(name: &str, children: KdlDocument) -> KdlNode {
    let mut node = KdlNode::new(name);
    node.set_children(children);
    node
}

fn node_with_arg(name: &str, value: &str) -> KdlNode {
    let mut node = KdlNode::new(name);
    node.push(KdlValue::String(value.to_owned()));
    node
}

fn json_value(value: &KdlValue) -> Result<serde_json::Value, Error> {
    Ok(match value {
        KdlValue::String(value) => serde_json::Value::String(value.clone()),
        KdlValue::Integer(value) => {
            let value = i64::try_from(*value)
                .map_err(|_| invalid("home option integer is out of range"))?;
            serde_json::Value::Number(value.into())
        }
        KdlValue::Float(value) => {
            let number = serde_json::Number::from_f64(*value)
                .ok_or_else(|| invalid("home option float must be finite"))?;
            serde_json::Value::Number(number)
        }
        KdlValue::Bool(value) => serde_json::Value::Bool(*value),
        KdlValue::Null => serde_json::Value::Null,
    })
}

fn kdl_value(value: &serde_json::Value) -> Result<KdlValue, Error> {
    Ok(match value {
        serde_json::Value::Null => KdlValue::Null,
        serde_json::Value::Bool(value) => KdlValue::Bool(*value),
        serde_json::Value::String(value) => KdlValue::String(value.clone()),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                KdlValue::Integer(i128::from(value))
            } else if let Some(value) = value.as_f64() {
                KdlValue::Float(value)
            } else {
                return Err(invalid("unsupported home option number"));
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            return Err(invalid("home options only accept scalar values"));
        }
    })
}

fn entry_value<'a>(node: &'a KdlNode, context: &str) -> Result<&'a KdlValue, Error> {
    match node.entries() {
        [entry] if entry.name().is_none() => Ok(entry.value()),
        _ => Err(invalid(format!(
            "`{context}` expects exactly one positional value"
        ))),
    }
}

fn string_entry(node: &KdlNode, context: &str) -> Result<String, Error> {
    entry_value(node, context)?
        .as_string()
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid(format!("`{context}` expects a string")))
}

fn bool_entry(node: &KdlNode, context: &str) -> Result<bool, Error> {
    entry_value(node, context)?
        .as_bool()
        .ok_or_else(|| invalid(format!("`{context}` expects a boolean")))
}

fn u32_entry(node: &KdlNode, context: &str) -> Result<u32, Error> {
    let raw = entry_value(node, context)?
        .as_integer()
        .ok_or_else(|| invalid(format!("`{context}` expects an integer")))?;
    u32::try_from(raw).map_err(|_| invalid(format!("`{context}` is out of range")))
}

fn string_value(entry: &KdlEntry, context: &str) -> Result<String, Error> {
    entry
        .value()
        .as_string()
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid(format!("`{context}` expects a string")))
}

fn string_args(node: &KdlNode, context: &str) -> Result<Vec<String>, Error> {
    node.entries()
        .iter()
        .map(|entry| {
            if entry.name().is_some() {
                return Err(invalid(format!("`{context}` only accepts strings")));
            }
            string_value(entry, context)
        })
        .collect()
}

fn no_entries(node: &KdlNode, context: &str) -> Result<(), Error> {
    if node.entries().is_empty() {
        Ok(())
    } else {
        Err(invalid(format!("`{context}` does not accept values")))
    }
}

fn child_nodes(node: &KdlNode) -> &[KdlNode] {
    node.children().map_or(&[], KdlDocument::nodes)
}

fn duplicate(what: &str) -> Error {
    invalid(format!("duplicate `{what}`"))
}

fn unknown_node(name: &str) -> Error {
    invalid(format!("unknown node `{name}`"))
}

fn invalid(message: impl Into<String>) -> Error {
    Error::InvalidConfig(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::IdentityBackendKind;

    const SAMPLE: &str = r#"
version 1

user "s4rch" {
    identity {
        backend "homed"
        uid 60001
        real-name "Sergio"
        shell "/run/current-system/sw/bin/nu"
        groups "video" "audio"
        admin #false
    }

    accounts {
        icon "/var/lib/AccountsService/icons/s4rch"
        language "es_BO.UTF-8"
        session "niri"
        automatic-login #false
    }

    home {
        option "programs.git.enable" #true
        option "programs.git.userName" "Sergio"
    }

    packages {
        package "ripgrep" backend="nix-profile" attr="nixpkgs#ripgrep"
        package "helix" backend="home-manager" attr="helix"
        package "org.mozilla.firefox" backend="flatpak" remote="flathub"
    }
}
"#;

    #[test]
    fn parses_the_sample() {
        let spec = UserSpec::from_kdl(SAMPLE).expect("valid sample");
        assert_eq!(spec.version, 1);
        assert_eq!(spec.name.as_str(), "s4rch");
        assert_eq!(spec.identity.backend, Some(IdentityBackendKind::Homed));
        assert_eq!(spec.identity.uid, Some(Uid::new(60_001)));
        assert_eq!(spec.identity.real_name.as_deref(), Some("Sergio"));
        assert_eq!(spec.identity.groups, ["video", "audio"]);
        assert!(!spec.identity.admin);
        assert_eq!(spec.accounts.session.as_deref(), Some("niri"));
        assert_eq!(spec.home.options.len(), 2);
        assert_eq!(
            spec.home.options["programs.git.enable"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(spec.packages.packages.len(), 3);
        assert_eq!(
            spec.packages.packages[2].backend,
            PackageBackendKind::Flatpak
        );
        assert_eq!(spec.packages.packages[2].remote.as_deref(), Some("flathub"));
    }

    #[test]
    fn round_trips_through_kdl() {
        let spec = UserSpec::from_kdl(SAMPLE).expect("valid sample");
        let text = spec.to_kdl_string().expect("serializable");
        let reparsed = UserSpec::from_kdl(&text).expect("reparsable");
        assert_eq!(reparsed, spec);

        let document = KdlDocument::try_from(&spec).expect("document");
        let from_document = UserSpec::try_from(&document).expect("spec");
        assert_eq!(from_document, spec);
    }

    #[test]
    fn version_is_required() {
        let error = UserSpec::from_kdl("user \"s4rch\" {\n}\n").expect_err("missing version");
        assert!(matches!(error, Error::InvalidConfig(message) if message.contains("version")));
    }

    #[test]
    fn unknown_version_is_rejected() {
        let error =
            UserSpec::from_kdl("version 2\nuser \"s4rch\" {\n}\n").expect_err("bad version");
        assert!(matches!(error, Error::InvalidConfig(message) if message.contains("unsupported")));
    }

    #[test]
    fn unknown_nodes_are_rejected() {
        let error = UserSpec::from_kdl(
            "version 1\nuser \"s4rch\" {\n  identity {\n    shell-x \"/bin/sh\"\n  }\n}\n",
        )
        .expect_err("unknown node");
        assert!(matches!(error, Error::InvalidConfig(_)));
    }

    #[test]
    fn invalid_user_name_is_rejected() {
        let error = UserSpec::from_kdl("version 1\nuser \"Sergio\" {\n}\n").expect_err("bad name");
        assert!(matches!(error, Error::InvalidConfig(_)));
    }

    #[test]
    fn package_requires_backend() {
        let error = UserSpec::from_kdl(
            "version 1\nuser \"s4rch\" {\n  packages {\n    package \"ripgrep\"\n  }\n}\n",
        )
        .expect_err("missing backend");
        assert!(matches!(error, Error::InvalidConfig(_)));
    }
}
