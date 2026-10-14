#!/usr/bin/env bash
#
# test.sh — checks for nixdots-installer / nixdots-disk.
#
# Runs the Rust lints and tests, builds the flake outputs, then drives both
# TUIs through a pty in simulation mode and asserts the expected screens and
# widgets render. Nothing touches a disk and no root is needed.
#
# Usage:
#   ./test.sh            # full run
#   ./test.sh --no-build # skip `nix build` (reuse ./result if present)
#
set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO"

BUILD=1
for arg in "$@"; do
    case "$arg" in
        --no-build) BUILD=0 ;;
        -h|--help) echo "uso: $0 [--no-build]"; exit 0 ;;
        *) echo "argumento desconocido: $arg" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "FALLO: se necesita python3 para conducir la TUI por un pty" >&2
    exit 1
fi

say()  { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFALLO: %s\033[0m\n' "$*" >&2; exit 1; }
ok()   { printf '\033[32mok\033[0m  %s\n' "$*"; }

# ---------------------------------------------------------------------------
say "cargo clippy -D warnings"
nix-shell -p cargo rustc gcc clippy --run \
    'cargo clippy --manifest-path installer/Cargo.toml --all-targets -- -D warnings'
ok "clippy"

say "cargo test"
nix-shell -p cargo rustc gcc --run 'cargo test --manifest-path installer/Cargo.toml'
ok "tests"

# ---------------------------------------------------------------------------
if [ "$BUILD" -eq 1 ]; then
    say "nix build .#installer .#disk"
    nix build .#installer .#disk
fi

INSTALLER="$REPO/result/bin/nixdots-installer"
DISK="$REPO/result/bin/nixdots-disk"
[ -x "$INSTALLER" ] || fail "no se encontró $INSTALLER (¿falta --no-build?)"
[ -x "$DISK" ] || fail "no se encontró $DISK"

# ---------------------------------------------------------------------------
# pty driver: runs a TUI, sends keys, prints the raw terminal output to stdout.
driver() {
    python3 - "$@" <<'PY'
import os, pty, sys, time, fcntl, termios, struct, select

mode, binpath, repo = sys.argv[1], sys.argv[2], sys.argv[3]
args = [binpath] + (["--repo", repo] if mode == "installer" else [])

pid, fd = pty.fork()
if pid == 0:
    os.environ["TERM"] = os.environ.get("PTY_TERM", "xterm-256color")
    os.execv(binpath, args)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 110, 0, 0))

buf = bytearray()
def pump(sec):
    end = time.time() + sec
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.1)
        if r:
            try:
                d = os.read(fd, 65536)
            except OSError:
                return
            if not d:
                return
            buf.extend(d)

def kick():
    # Force a full repaint so the current screen is emitted contiguously
    # (crossterm otherwise only writes changed cells).
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 41, 110, 0, 0)); pump(0.15)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 110, 0, 0)); pump(0.15)

def send(s, wait=0.5):
    os.write(fd, s)
    pump(wait)
    kick()

def wait_for(marker, timeout=15):
    end = time.time() + timeout
    while time.time() < end:
        pump(0.2)
        if marker.encode() in buf:
            return True
    return False

if mode == "installer":
    wait_for("Bienvenido")
    send(b"\r")                       # -> Red (Lista)
    wait_for("Red")
    send(b"\r")                       # elegir red WPA -> Contrasena
    wait_for("contraseña")
    for c in b"contrasena-larga":
        send(bytes([c]), 0.05)
    send(b"\r")                       # Contrasena -> Conectando
    wait_for("alcanzable", 12)        # -> Conectado
    send(b"\r")                       # Conectado -> Continuar -> Licencia
    wait_for("Licencia")
    send(b" ")                        # acepto
    send(b"\r")                       # Licencia -> Equipo
    wait_for("Equipo")
    send(b"\r")                       # Equipo -> Zona horaria
    wait_for("Zona horaria")
    send(b"\r")                       # Zona horaria -> Idioma
    wait_for("Idioma")
    send(b"\r")                       # Idioma -> Usuario
    wait_for("Usuario")
    send(b"\r")                       # Usuario -> Contraseña
    wait_for("Contraseña")
    send(b"test1234")
    send(b"\x1b[B")
    send(b"test1234")
    send(b"\r")                       # Contraseña -> Git
    wait_for("Git")
    send(b"\r")                       # Git (omitir) -> Disco
    wait_for("Disco")
    send(b" ")                        # confirmar borrado
    send(b"\r")                       # Disco -> Resumen
    wait_for("Resumen")
    send(b"\r")                       # Resumen -> Instalando
    pump(6.0)                         # instalando (dry run)
    send(b"\r", 1.0)                  # Listo -> reiniciar (dry run sale)
else:
    pump(1.0)
    send(b"v", 0.4)       # comandos
    send(b"\t")           # foco barra lateral
    send(b"\x1b[B")
    send(b"\x1b[B")
    send(b"\t")           # foco barra de herramientas
    send(b"\x1b[D")       # -> Borrar
    send(b"\r")           # abrir hoja Borrar
    send(b"\x1b")         # cerrar
    send(b"q")

pump(0.5)
try:
    os.close(fd)
except OSError:
    pass
sys.stdout.buffer.write(bytes(buf))
PY
}

# Capture with retries: crossterm only emits changed cells, so a title can
# occasionally land between frames. Prints the raw capture on success.
MISSING=""
capture() { # mode binary
    local mode="$1" bin="$2" out attempt missing
    out="$(mktemp)"
    for attempt in 1 2 3; do
        driver "$mode" "$bin" "$REPO" >"$out"
        missing=""
        for want in "${EXPECT[@]}"; do
            grep -aq -- "$want" "$out" || { missing="$want"; break; }
        done
        if [ -z "$missing" ]; then
            rm -f "$out"
            return 0
        fi
    done
    MISSING="$missing"
    rm -f "$out"
    return 1
}

check() { # mode, then expected strings
    local mode="$1"; shift
    EXPECT=("$@")
    local bin="$INSTALLER"
    [ "$mode" = "disk" ] && bin="$DISK"
    say "simulación: $mode"
    if capture "$mode" "$bin"; then
        ok "$mode: ${#EXPECT[@]} comprobaciones"
    else
        fail "$mode: no apareció «$MISSING»"
    fi
}

check installer \
    Bienvenido Red Licencia Equipo "Zona horaria" Idioma Usuario \
    "Contraseña" Git Disco Resumen Instalando Listo \
    "simulación" "no se puede deshacer" "efímero" "alcanzable"

check disk \
    Volumen Auxilios Particionar Borrar Restaurar Desmontar Info \
    Interno Externo nvme0n1 "@rootfs" Montaje

# ---------------------------------------------------------------------------
say "paleta en TERM=linux (OSC P / OSC R)"
LINUX="$(mktemp)"
PTY_TERM=linux driver installer "$INSTALLER" "$REPO" >"$LINUX"
grep -aqU $'\x1b]P4' "$LINUX" || fail "no se programó el slot OSC P 4 en TERM=linux"
grep -aqU $'\x1b]R'  "$LINUX" || fail "no se restauró la paleta (OSC R)"
rm -f "$LINUX"
ok "TERM=linux: 16 slots + restauración"

printf '\n\033[32mTODO OK\033[0m\n'
