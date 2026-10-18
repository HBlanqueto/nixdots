//! `nixdots-setup-gui` — asistente gráfico de primer arranque (Slint).
//!
//! No duplica lógica: recoge los datos del modelo de la UI, escribe un JSON
//! temporal y ejecuta el CLI `nixdots-setup apply` (con `--dry-run` en `--sim`),
//! mostrando su salida y deduciendo la fase.

slint::include_modules!();

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use slint::{ComponentHandle, Model, ModelRc, SharedString};

mod apply;
mod wallpaper;

fn main() -> Result<(), slint::PlatformError> {
    let mut sim = false;
    let mut repo = "/persist/nixdots/repo".to_string();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--sim" => sim = true,
            "--repo" => {
                if let Some(v) = it.next() {
                    repo = v;
                }
            }
            _ => {}
        }
    }

    let ui = AppWindow::new()?;

    // Lista de locales (todos los UTF-8 de glibc, nombre nativo + código) y
    // selección inicial es_MX (o el primero).
    fill_locales(&ui, "");
    {
        let st = ui.global::<State>();
        let locales = locales();
        let idx = locales
            .iter()
            .position(|l| l.code == "es_MX.UTF-8")
            .unwrap_or(0);
        if let Some(l) = locales.get(idx) {
            st.set_lang_selected(l.code.clone().into());
            st.set_lang_selected_name(l.name.clone().into());
        }
    }
    // Lista de ciudades (filtrable) y ciudad inicial.
    fill_cities(&ui, "");

    // Idioma por defecto del asistente: inglés. Se cambia en vivo al elegir
    // idioma (las traducciones van embebidas desde i18n/*.po); si el idioma no
    // tiene .po, se muestra la interfaz en inglés.
    let _ = slint::select_bundled_translation("en");
    ui.on_language_changed({
        let weak = ui.as_weak();
        move |code| {
            let code = code.to_string();
            let base = code.split(['_', '-']).next().unwrap_or("en");
            if slint::select_bundled_translation(base).is_err() {
                let _ = slint::select_bundled_translation("en");
            }
            if let Some(ui) = weak.upgrade() {
                let name = locales()
                    .iter()
                    .find(|l| l.code == code)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| code.clone());
                ui.global::<State>().set_lang_selected_name(name.into());
            }
        }
    });
    // Buscador de idioma: filtra por nombre o código.
    ui.on_lang_search_changed({
        let weak = ui.as_weak();
        move |query| {
            if let Some(ui) = weak.upgrade() {
                fill_locales(&ui, &query);
            }
        }
    });
    // Buscador de ciudad: filtra por nombre o zona.
    ui.on_city_search_changed({
        let weak = ui.as_weak();
        move |query| {
            if let Some(ui) = weak.upgrade() {
                fill_cities(&ui, &query);
            }
        }
    });

    // Fondo desenfocado una vez (con caché en /run/psamathe).
    if let Some(image) = wallpaper::load() {
        ui.set_wallpaper(image);
    }

    // Preferencia del sistema para la reducción de movimiento (GNOME).
    ui.global::<Theme>().set_reduced_motion(reduced_motion());

    // «Reducir animaciones» en vivo: el saludo cambia el ritmo (palabra completa
    // cada 2,5 s en lugar de máquina de escribir).
    let reduce = Arc::new(AtomicBool::new(
        ui.global::<Theme>().get_reduced_motion() || ui.global::<State>().get_a11y_reduce_motion(),
    ));
    ui.on_reduce_changed({
        let reduce = reduce.clone();
        move |on| reduce.store(on, Ordering::Relaxed)
    });

    // Saludo del paso 0: máquina de escribir (letra a letra) y luego fundido.
    spawn_greeting(&ui, reduce);

    // Esc cierra la ventana solo en simulación. En modo real el asistente lo
    // relanza lib/data.nix al salir con 0 y reiniciaría el equipo, así que se
    // ignora: para salir están «Apagar» y «Reiniciar».
    ui.on_quit_app({
        let weak = ui.as_weak();
        move || {
            if !sim {
                return;
            }
            if let Some(ui) = weak.upgrade() {
                let _ = ui.hide();
            }
        }
    });

    // Validación del usuario con nixdots_core, y derivación desde el nombre.
    let user_edited = Rc::new(Cell::new(false));
    ui.on_username_edited({
        let user_edited = user_edited.clone();
        let weak = ui.as_weak();
        move |text| {
            user_edited.set(true);
            if let Some(ui) = weak.upgrade() {
                let err = nixdots_core::user::username_error(&text).unwrap_or("");
                ui.global::<State>().set_username_error(err.into());
            }
        }
    });
    ui.on_name_edited({
        let user_edited = user_edited.clone();
        let weak = ui.as_weak();
        move |text| {
            if user_edited.get() {
                return;
            }
            if let Some(ui) = weak.upgrade() {
                let st = ui.global::<State>();
                let full = text.to_string();
                let username = nixdots_core::user::generate_username(&full);
                st.set_username(username.clone().into());
                st.set_initials(initials(&full).into());
                st.set_first_name(first_word(&full).into());
                let err = nixdots_core::user::username_error(&username).unwrap_or("");
                st.set_username_error(err.into());
            }
        }
    });

    // Medidor de seguridad de la contraseña y coincidencia (paso Cuenta).
    ui.on_passwords_changed({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let st = ui.global::<State>();
                let p = st.get_password().to_string();
                let p2 = st.get_password2().to_string();
                st.set_pw_strength(password_strength(&p));
                st.set_pw_match(p2.is_empty() || p2 == p);
            }
        }
    });

    // Wi‑Fi: conecta con NetworkManager. En simulación sólo simula la espera.
    ui.on_wifi_connect({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let (ssid, pass, idx) = {
                let st = ui.global::<State>();
                let hidden = st.get_wifi_hidden();
                let ssid = if hidden {
                    st.get_wifi_ssid().to_string()
                } else {
                    pick(st.get_net_names(), st.get_net_index())
                };
                let idx = if hidden { -1 } else { st.get_net_index() };
                (ssid, st.get_wifi_pass().to_string(), idx)
            };
            let weak2 = weak.clone();
            std::thread::spawn(move || {
                let ok = if sim {
                    std::thread::sleep(std::time::Duration::from_millis(1100));
                    true
                } else {
                    nmcli_connect(&ssid, &pass)
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak2.upgrade() {
                        let st = ui.global::<State>();
                        st.set_wifi_join(false);
                        if ok {
                            st.set_wifi_connected(idx);
                            st.set_wifi_pw(false);
                            st.set_wifi_error(false);
                        } else {
                            st.set_wifi_error(true);
                        }
                    }
                });
            });
        }
    });

    // «Aplicar»: escribe el JSON y lanza el CLI, deduciendo la fase.
    ui.on_apply({
        let weak = ui.as_weak();
        let repo = repo.clone();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let (json, password) = {
                let st = ui.global::<State>();
                if st.get_applying() {
                    return;
                }
                st.set_applying(true);
                st.set_apply_error("".into());
                st.set_show_details(false);
                st.set_log_text("".into());
                st.set_prep_phase(0);
                st.set_prep_black(false);
                st.set_step(9);
                let json = build_json(&st, &ui.global::<Theme>(), &ui.global::<Tokens>());
                (json, st.get_password().to_string())
            };
            drop(ui);

            let weak_line = weak.clone();
            let on_line = move |line: String| {
                let weak = weak_line.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = weak.upgrade() {
                        let st = ui.global::<State>();
                        let clean = strip_ansi(&line);
                        let trimmed = clean.trim();
                        if let Some(rest) = trimmed.strip_prefix("PHASE ") {
                            if let Ok(n) = rest.trim().parse::<i32>() {
                                if n > st.get_prep_phase() {
                                    st.set_prep_phase(n);
                                }
                            }
                            return;
                        }
                        if let Some(msg) = trimmed.strip_prefix("ERROR: ") {
                            let msg = msg.trim().to_string();
                            st.set_error_step(if msg.to_lowercase().contains("contraseña") {
                                4
                            } else {
                                9
                            });
                            st.set_apply_error(msg.into());
                        }
                        let mut log = st.get_log_text().to_string();
                        if !log.is_empty() {
                            log.push('\n');
                        }
                        log.push_str(&clean);
                        st.set_log_text(log.into());
                    }
                });
            };

            let weak_done = weak.clone();
            let on_done = move |ok: bool| {
                let weak = weak_done.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(ui) = weak.upgrade() else { return };
                    let st = ui.global::<State>();
                    st.set_applying(false);
                    if !ok {
                        st.set_show_details(true);
                        if st.get_apply_error().is_empty() {
                            st.set_error_step(8);
                            st.set_apply_error("No se pudo terminar la configuración.".into());
                        }
                        return;
                    }
                    st.set_prep_phase(4);
                    // Fundido a negro y reinicio (en sim se vuelve al saludo).
                    let weak2 = weak.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        let _ = slint::invoke_from_event_loop({
                            let w = weak2.clone();
                            move || {
                                if let Some(ui) = w.upgrade() {
                                    ui.global::<State>().set_prep_black(true);
                                }
                            }
                        });
                        std::thread::sleep(std::time::Duration::from_millis(1600));
                        let _ = slint::invoke_from_event_loop({
                            let w = weak2.clone();
                            move || {
                                if let Some(ui) = w.upgrade() {
                                    if sim {
                                        let st = ui.global::<State>();
                                        st.set_prep_black(false);
                                        st.set_prep_phase(0);
                                        st.set_step(0);
                                    } else {
                                        let _ = std::process::Command::new("systemctl")
                                            .arg("reboot")
                                            .status();
                                    }
                                }
                            }
                        });
                    });
                });
            };

            if let Err(e) = apply::start(&json, &repo, sim, &password, on_line, on_done) {
                if let Some(ui) = weak.upgrade() {
                    let st = ui.global::<State>();
                    st.set_applying(false);
                    st.set_show_details(true);
                    st.set_apply_error(e.into());
                }
            }
        }
    });

    ui.run()
}

