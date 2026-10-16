//! Información del equipo, solo lectura, para la pantalla «Este equipo».

/// Datos de la máquina; cadena vacía si no se pudo leer algo.
pub struct Info {
    pub model: String,
    pub cpu: String,
    pub ram: String,
    pub host: String,
}

pub fn info() -> Info {
    Info {
        model: model(),
        cpu: cpu(),
        ram: ram(),
        host: hostname(),
    }
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

fn cpu() -> String {
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

fn ram() -> String {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return "—".to_string();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            return format!("{:.1} GiB", kb as f64 / 1_048_576.0);
        }
    }
    "—".to_string()
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string())
}
