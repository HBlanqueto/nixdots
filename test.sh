#!/usr/bin/env bash
#
# test.sh — checks for nixdots-installer.
#
# Runs the Rust lints and tests, builds the installer, then drives its TUI
# through a pty in simulation mode and asserts the expected screens and widgets
# render. Nothing touches a disk and no root is needed.
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
nix develop ".#installer" -c \
    cargo clippy --workspace --all-targets -- -D warnings
ok "clippy"

say "cargo test"
nix develop ".#installer" -c cargo test --workspace
ok "tests"

# ---------------------------------------------------------------------------
if [ "$BUILD" -eq 1 ]; then
    say "nix build .#installer"
    nix build .#installer
fi

INSTALLER="$REPO/result/bin/nixdots-installer"
[ -x "$INSTALLER" ] || fail "no se encontró $INSTALLER (¿falta --no-build?)"

# ---------------------------------------------------------------------------
# pty driver: runs the installer, sends keys, prints the raw terminal output.
driver() {
    python3 - "$1" "$2" <<'PY'
import os, pty, sys, time, fcntl, termios, struct, select

binpath, repo = sys.argv[1], sys.argv[2]
args = [binpath, "--repo", repo]

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

# Inicio (splash) -> Bienvenido
wait_for("Continuar")
send(b"\r")
wait_for("Bienvenido")
send(b"\r")                       # -> Licencia
# Licencia: tab al checkbox, espacio acepta, tab a los botones, enter Aceptar
wait_for("Licencia")
send(b"\t")
send(b" ")
send(b"\t")
send(b"\r")                       # Aceptar -> Red
# Red: elegir una red WPA, escribir la contraseña y conectar
wait_for("Red")
send(b"\r")
wait_for("contraseña")
for c in b"contrasena-larga":
    send(bytes([c]), 0.05)
send(b"\r")                       # Conectando
wait_for("alcanzable", 12)        # Conectado
send(b"\r")                       # Continuar -> Idioma
wait_for("Idioma")
send(b"\r")                       # Idioma -> Zona horaria
wait_for("Zona horaria")
send(b"\r")                       # Zona horaria -> Equipo
wait_for("Equipo")
send(b"\r")                       # Equipo -> Usuario
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
send(b"\r")                       # Resumen -> Instalando (dry run ~20 s)
wait_for("Instalando")
wait_for("Reiniciar", 40)         # Instalando -> Listo (auto)
send(b"\r", 1.0)                  # Listo -> Reiniciar (dry run sale)

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
capture() { # binary
    local bin="$1" out attempt missing
    out="$(mktemp)"
    for attempt in 1 2 3; do
        driver "$bin" "$REPO" >"$out"
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

# ---------------------------------------------------------------------------
say "simulación: recorrido completo"
EXPECT=(
    Bienvenido Licencia Red "Idioma" "Zona horaria" Equipo Usuario
    "Contraseña" Git Disco Resumen Instalando Reiniciar
    "simulación" "no se puede deshacer" "efímero" "alcanzable"
)
if capture "$INSTALLER"; then
    ok "installer: ${#EXPECT[@]} comprobaciones"
else
    fail "installer: no apareció «$MISSING»"
fi

# ---------------------------------------------------------------------------
say "paleta en TERM=linux (OSC P / OSC R)"
LINUX="$(mktemp)"
PTY_TERM=linux driver "$INSTALLER" "$REPO" >"$LINUX"
grep -aqU $'\x1b]P4' "$LINUX" || fail "no se programó el slot OSC P 4 en TERM=linux"
grep -aqU $'\x1b]R'  "$LINUX" || fail "no se restauró la paleta (OSC R)"
rm -f "$LINUX"
ok "TERM=linux: 16 slots + restauración"

printf '\n\033[32mTODO OK\033[0m\n'
