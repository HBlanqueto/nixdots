//! `nixdots-disk` — a small TUI disk utility (macOS Disk Utility style).
//!
//! Runs in dry-run with sample data by default; `--real` executes the
//! destructive commands.

use color_eyre::eyre::Result;
use nixdots_installer::diskui;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut real = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--real" => real = true,
            "--help" | "-h" => {
                println!(
                    "nixdots-disk [--real]\n\n  (sin argumentos)  simulación con datos de ejemplo\n  --real            opera sobre los discos reales"
                );
                return Ok(());
            }
            other => {
                eprintln!("argumento desconocido: {other}");
                std::process::exit(2);
            }
        }
    }

    diskui::run(!real)
}
