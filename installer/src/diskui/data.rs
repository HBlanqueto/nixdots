//! Disk data for the utility: `lsblk` tree, btrfs subvolumes/usages, and a
//! sample tree for the dry-run.

use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Subvol {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub name: String,
    pub size: u64,
    pub fstype: String,
    pub label: String,
    pub mountpoints: Vec<String>,
    pub kind: String,
    pub model: String,
    pub removable: bool,
    pub used: u64,
    pub children: Vec<Info>,
    pub subvolumes: Vec<Subvol>,
}

impl Info {
    fn total(&self) -> u64 {
        self.size
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub depth: usize,
    pub icon: char,
    pub label: String,
    pub size: u64,
    pub fstype: String,
    pub path: Vec<usize>,
    pub protected: bool,
    pub removable: bool,
    pub is_disk: bool,
}

pub struct Loaded {
    pub disks: Vec<Info>,
}

impl Loaded {
    fn find(&self, path: &[usize]) -> Option<&Info> {
        let mut nodes = &self.disks;
        let mut found = None;
        for &idx in path {
            let node = nodes.get(idx)?;
            found = Some(node);
            nodes = &node.children;
        }
        found
    }

    pub fn usage_for(&self, path: &[usize]) -> (u64, u64) {
        match self.find(path) {
            Some(info) => (info.used, info.total()),
            None => (0, 0),
        }
    }

    pub fn info_for(&self, path: &[usize]) -> Vec<(String, String)> {
        let Some(info) = self.find(path) else {
            return Vec::new();
        };
        let mount = if info.mountpoints.is_empty() {
            "—".to_string()
        } else {
            info.mountpoints.join(", ")
        };
        let label = if info.label.is_empty() { "—" } else { &info.label };
        let mut rows = vec![
            ("Montaje".to_string(), mount),
            ("Tipo".to_string(), info.kind.clone()),
            ("Capacidad".to_string(), human(info.total())),
            ("Dispositivo".to_string(), format!("/dev/{}", info.name)),
            ("Disponible".to_string(), human(info.total().saturating_sub(info.used))),
            ("Etiqueta".to_string(), label.to_string()),
            ("Usado".to_string(), human(info.used)),
            ("Compresión".to_string(), if info.kind == "btrfs" { "zstd" } else { "—" }.to_string()),
        ];
        if !info.model.is_empty() {
            rows.push(("Modelo".to_string(), info.model.clone()));
        }
        if !info.subvolumes.is_empty() {
            rows.push(("Subvolúmenes".to_string(), info.subvolumes.len().to_string()));
        }
        rows
    }
}

pub fn human(bytes: u64) -> String {
    crate::detect::human_size(bytes)
}

/// Loads the disk tree, or a sample in the dry-run.
pub fn load(dry_run: bool) -> Loaded {
    if dry_run {
        return Loaded { disks: sample() };
    }
    Loaded { disks: probe() }
}

fn probe() -> Vec<Info> {
    let Ok(output) = Command::new("lsblk")
        .args([
            "-J",
            "-b",
            "-o",
            "NAME,SIZE,FSTYPE,LABEL,MOUNTPOINTS,TYPE,MODEL,RM",
        ])
        .output()
    else {
        return Vec::new();
    };
    let Ok(tree) = serde_json::from_slice::<Lsblk>(&output.stdout) else {
        return Vec::new();
    };
    tree.blockdevices
        .into_iter()
        .filter(|d| d.kind.as_deref() == Some("disk"))
        .map(convert)
        .collect()
}

#[derive(Deserialize)]
struct Lsblk {
    #[serde(default)]
    blockdevices: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    fstype: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    mountpoints: Vec<Option<String>>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    rm: bool,
    #[serde(default)]
    children: Vec<Raw>,
}

fn convert(raw: Raw) -> Info {
    let kind = raw.fstype.clone().unwrap_or_else(|| "—".into());
    let name = raw.name.clone();
    let mut info = Info {
        name,
        size: raw.size.unwrap_or(0),
        fstype: raw.fstype.clone().unwrap_or_default(),
        label: raw.label.clone().unwrap_or_default(),
        mountpoints: raw.mountpoints.into_iter().flatten().collect(),
        kind,
        model: raw.model.clone().unwrap_or_default(),
        removable: raw.rm,
        used: 0,
        children: raw.children.into_iter().map(convert).collect(),
        subvolumes: Vec::new(),
    };
    if info.fstype == "btrfs" {
        info.subvolumes = btrfs_subvolumes(&info.mountpoints);
        info.used = btrfs_usage(&info.name).unwrap_or(0);
    }
    info
}

fn btrfs_subvolumes(mountpoints: &[String]) -> Vec<Subvol> {
    let Some(target) = mountpoints.first() else {
        return Vec::new();
    };
    let Ok(output) = Command::new("btrfs")
        .args(["subvolume", "list", target])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let path = line.split(" path ").nth(1)?.trim().to_string();
            Some(Subvol { name: path })
        })
        .collect()
}

