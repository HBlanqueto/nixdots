//! Información del equipo, solo lectura, para la pantalla «Este equipo».
//!
//! Prefiere `facter.json` (modelo de CPU y memoria instalada); si no está, cae
//! a `/proc/cpuinfo` y `/proc/meminfo`. La memoria se redondea al múltiplo de
//! 4 GB más cercano hacia arriba porque MemTotal/facter dan la memoria usable,
//! no la instalada.

use serde_json::Value;

/// Datos de la máquina; «—» si no se pudo leer algo.
pub struct Info {
    pub model: String,
    pub cpu: String,
    pub ram: String,
    pub host: String,
}

pub fn info() -> Info {
    let facter = facter();
    Info {
        model: model(),
        cpu: cpu(facter.as_ref()),
        ram: ram(facter.as_ref()),
        host: hostname(),
    }
}

fn facter() -> Option<Value> {
    let root = std::env::var("NIXDOTS_ROOT").unwrap_or_default();
    let path = format!("{root}/persist/nixdots/facter.json");
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn model() -> String {
    let dmi = |name: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{name}")).ok();
    let value = dmi("product_version")
        .or_else(|| dmi("product_name"))
        .or_else(|| {
            std::fs::read_to_string("/proc/device-tree/model")
                .ok()
                .map(|s| s.replace('\0', ""))
        });
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string())
}

fn cpu(facter: Option<&Value>) -> String {
    if let Some(name) = facter
        .and_then(|v| v.pointer("/hardware/cpu/0/model_name"))
        .and_then(Value::as_str)
    {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    let Ok(text) = std::fs::read_to_string("/proc/cpuinfo") else {
        return "—".to_string();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            if let Some((_, value)) = rest.split_once(':') {
                return value.trim().to_string();
            }
        }
    }
    "—".to_string()
}

fn ram(facter: Option<&Value>) -> String {
    let bytes = facter
        .and_then(smbios_installed_bytes)
        .or_else(|| facter.and_then(installed_bytes))
        .or_else(mem_total_bytes);
    match bytes {
        Some(b) => format!("{} GB", even_gb(b)),
        None => "—".to_string(),
    }
}

/// Suma de los módulos SMBIOS (tipo 17): memoria instalada de verdad. `size`
/// viene en KB y 0/ausente = ranura vacía.
fn smbios_installed_bytes(facter: &Value) -> Option<u64> {
    let devices = facter.pointer("/smbios/memory_device")?.as_array()?;
    let mut total = 0u64;
    for device in devices {
        total += device.get("size").and_then(Value::as_u64).unwrap_or(0) * 1024;
    }
    (total > 0).then_some(total)
}

/// Suma de los rangos `phys_mem` de facter (memoria disponible, no la total).
fn installed_bytes(facter: &Value) -> Option<u64> {
    let devices = facter.pointer("/hardware/memory")?.as_array()?;
    let mut total = 0u64;
    for device in devices {
        if let Some(resources) = device.get("resources").and_then(Value::as_array) {
            for res in resources {
                if res.get("type").and_then(Value::as_str) == Some("phys_mem") {
                    total += res.get("range").and_then(Value::as_u64).unwrap_or(0);
                }
            }
        }
    }
    (total > 0).then_some(total)
}

fn mem_total_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

/// Redondea al número par de GB más cercano (mínimo 2).
fn even_gb(bytes: u64) -> u64 {
    let gb = bytes as f64 / 1_073_741_824.0;
    (((gb / 2.0).round() as u64) * 2).max(2)
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string())
}