/// Quita secuencias ANSI (`ESC [ ... letra`) para mostrar el log limpio.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Aplica un cambio al `State` desde otro hilo.
fn set_state(weak: &slint::Weak<AppWindow>, f: impl FnOnce(State) + Send + 'static) {
    let weak = weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(ui) = weak.upgrade() {
            f(ui.global::<State>());
        }
    });
}

/// Máquina de escribir del saludo: escribe cada palabra letra a letra (una cada
/// 110 ms, contando caracteres Unicode, no bytes) y luego la funde 450 ms antes
/// de pasar a la siguiente. Con «Reducir animaciones» muestra la palabra entera
/// cada 2,5 s.
fn spawn_greeting(ui: &AppWindow, reduce: Arc<AtomicBool>) {
    let greets = ui.global::<State>().get_greets();
    let words: Vec<String> = (0..greets.row_count())
        .map(|i| {
            greets
                .row_data(i)
                .map(|g| g.word.to_string())
                .unwrap_or_default()
        })
        .collect();
    if words.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        // Deja arrancar el bucle de eventos antes del primer invoke.
        std::thread::sleep(std::time::Duration::from_millis(300));
        // Presentación «Welcome to NixOS» (siempre en inglés): funde de entrada,
        // se queda ~2 s y funde; luego empieza la máquina de escribir. El
        // upgrade del Weak va dentro de `set_state` (en el hilo del bucle de
        // eventos): desde otro hilo siempre devolvería None.
        set_state(&weak, |st| st.set_intro_op(1.0));
        std::thread::sleep(std::time::Duration::from_millis(2000));
        set_state(&weak, |st| st.set_intro_op(0.0));
        std::thread::sleep(std::time::Duration::from_millis(750));
        let n = words.len();
        let mut i = 0usize;
        loop {
            let word = words[i].clone();
            let is_reduced = reduce.load(Ordering::Relaxed);
            if is_reduced {
                let full = word.clone();
                set_state(&weak, move |st| {
                    st.set_gi(i as i32);
                    st.set_greet_typed(full.into());
                    st.set_greet_animate(true);
                    st.set_g_shown(true);
                });
                std::thread::sleep(std::time::Duration::from_millis(2500));
                i = (i + 1) % n;
                continue;
            }
            let chars: Vec<char> = word.chars().collect();
            set_state(&weak, move |st| {
                st.set_gi(i as i32);
                st.set_greet_typed("".into());
                st.set_greet_animate(true);
                st.set_g_shown(true);
            });
            let mut prefix = String::new();
            for c in chars {
                prefix.push(c);
                let p = prefix.clone();
                set_state(&weak, move |st| st.set_greet_typed(p.into()));
                std::thread::sleep(std::time::Duration::from_millis(110));
            }
            std::thread::sleep(std::time::Duration::from_millis(1800));
            set_state(&weak, |st| st.set_g_shown(false));
            // 450 ms de fundido + 50 ms antes de la siguiente palabra.
            std::thread::sleep(std::time::Duration::from_millis(500));
            i = (i + 1) % n;
        }
    });
}

