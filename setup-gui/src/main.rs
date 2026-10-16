//! `nixdots-setup-gui` — asistente gráfico de primer arranque.
//!
//! No duplica lógica: recoge los datos, escribe un JSON temporal y ejecuta el
//! CLI `nixdots-setup apply` (con `--dry-run` en `--sim`), mostrando su salida.

mod apply;
mod machine;
mod theme;

use adw::prelude::*;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Datos del asistente (los mismos campos que el JSON del CLI).
#[derive(Default, Clone)]
struct Model {
    locale: String,
    keyboard: String,
    full_name: String,
    username: String,
    password: String,
    time_zone: String,
    git_enabled: bool,
    git_name: String,
    git_email: String,
}

fn main() -> glib::ExitCode {
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

    let app = adw::Application::builder()
        .application_id("io.nixdots.Setup")
        .build();
    app.connect_startup(|_| theme::install());
    let state = Rc::new(RefCell::new(Model::default()));
    app.connect_activate(move |app| build(app, state.clone(), Rc::new(repo.clone()), sim));
    // Mis propios argumentos ya están leídos; GApplication no debe ver --sim.
    app.run_with_args::<&str>(&["nixdots-setup-gui"])
}

fn build(app: &adw::Application, state: Rc<RefCell<Model>>, repo: Rc<String>, sim: bool) {
    let window = adw::Window::builder()
        .title("nixdots")
        .default_width(760)
        .default_height(600)
        .build();
    window.set_application(Some(app));

    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::SlideLeftRight);

    // --- Hola ---------------------------------------------------------------
    let hola = page();
    hola.append(
        &gtk::Label::builder()
            .label("¡Hola! · Hello! · Bonjour! · Ciao! · Olá! · Hallo!")
            .wrap(true)
            .css_classes(["title-1"])
            .build(),
    );
    hola.append(
        &gtk::Label::builder()
            .label("Vamos a preparar tu equipo. Son unos pocos pasos.")
            .wrap(true)
            .css_classes(["dim-label"])
            .build(),
    );
    stack.add_named(&hola, Some("hola"));

    // --- Idioma y teclado ---------------------------------------------------
    let locales = nixdots_lists::locale::all();
    let locale_names: Vec<&str> = locales.iter().map(|l| l.name.as_str()).collect();
    let (locale_row, locale_dd) = combo("Idioma", &locale_names, 0);
    let keyboards = nixdots_lists::keyboard::all();
    let kb_names: Vec<&str> = keyboards.iter().map(|k| k.name.as_str()).collect();
    let (kb_row, kb_dd) = combo("Teclado", &kb_names, 0);
    let idioma = page();
    idioma.append(&group("Idioma y teclado", &[locale_row.upcast_ref(), kb_row.upcast_ref()]));
    stack.add_named(&idioma, Some("idioma"));

    let locales = Rc::new(locales);
    {
        let state = state.clone();
        let locales2 = locales.clone();
        locale_dd.connect_selected_notify(move |dd| {
            if let Some(l) = locales2.get(dd.selected() as usize) {
                state.borrow_mut().locale = l.code.clone();
            }
        });
    }
    {
        let state = state.clone();
        kb_dd.connect_selected_notify(move |dd| {
            if let Some(k) = keyboards.get(dd.selected() as usize) {
                state.borrow_mut().keyboard = k.code.clone();
            }
        });
    }
    // Valores iniciales.
    if let Some(l) = locales.first() {
        state.borrow_mut().locale = l.code.clone();
    }
    if let Some(k) = keyboards.first() {
        state.borrow_mut().keyboard = k.code.clone();
    }

    // --- Tu cuenta ----------------------------------------------------------
    let name_row = adw::EntryRow::builder().title("Nombre completo").build();
    let user_row = adw::EntryRow::builder().title("Usuario").build();
    let pass_row = adw::PasswordEntryRow::builder().title("Contraseña").build();
    let pass2_row = adw::PasswordEntryRow::builder().title("Repite la contraseña").build();
    let user_error = gtk::Label::builder()
        .xalign(0.0)
        .visible(false)
        .css_classes(["error"])
        .build();
    let cuenta = page();
    cuenta.append(&group(
        "Tu cuenta",
        &[
            name_row.upcast_ref(),
            user_row.upcast_ref(),
            pass_row.upcast_ref(),
            pass2_row.upcast_ref(),
        ],
    ));
    cuenta.append(&user_error);
    stack.add_named(&cuenta, Some("cuenta"));

    {
        let edited = Rc::new(Cell::new(false));
        let updating = Rc::new(Cell::new(false));

        let state2 = state.clone();
        let user_row2 = user_row.clone();
        let user_error2 = user_error.clone();
        let edited2 = edited.clone();
        let updating2 = updating.clone();
        name_row.connect_changed(move |row| {
            let name = row.text().to_string();
            state2.borrow_mut().full_name = name.clone();
            if !edited2.get() {
                let generated = nixdots_core::user::generate_username(&name);
                updating2.set(true);
                user_row2.set_text(&generated);
                updating2.set(false);
                state2.borrow_mut().username = generated;
                set_user_error(&user_error2, None);
            }
        });

        let state3 = state.clone();
        user_row.connect_changed(move |row| {
            let value = row.text().to_string();
            state3.borrow_mut().username = value.clone();
            if updating.get() {
                return;
            }
            edited.set(true);
            set_user_error(&user_error, nixdots_core::user::username_error(&value));
        });

        let state4 = state.clone();
        pass_row.connect_changed(move |row| {
            state4.borrow_mut().password = row.text().to_string();
        });
        let state5 = state.clone();
        pass2_row.connect_changed(move |_row| {
            // La confirmación no viaja al CLI; solo se comprueba aquí.
            let _ = state5.borrow();
        });
    }

    // --- Zona horaria -------------------------------------------------------
    let zones = Rc::new(nixdots_lists::timezone::zones());
    let zone_names: Vec<&str> = zones.iter().map(String::as_str).collect();
    let (zone_row, zone_dd) = combo("Zona horaria", &zone_names, 0);
    let zona = page();
    zona.append(&group("Zona horaria", &[zone_row.upcast_ref()]));
    stack.add_named(&zona, Some("zona"));
    {
        let state = state.clone();
        let zones = zones.clone();
        zone_dd.connect_selected_notify(move |dd| {
            if let Some(z) = zones.get(dd.selected() as usize) {
                state.borrow_mut().time_zone = z.clone();
            }
        });
    }
    if let Some(z) = zones.first() {
        state.borrow_mut().time_zone = z.clone();
    }

    // --- Este equipo --------------------------------------------------------
    let equipo = page();
    let equip_rows: Vec<adw::ActionRow> = machine::rows()
        .into_iter()
        .map(|(title, value)| {
            adw::ActionRow::builder()
                .title(title)
                .subtitle(value)
                .build()
        })
        .collect();
    let equip_refs: Vec<&gtk::Widget> = equip_rows.iter().map(|r| r.upcast_ref()).collect();
    equipo.append(&group("Este equipo", &equip_refs));
    stack.add_named(&equipo, Some("equipo"));

    // --- Git ----------------------------------------------------------------
    let git_switch = adw::SwitchRow::builder().title("Configurar Git").build();
    let git_name_row = adw::EntryRow::builder().title("Nombre").build();
    let git_email_row = adw::EntryRow::builder().title("Correo").build();
    git_name_row.set_sensitive(false);
    git_email_row.set_sensitive(false);
    let git = page();
    git.append(&group(
        "Git (opcional)",
        &[
            git_switch.upcast_ref(),
            git_name_row.upcast_ref(),
            git_email_row.upcast_ref(),
        ],
    ));
    stack.add_named(&git, Some("git"));
    {
        let state = state.clone();
        let name_row = git_name_row.clone();
        let email_row = git_email_row.clone();
        git_switch.connect_active_notify(move |sw| {
            let on = sw.is_active();
            name_row.set_sensitive(on);
            email_row.set_sensitive(on);
            state.borrow_mut().git_enabled = on;
        });
    }
    {
        let state = state.clone();
        git_name_row.connect_changed(move |row| {
            state.borrow_mut().git_name = row.text().to_string();
        });
    }
    {
        let state = state.clone();
        git_email_row.connect_changed(move |row| {
            state.borrow_mut().git_email = row.text().to_string();
        });
    }

    // --- Aplicando ----------------------------------------------------------
    let spinner = gtk::Spinner::new();
    let status = gtk::Label::builder().xalign(0.0).wrap(true).build();
    let retry = gtk::Button::with_label("Reintentar");
    retry.set_visible(false);
    retry.set_halign(gtk::Align::Start);
    let log_view = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .build();
    let log_scroll = gtk::ScrolledWindow::builder()
        .child(&log_view)
        .vexpand(true)
        .build();
    let aplicando = gtk::Box::new(gtk::Orientation::Vertical, 8);
    aplicando.append(&spinner);
    aplicando.append(&status);
    aplicando.append(&log_scroll);
    aplicando.append(&retry);
    stack.add_named(&aplicando, Some("aplicando"));

    // --- Listo --------------------------------------------------------------
    let empezar = gtk::Button::with_label("Empezar");
    empezar.set_halign(gtk::Align::Center);
    let listo = gtk::Box::new(gtk::Orientation::Vertical, 12);
    listo.set_valign(gtk::Align::Center);
    listo.append(
        &gtk::Label::builder()
            .label("¡Listo!")
            .css_classes(["title-1"])
            .build(),
    );
    listo.append(
        &gtk::Label::builder()
            .label("Tu equipo está configurado. Reiniciaremos para entrar.")
            .wrap(true)
            .css_classes(["dim-label"])
            .build(),
    );
    listo.append(&empezar);
    stack.add_named(&listo, Some("listo"));
    {
        let app = app.clone();
        empezar.connect_clicked(move |_| app.quit());
    }

    // --- Barra de navegación ------------------------------------------------
    let back = gtk::Button::with_label("Atrás");
    let next = gtk::Button::with_label("Continuar");
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    bar.set_halign(gtk::Align::End);
    bar.set_margin_top(6);
    bar.set_margin_bottom(6);
    bar.set_margin_end(12);
    bar.append(&back);
    bar.append(&next);

    let header = adw::HeaderBar::new();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&stack));
    view.add_bottom_bar(&bar);
    window.set_content(Some(&view));

    let pages = ["hola", "idioma", "cuenta", "zona", "equipo", "git"];
    let index = Rc::new(Cell::new(0usize));

    let goto = {
        let stack = stack.clone();
        let bar = bar.clone();
        let back = back.clone();
        let index = index.clone();
        move |i: usize| {
            let i = i.min(pages.len() - 1);
            index.set(i);
            stack.set_visible_child_name(pages[i]);
            bar.set_visible(true);
            back.set_sensitive(i > 0);
        }
    };

    let run_apply: Rc<dyn Fn()> = {
        let state = state.clone();
        let repo = repo.clone();
        let stack = stack.clone();
        let log = log_view.clone();
        let spinner = spinner.clone();
        let retry = retry.clone();
        let status = status.clone();
        Rc::new(move || {
            run_apply(&state, &repo, sim, &stack, &log, &spinner, &retry, &status);
        })
    };

    {
        let goto = goto.clone();
        let index = index.clone();
        let run_apply = run_apply.clone();
        let bar = bar.clone();
        next.connect_clicked(move |_| {
            if index.get() + 1 >= pages.len() {
                bar.set_visible(false);
                run_apply();
            } else {
                goto(index.get() + 1);
            }
        });
    }
    {
        let goto = goto.clone();
        let index = index.clone();
        back.connect_clicked(move |_| {
            let i = index.get();
            if i > 0 {
                goto(i - 1);
            }
        });
    }
    {
        let run_apply = run_apply.clone();
        retry.connect_clicked(move |_| run_apply());
    }

    goto(0);
    window.present();
}

