#!/bin/sh
# nixdots installer bootstrap.
#
# Downloads the precompiled installer for this machine's architecture from the
# latest GitHub release, verifies it against SHA256SUMS and runs it. On a real
# virtual terminal it opens kmscon (Nerd Font + the neptunia palette, read from
# the generated share/themes/kmscon-palette) and runs the installer inside it.
#
#   git clone --depth 1 https://github.com/HBlanqueto/nixdots ~/nixdots
#   sudo ~/nixdots/install.sh
set -eu

REPO="github:HBlanqueto/nixdots"
RELEASES="https://github.com/HBlanqueto/nixdots/releases/latest/download"
WORK="/tmp/nixdots-installer"

# 6. Every nix invocation below may use flakes.
NIX_CONFIG="experimental-features = nix-command flakes"
export NIX_CONFIG

# 1. Root only; relaunch through sudo otherwise.
if [ "$(id -u)" -ne 0 ]; then
    exec sudo -- "$0" "$@"
fi

# Directory this script lives in, to read the generated palette file.
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# 2. Architecture.
machine=$(uname -m)
case "$machine" in
    x86_64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *)
        echo "Arquitectura no soportada: $machine (solo x86_64 y aarch64)." >&2
        exit 1
        ;;
esac

# 3. Download and verify the precompiled binary.
mkdir -p "$WORK"
binary="$WORK/nixdots-installer-$arch"

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        echo "Se necesita curl o wget para descargar el instalador." >&2
        return 1
    fi
}

verified=0
if fetch "$RELEASES/nixdots-installer-$arch" "$binary" &&
    fetch "$RELEASES/SHA256SUMS" "$WORK/SHA256SUMS"; then
    grep " nixdots-installer-$arch\$" "$WORK/SHA256SUMS" >"$WORK/SHA256SUMS.$arch" || true
    if [ -s "$WORK/SHA256SUMS.$arch" ] &&
        (cd "$WORK" && sha256sum -c "SHA256SUMS.$arch"); then
        verified=1
    fi
fi

if [ "$verified" -ne 1 ]; then
    echo "No se pudo descargar o verificar el instalador precompilado." >&2
    printf '¿Compilarlo con Nix y ejecutarlo? [s/N] ' >&2
    read -r reply || reply=n
    case "$reply" in
        s | S | y | Y) exec nix run "$REPO#installer" -- --install "$@" ;;
        *)
            echo "Alternativa: sudo nix run $REPO#installer -- --install" >&2
            exit 1
            ;;
    esac
fi
chmod +x "$binary"

# 5. Real VT: open kmscon with the Nerd Font and the neptunia palette.
if [ "${TERM:-}" = linux ] && [ -z "${NIXDOTS_NO_KMSCON:-}" ]; then
    case "$(tty 2>/dev/null || true)" in
        /dev/tty*)
            palette_file="$here/share/themes/kmscon-palette"
            if [ -r "$palette_file" ]; then
                palette=$(cat "$palette_file")

                # Point fontconfig at the Nerd Font so kmscon resolves it.
                fontdir=$(nix build --no-link --print-out-paths \
                    nixpkgs#nerd-fonts.jetbrains-mono 2>/dev/null || true)
                if [ -n "$fontdir" ]; then
                    FONTCONFIG_FILE="$WORK/fonts.conf"
                    export FONTCONFIG_FILE
                    cat >"$FONTCONFIG_FILE" <<EOF
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <dir>$fontdir/share/fonts</dir>
  <cachedir>$WORK/fontcache</cachedir>
</fontconfig>
EOF
                fi

                hwaccel=
                if [ -e /dev/dri/renderD128 ]; then hwaccel=--hwaccel; fi

                # shellcheck disable=SC2086
                if nix --extra-experimental-features 'nix-command flakes' \
                    shell nixpkgs#kmscon nixpkgs#nerd-fonts.jetbrains-mono -c \
                    kmscon --vt auto --switchvt $hwaccel \
                    --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                    $palette --login -- "$binary" --install "$@"; then
                    exit 0
                fi
                echo "kmscon no pudo iniciar; se usa el modo ASCII." >&2
            fi
            ;;
    esac
fi

# 6. Anything else: run the binary directly.
exec "$binary" --install "$@"
