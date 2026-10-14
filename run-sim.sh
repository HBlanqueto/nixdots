#!/usr/bin/env bash
#
# run-sim.sh — abre el instalador de nixdots en modo SIMULACIÓN.
#
# Uso:
#   ./run-sim.sh          # ventana nueva de WezTerm, con recarga al guardar
#   ./run-sim.sh --here   # en esta misma terminal (sin recarga)
#
# Nunca usa --install: solo simula, no toca discos.
set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO"

# En la misma terminal.
if [ "${1:-}" = "--here" ] || [ "${1:-}" = "-t" ]; then
    exec nix develop .#installer --command \
        cargo run --manifest-path installer/Cargo.toml -- --repo "$REPO"
fi

# Sin Wayland no hay ventana WezTerm: cae a esta terminal.
if [ -z "${WAYLAND_DISPLAY:-}" ] || [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    echo "Sin WAYLAND_DISPLAY/XDG_RUNTIME_DIR; ejecutando en esta terminal." >&2
    exec nix develop .#installer --command \
        cargo run --manifest-path installer/Cargo.toml -- --repo "$REPO"
fi

# No abras otra si ya existe.
if pgrep -f -- "--class nixdots-installer-dev" >/dev/null 2>&1; then
    echo "Ya hay una ventana nixdots-installer-dev abierta (se recarga al guardar)."
    exit 0
fi

# Ventana nueva e independiente, con recarga automática al guardar.
setsid -f wezterm start --always-new-process --class nixdots-installer-dev \
    --cwd "$REPO" -- nix develop .#installer --command \
    watchexec -r -c -w installer/src -w installer/Cargo.toml -e rs,toml -- \
    cargo run --manifest-path installer/Cargo.toml -- --repo "$REPO"

echo "Ventana de simulación lanzada (clase nixdots-installer-dev)."