/// Una página con margen y ancho máximo.
fn page() -> gtk::Box {
    let inner = gtk::Box::new(gtk::Orientation::Vertical, 18);
    inner.set_margin_top(24);
    inner.set_margin_bottom(24);
    inner.set_margin_start(24);
    inner.set_margin_end(24);
    let clamp = adw::Clamp::builder()
        .maximum_size(620)
        .child(&inner)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .child(&clamp)
        .vexpand(true)
        .build();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.append(&scroll);
    outer
}

/// Un grupo con las filas dadas.
fn group(title: &str, rows: &[&gtk::Widget]) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(title).build();
    for row in rows {
        group.add(*row);
    }
    group
}

/// Una fila de acción con un desplegable buscable.
fn combo(title: &str, names: &[&str], selected: u32) -> (adw::ActionRow, gtk::DropDown) {
    let dd = gtk::DropDown::from_strings(names);
    dd.set_enable_search(true);
    dd.set_selected(selected);
    let row = adw::ActionRow::builder().title(title).build();
    row.add_suffix(&dd);
    row.set_activatable_widget(Some(&dd));
    (row, dd)
}

fn set_user_error(label: &gtk::Label, error: Option<&'static str>) {
    match error {
        Some(message) => {
            label.set_text(message);
            label.set_visible(true);
        }
        None => label.set_visible(false),
    }
}

