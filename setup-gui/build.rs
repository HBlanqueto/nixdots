fn main() {
    // EmbedFiles mete los .ttf y el .jpg en el binario (no dependen del sistema
    // de archivos en tiempo de ejecución); el renderizador es femtovg/software.
    // Traducciones gettext (.po) embebidas; el idioma se elige en tiempo de
    // ejecución con `slint::select_bundled_translation`. Sin contexto por
    // defecto para que la clave sea el propio texto en español.
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles)
        .with_bundled_translations("i18n")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).expect("compilar ui/app.slint");
}
