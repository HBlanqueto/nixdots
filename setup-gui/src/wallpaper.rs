//! Fondo desenfocado una sola vez al arrancar (nunca por fotograma).
//!
//! Reduce la imagen a 1/4, le aplica un desenfoque gaussiano (~40 px) y la
//! escala de nuevo. El PNG resultante se guarda en caché en
//! `/run/psamathe/wallpaper-blur.png` y se muestra como `Image`.
//!
//! El repo solo trae el fondo ya desenfocado del mockup
//! (`ui/firstboot-blur.jpg`); `NIXDOTS_WALLPAPER` permite apuntar a la imagen
//! real sin desenfocar si está disponible.

use std::path::Path;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

const SOURCE: &[u8] = include_bytes!("../ui/firstboot-blur.jpg");
const CACHE: &str = "/run/psamathe/wallpaper-blur.png";

/// Devuelve el fondo listo para mostrar: la caché si existe, o la imagen
/// desenfocada (que además se guarda en caché) si no.
pub fn load() -> Option<Image> {
    if let Ok(bytes) = std::fs::read(CACHE) {
        if let Ok(img) = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png) {
            if let Some(image) = to_slint(img.into_rgba8()) {
                return Some(image);
            }
        }
    }

    let source = source_bytes();
    let decoded = image::load_from_memory(&source).ok()?;
    let (w, h) = (decoded.width(), decoded.height());
    if w == 0 || h == 0 {
        return None;
    }
    let small = decoded.resize_exact(
        (w / 4).max(1),
        (h / 4).max(1),
        image::imageops::FilterType::Triangle,
    );
    let full = small
        .blur(40.0)
        .resize_exact(w, h, image::imageops::FilterType::Triangle)
        .to_rgba8();

    if let Some(parent) = Path::new(CACHE).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = full.save(CACHE);

    to_slint(full)
}

/// Los bytes del fondo: `NIXDOTS_WALLPAPER` si se indicó, o el jpg embebido.
fn source_bytes() -> Vec<u8> {
    match std::env::var_os("NIXDOTS_WALLPAPER") {
        Some(path) => std::fs::read(path).unwrap_or_else(|_| SOURCE.to_vec()),
        None => SOURCE.to_vec(),
    }
}

fn to_slint(img: image::RgbaImage) -> Option<Image> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(img.as_raw(), w, h);
    Some(Image::from_rgba8(buffer))
}