/// Construye el JSON que espera `nixdots-setup apply`.
fn build_json(st: &State, theme: &Theme, tokens: &Tokens) -> String {
    let mut map = serde_json::Map::new();
    map.insert("fullName".into(), st.get_full_name().to_string().into());
    map.insert("username".into(), st.get_username().to_string().into());
    let locale = st.get_lang_selected().to_string();
    map.insert("locale".into(), locale.clone().into());
    // Región y formatos = mismo locale elegido (no hay paso aparte).
    map.insert("region".into(), locale.into());
    map.insert(
        "keyboardLayout".into(),
        pick(st.get_kb_codes(), st.get_kb_index()).into(),
    );
    map.insert("keyboardVariant".into(), "".into());
    map.insert(
        "timeZone".into(),
        st.get_city_selected().to_string().into(),
    );
    map.insert("timeZoneAuto".into(), st.get_tz_auto().into());
    // GNOME solo distingue claro/oscuro; «dark-tint» del asistente es oscuro.
    let scheme = if tokens.get_theme() == "light" {
        "light"
    } else {
        "dark"
    };
    map.insert("colorScheme".into(), scheme.into());
    map.insert(
        "accentColor".into(),
        theme.get_accent_id().to_string().into(),
    );
    // Color del avatar (variante A: el acento; el asistente no ofrece foto).
    map.insert(
        "avatarColor".into(),
        theme.get_accent_id().to_string().into(),
    );
    map.insert("autoLogin".into(), st.get_auto_login().into());
    map.insert(
        "a11y".into(),
        serde_json::json!({
            "largeText": st.get_a11y_large_text(),
            "highContrast": st.get_a11y_high_contrast(),
            "screenReader": st.get_a11y_screen_reader(),
            "reduceMotion": st.get_a11y_reduce_motion(),
            "screenKeyboard": st.get_a11y_screen_keyboard(),
        }),
    );
    if st.get_git_on() {
        map.insert("gitName".into(), st.get_git_name().to_string().into());
        map.insert("gitEmail".into(), st.get_git_email().to_string().into());
    }
    serde_json::Value::Object(map).to_string()
}

