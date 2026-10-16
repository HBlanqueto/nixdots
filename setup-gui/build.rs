fn main() {
    // EmbedFiles mete los .ttf y el .jpg en el binario (no dependen del sistema
    // de archivos en tiempo de ejecución); el renderizador es femtovg/software.
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles);
    slint_build::compile_with_config("ui/app.slint", config).expect("compilar ui/app.slint");
}