fn append_line(view: &gtk::TextView, line: &str) {
    let buffer = view.buffer();
    let mut end = buffer.end_iter();
    buffer.insert(&mut end, line);
    buffer.insert(&mut end, "\n");
    view.scroll_to_iter(&mut end, 0.0, false, 0.0, 0.0);
}

fn split_keyboard(code: &str) -> (String, String) {
    match code.split_once(':') {
        Some((layout, variant)) => (layout.to_string(), variant.to_string()),
        None => (code.to_string(), String::new()),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_apply(
    state: &Rc<RefCell<Model>>,
    repo: &Rc<String>,
    sim: bool,
    stack: &gtk::Stack,
    log: &gtk::TextView,
    spinner: &gtk::Spinner,
    retry: &gtk::Button,
    status: &gtk::Label,
) {
    let m = state.borrow().clone();
    let (keyboard_layout, keyboard_variant) = split_keyboard(&m.keyboard);
    let mut data = serde_json::Map::new();
    data.insert("fullName".into(), m.full_name.clone().into());
    data.insert("username".into(), m.username.clone().into());
    data.insert("locale".into(), m.locale.clone().into());
    data.insert("keyboardLayout".into(), keyboard_layout.into());
    data.insert("keyboardVariant".into(), keyboard_variant.into());
    data.insert("timeZone".into(), m.time_zone.clone().into());
    if m.git_enabled {
        data.insert("gitName".into(), m.git_name.clone().into());
        data.insert("gitEmail".into(), m.git_email.clone().into());
    }
    let json = serde_json::Value::Object(data).to_string();
    let password = m.password.clone();

    log.buffer().set_text("");
    spinner.start();
    retry.set_visible(false);
    status.set_text("Aplicando los cambios…");
    stack.set_visible_child_name("aplicando");

    let log2 = log.clone();
    let stack2 = stack.clone();
    let spinner2 = spinner.clone();
    let retry2 = retry.clone();
    let status2 = status.clone();
    let result = apply::start(
        &json,
        repo,
        sim,
        &password,
        move |line| append_line(&log2, line),
        move |ok| {
            spinner2.stop();
            if ok {
                stack2.set_visible_child_name("listo");
            } else {
                status2.set_text("Algo falló. Revisa el registro y reinténtalo.");
                retry2.set_visible(true);
            }
        },
    );
    if let Err(err) = result {
        spinner.stop();
        status.set_text(&err);
        retry.set_visible(true);
    }
}