/// Lee `org.gnome.desktop.interface enable-animations`; true si está desactivado.
fn reduced_motion() -> bool {
    let out = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "enable-animations"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim() == "false",
        _ => false,
    }
}

/// Todos los locales UTF-8, cacheados (se parsea la tabla una sola vez).
fn locales() -> &'static [nixdots_lists::locale::Locale] {
    static CACHE: OnceLock<Vec<nixdots_lists::locale::Locale>> = OnceLock::new();
    CACHE.get_or_init(nixdots_lists::locale::all)
}

/// Rellena los modelos de la lista de idiomas filtrando por `query` (nombre o
/// código, sin distinguir mayúsculas).
fn fill_locales(ui: &AppWindow, query: &str) {
    let q = query.to_lowercase();
    let items: Vec<&nixdots_lists::locale::Locale> = locales()
        .iter()
        .filter(|l| {
            q.is_empty() || l.name.to_lowercase().contains(&q) || l.code.to_lowercase().contains(&q)
        })
        .collect();
    let names: Vec<SharedString> = items.iter().map(|l| l.name.clone().into()).collect();
    let codes: Vec<SharedString> = items.iter().map(|l| l.code.clone().into()).collect();
    let st = ui.global::<State>();
    st.set_lang_names(ModelRc::from(names.as_slice()));
    st.set_lang_codes(ModelRc::from(codes.as_slice()));
}

