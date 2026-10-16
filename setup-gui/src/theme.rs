//! Tema: fuerza oscuro y aplica encima el CSS de neptunia (embebido por el
//! paquete, generado desde `share/themes/neptunia.nix`).

/// CSS generado por `packages/neptunia-css.nix` (`NIXDOTS_GTK_CSS`).
const CSS: &str = include_str!(env!("NIXDOTS_GTK_CSS"));

/// Fuerza el esquema oscuro y carga el CSS de neptunia.
pub fn install() {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
