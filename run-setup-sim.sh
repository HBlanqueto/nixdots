#!/usr/bin/env bash
#
# run-setup-sim.sh — abre el asistente de primer arranque (Slint) en una ventana
# de prueba dentro de la sesión, sin tocar el sistema.
#
# Uso:
#   ./run-setup-sim.sh          # ventana nueva de WezTerm, recompila al guardar
#   ./run-setup-sim.sh --here   # en esta misma terminal (sin recarga)
#
# La app corre con --sim y el CLI se invoca con --dry-run: no aplica nada.
set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO"

RUN=(nix develop .#setup-gui --command \
    cargo run --manifest-path setup-gui/Cargo.toml -- --sim)

# En la misma terminal.
if [ "${1:-}" = "--here" ] || [ "${1:-}" = "-t" ]; then
    exec "${RUN[@]}"
fi

# Sin Wayland no hay ventana WezTerm: cae a esta terminal.
if [ -z "${WAYLAND_DISPLAY:-}" ] || [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "Sin WAYLAND_DISPLAY/XDG_RUNTIME_DIR; ejecutando en esta terminal." >&2
    exec "${RUN[@]}"
fi

# No abras otra si ya existe.
if pgrep -f -- "--class nixdots-setup-sim" >/dev/null 2>&1; then
    echo "Ya hay una ventana nixdots-setup-sim abierta (se recarga al guardar)."
    exit 0
fi

# Ventana nueva e independiente, con recarga automática al guardar.
setsid -f wezterm start --always-new-process --class nixdots-setup-sim \
    --cwd "$REPO" -- nix develop .#setup-gui --command \
    watchexec -r -c -w setup-gui/ui -w setup-gui/src \
        -w setup-gui/Cargo.toml -w setup-gui/build.rs -w Cargo.toml \
        -e slint,rs,toml -- \
    cargo run --manifest-path setup-gui/Cargo.toml -- --sim

echo "Ventana de simulación lanzada (clase nixdots-setup-sim)."