/// Ciudades del paso Zona horaria: (nombre, zona). Se filtran en Rust.
const CITIES: [(&str, &str); 5] = [
    ("Mérida", "America/Merida"),
    ("Ciudad de México", "America/Mexico_City"),
    ("Monterrey", "America/Monterrey"),
    ("Tijuana", "America/Tijuana"),
    ("Madrid", "Europe/Madrid"),
];

/// Rellena los modelos de ciudades filtrando por `query` (nombre o zona).
fn fill_cities(ui: &AppWindow, query: &str) {
    let q = query.to_lowercase();
    let items: Vec<(&str, &str)> = CITIES
        .iter()
        .copied()
        .filter(|(name, zone)| {
            q.is_empty() || name.to_lowercase().contains(&q) || zone.to_lowercase().contains(&q)
        })
        .collect();
    let names: Vec<SharedString> = items.iter().map(|(n, _)| (*n).into()).collect();
    let zones: Vec<SharedString> = items.iter().map(|(_, z)| (*z).into()).collect();
    let st = ui.global::<State>();
    st.set_city_names(ModelRc::from(names.as_slice()));
    st.set_city_tz(ModelRc::from(zones.as_slice()));
}

fn pick(model: ModelRc<SharedString>, index: i32) -> String {
    if index < 0 {
        return String::new();
    }
    model
        .row_data(index as usize)
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Hasta dos iniciales del nombre, en mayúsculas: «Humberto Blanqueto» → «HB».
fn initials(name: &str) -> String {
    let mut out = String::new();
    for word in name.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    out
}

fn first_word(name: &str) -> String {
    name.split_whitespace().next().unwrap_or("").to_string()
}

/// Fuerza de la contraseña (0..4): longitud ≥ 8, mayúsculas y minúsculas,
/// números y símbolos.
fn password_strength(p: &str) -> i32 {
    if p.is_empty() {
        return 0;
    }
    let mut s = 0;
    if p.chars().count() >= 8 {
        s += 1;
    }
    if p.chars().any(|c| c.is_uppercase()) && p.chars().any(|c| c.is_lowercase()) {
        s += 1;
    }
    if p.chars().any(|c| c.is_ascii_digit()) {
        s += 1;
    }
    if p.chars().any(|c| !c.is_alphanumeric()) {
        s += 1;
    }
    s
}

/// Conecta a una red Wi‑Fi con NetworkManager. `true` si conectó.
fn nmcli_connect(ssid: &str, pass: &str) -> bool {
    let mut cmd = std::process::Command::new("nmcli");
    cmd.args(["device", "wifi", "connect", ssid]);
    if !pass.is_empty() {
        cmd.args(["password", pass]);
    }
    matches!(cmd.output(), Ok(o) if o.status.success())
}
