//! `nixdots-setup-gui` — asistente gráfico de primer arranque (Slint).
//!
//! No duplica lógica: recoge los datos del modelo de la UI, escribe un JSON
//! temporal y ejecuta el CLI `nixdots-setup apply` (con `--dry-run` en `--sim`),
//! mostrando su salida y deduciendo la fase.

slint::include_modules!();

use std::cell::Cell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString};

mod apply;
mod machine;

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

    // Datos reales del equipo (solo lectura).
    let info = machine::info();
    {
        let st = ui.global::<State>();
        st.set_machine_model(info.model.into());
        st.set_machine_cpu(info.cpu.into());
        st.set_machine_ram(info.ram.into());
        st.set_machine_host(info.host.into());
    }

    // Preferencia del sistema para la reducción de movimiento (GNOME).
    ui.global::<Theme>().set_reduced_motion(reduced_motion());

    // Saludo del paso 0: máquina de escribir (letra a letra) y luego fundido.
    spawn_greeting(&ui);

    // Esc cierra la ventana de prueba.
    ui.on_quit_app({
        let weak = ui.as_weak();
        move || {
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
                st.set_step(10);
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
                            st.set_error_step(9);
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

/// Máquina de escribir del saludo: escribe cada palabra letra a letra y luego
/// la funde antes de pasar a la siguiente.
fn spawn_greeting(ui: &AppWindow) {
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
        let n = words.len();
        let mut i = 0usize;
        loop {
            let chars: Vec<char> = words[i].chars().collect();
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
                if weak.upgrade().is_none() {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(90));
            }
            std::thread::sleep(std::time::Duration::from_millis(1500));
            set_state(&weak, |st| st.set_g_shown(false));
            if weak.upgrade().is_none() {
                return;
            }
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
    map.insert(
        "locale".into(),
        pick(st.get_lang_codes(), st.get_lang_index()).into(),
    );
    map.insert(
        "region".into(),
        pick(st.get_region_codes(), st.get_region_index()).into(),
    );
    map.insert(
        "keyboardLayout".into(),
        pick(st.get_kb_codes(), st.get_kb_index()).into(),
    );
    map.insert("keyboardVariant".into(), "".into());
    map.insert(
        "timeZone".into(),
        pick(st.get_city_tz(), st.get_city_index()).into(),
    );
    map.insert("timeZoneAuto".into(), st.get_tz_auto().into());
    map.insert("theme".into(), tokens.get_theme().to_string().into());
    map.insert(
        "accentColor".into(),
        theme.get_accent_id().to_string().into(),
    );
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
