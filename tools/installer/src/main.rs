//! `nixdots-installer` — a text-mode installer for the nixdots NixOS config.
//!
//! With no arguments it runs in dry-run mode: every screen is shown, every
//! command is displayed, but nothing outside a temporary directory is touched.
//! `--install` performs the real installation from the NixOS live ISO.

use color_eyre::eyre::{bail, Result};
use nixdots_installer::app::App;
use nixdots_installer::detect;
use nixdots_installer::settings;
use nixdots_installer::theme;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;
use std::io::Stdout;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Args {
    dry_run: bool,
    repo: String,
    help: bool,
}

fn parse_args() -> Result<Args> {
    let mut dry_run = true;
    let mut repo = "github:HBlanqueto/nixdots".to_string();
    let mut help = false;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--install" => dry_run = false,
            "--repo" => {
                if let Some(value) = it.next() {
                    repo = value;
                }
            }
            "--help" | "-h" => help = true,
            other => bail!("argumento desconocido: {other}\n\n{}", help_text()),
        }
    }
    Ok(Args {
        dry_run,
        repo,
        help,
    })
}

fn help_text() -> String {
    "nixdots-installer [--install] [--repo <ruta|url>]\n\n  (sin argumentos)  simulación\n  --install         instalación real (requiere root, desde la ISO)\n  --repo            repositorio a instalar (defecto github:HBlanqueto/nixdots)".to_string()
}

/// Where the installer records that it started and how it exited, so
/// `install.sh` can tell a kmscon failure from a real run and reuse the code.
const STATE_DIR: &str = "/tmp/nixdots-installer";

fn main() -> Result<()> {
    color_eyre::install()?;
    theme::console::install_panic_hook();

    // Keep kernel messages off the console so they never corrupt the TUI.
    let _ = std::process::Command::new("dmesg")
        .arg("-n")
        .arg("1")
        .status();

    let args = parse_args()?;
    if args.help {
        println!("{}", help_text());
        return Ok(());
    }

    if args.dry_run {
        return run(args);
    }

    // Real install: tell `install.sh` we started (so it does not relaunch the
    // ASCII fallback) and record the exit code once we are done.
    mark_started();
    let result = run(args);
    write_exit_code(result.is_ok());
    result
}

fn run(args: Args) -> Result<()> {
    if !args.dry_run {
        if !detect::is_root() {
            bail!("--install requiere root");
        }
        if !detect::is_live_environment() {
            bail!("--install debe ejecutarse desde el entorno live de NixOS");
        }
    }

    let _console = theme::console::Guard::activate();

    let repo_dir = std::env::temp_dir().join("nixdots");
    let cloned = prepare_repo(&args.repo, &repo_dir).is_ok();
    if !cloned && args.dry_run {
        write_stub_repo(&repo_dir).ok();
    } else if !cloned {
        prepare_repo(&args.repo, &repo_dir)?;
    }

    let mut app = App::new(args.dry_run, args.repo, repo_dir);
    app.source_ok = cloned;
    app.network = detect::network_online();
    app.refresh_network();

    let mut terminal = setup_terminal()?;
    let result = app.run(&mut terminal);
    restore_terminal(&mut terminal)?;
    result
}

/// Marks that the installer really started, so `install.sh` knows kmscon handed
/// control over instead of failing to start.
fn mark_started() {
    let _ = std::fs::create_dir_all(STATE_DIR);
    let _ = std::fs::write(format!("{}/started", STATE_DIR), b"");
}

/// Records the exit code (0 ok, 1 otherwise) for `install.sh` to reuse.
fn write_exit_code(ok: bool) {
    let _ = std::fs::create_dir_all(STATE_DIR);
    let _ = std::fs::write(
        format!("{}/exit-code", STATE_DIR),
        if ok { "0" } else { "1" },
    );
}

fn prepare_repo(spec: &str, dest: &Path) -> Result<()> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).ok();
    }

    // Prefer the repo embedded in the live ISO (`/etc/nixdots-src`), so the
    // install does not depend on the network nor on what is pushed to GitHub.
    let embedded = Path::new("/etc/nixdots-src");
    if embedded.exists() {
        let src = std::fs::canonicalize(embedded).unwrap_or_else(|_| embedded.to_path_buf());
        return copy_dir(&src, dest);
    }

    if let Some(slug) = spec.strip_prefix("github:") {
        git_clone(&format!("https://github.com/{slug}.git"), dest)?;
        checkout_iso_rev(dest);
        Ok(())
    } else if spec.contains("://") || spec.starts_with("git@") {
        git_clone(spec, dest)?;
        checkout_iso_rev(dest);
        Ok(())
    } else {
        let src = PathBuf::from(spec);
        if !src.exists() {
            bail!("no existe el repositorio {spec}");
        }
        copy_dir(&src, dest)
    }
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    let status = Command::new("cp").arg("-a").arg(src).arg(dest).status()?;
    if !status.success() {
        bail!("no se pudo copiar {}", src.display());
    }
    Ok(())
}

/// Checks out the commit recorded in `/etc/nixdots/rev` (written by the live
/// ISO) so the install matches what was built, not the tip of the branch.
/// Best-effort: a shallow clone may lack it, so it is fetched first.
fn checkout_iso_rev(repo: &Path) {
    let Ok(rev) = std::fs::read_to_string("/etc/nixdots/rev") else {
        return;
    };
    let rev = rev.trim();
    if rev.is_empty() {
        return;
    }
    let safe = format!("safe.directory={}", repo.display());
    let _ = Command::new("git")
        .args(["-c", safe.as_str(), "-C"])
        .arg(repo)
        .args(["fetch", "--depth", "1", "origin", rev])
        .status();
    let _ = Command::new("git")
        .args(["-c", safe.as_str(), "-C"])
        .arg(repo)
        .args(["checkout", rev])
        .status();
}

fn git_clone(url: &str, dest: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["clone", "--depth", "1"])
        .arg(url)
        .arg(dest)
        .status()?;
    if !status.success() {
        bail!("git clone de {url} falló");
    }
    Ok(())
}

fn write_stub_repo(dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest.join("lib"))?;
    std::fs::write(
        dest.join("lib/defaults.nix"),
        settings::Settings::fallback().without_user().render(),
    )?;
    std::fs::write(
        dest.join("LICENSE"),
        "GNU GENERAL PUBLIC LICENSE\nVersion 2, June 1991\n",
    )?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