fn btrfs_usage(name: &str) -> Option<u64> {
    let output = Command::new("btrfs")
        .args(["filesystem", "usage", "-b", &format!("/dev/{name}")])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Used:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn sample() -> Vec<Info> {
    let subvol = |name: &str| Subvol {
        name: name.to_string(),
    };
    vec![
        Info {
            name: "nvme0n1".into(),
            size: 500_107_862_016,
            fstype: String::new(),
            label: String::new(),
            mountpoints: Vec::new(),
            kind: "disco".into(),
            model: "Samsung SSD 980".into(),
            removable: false,
            used: 0,
            subvolumes: Vec::new(),
            children: vec![
                Info {
                    name: "nvme0n1p1".into(),
                    size: 1_073_741_824,
                    fstype: "vfat".into(),
                    label: "BOOT".into(),
                    mountpoints: vec!["/boot".into()],
                    kind: "vfat".into(),
                    model: String::new(),
                    removable: false,
                    used: 51_380_224,
                    children: Vec::new(),
                    subvolumes: Vec::new(),
                },
                Info {
                    name: "nvme0n1p2".into(),
                    size: 499_034_120_192,
                    fstype: "btrfs".into(),
                    label: "nixos".into(),
                    mountpoints: vec!["/".into()],
                    kind: "btrfs".into(),
                    model: String::new(),
                    removable: false,
                    used: 180_388_659_200,
                    children: Vec::new(),
                    subvolumes: vec![
                        subvol("@rootfs"),
                        subvol("@nix"),
                        subvol("@home"),
                        subvol("@persist"),
                        subvol("@log"),
                    ],
                },
            ],
        },
        Info {
            name: "sda".into(),
            size: 31_457_280_000,
            fstype: String::new(),
            label: String::new(),
            mountpoints: Vec::new(),
            kind: "disco".into(),
            model: "SanDisk Ultra".into(),
            removable: true,
            used: 0,
            subvolumes: Vec::new(),
            children: vec![Info {
                name: "sda1".into(),
                size: 31_457_280_000,
                fstype: "exfat".into(),
                label: "USB".into(),
                mountpoints: Vec::new(),
                kind: "exfat".into(),
                model: String::new(),
                removable: true,
                used: 2_000_000_000,
                children: Vec::new(),
                subvolumes: Vec::new(),
            }],
        },
    ]
}

/// Flattens the tree into sidebar rows.
pub fn flatten(loaded: &Loaded) -> Vec<Node> {
    let mut nodes = Vec::new();
    for (di, disk) in loaded.disks.iter().enumerate() {
        push_node(&mut nodes, disk, vec![di], 0, disk.removable);
    }
    nodes
}

fn push_node(nodes: &mut Vec<Node>, info: &Info, path: Vec<usize>, depth: usize, removable: bool) {
    let (icon, is_disk) = if depth == 0 {
        (if removable { '⏏' } else { '▤' }, true)
    } else if depth == 1 {
        ('◫', false)
    } else {
        ('▫', false)
    };
    let protected = is_protected(info);
    let display_type = if depth == 0 {
        "disco".to_string()
    } else if info.kind.is_empty() {
        "—".to_string()
    } else {
        info.kind.clone()
    };
    nodes.push(Node {
        depth,
        icon,
        label: info.name.clone(),
        size: info.size,
        fstype: display_type,
        path: path.clone(),
        protected,
        removable,
        is_disk,
    });
    for (ci, child) in info.children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(ci);
        push_node(nodes, child, child_path, depth + 1, removable);
    }
    for sub in &info.subvolumes {
        nodes.push(Node {
            depth: depth + 1,
            icon: '▫',
            label: sub.name.clone(),
            size: 0,
            fstype: "subvolumen".into(),
            path: path.clone(),
            protected,
            removable,
            is_disk: false,
        });
    }
}

fn is_protected(info: &Info) -> bool {
    if info
        .mountpoints
        .iter()
        .any(|m| matches!(m.as_str(), "/" | "/iso" | "/run/iso" | "/nix"))
    {
        return true;
    }
    info.children.iter().any(is_protected)
}
