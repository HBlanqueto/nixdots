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
#
# The static binary orchestrates external tools. The live ISO already has most
# of them; the rest are provided with `nix shell` (prebuilt from
# cache.nixos.org, current architecture only) pinned to the nixpkgs revision in
# flake.lock:
#
#   Missing on the ISO, added here:
#     disko, mkpasswd (whois), sbctl
#   Already on the ISO (no need):
#     util-linux  blkid lsblk mount umount swapon swapoff wipefs partprobe
#                 blockdev mkswapfile mountpoint dmesg
#     systemd     systemd-cryptenroll systemd-machine-id-setup
#                 systemd-detect-virt udevadm localectl reboot
#     coreutils   cp chmod chown df sync shred
#     gptfdisk    sgdisk          cryptsetup  cryptsetup
#     btrfs-progs btrfs mkfs.btrfs      dosfstools mkfs.vfat
#     ntfs3g      ntfsresize      kbd         loadkeys
#     git  nix  nixos-install  nixos-generate-config  NetworkManager  nmcli
set -eu

REPO="github:HBlanqueto/nixdots"
# The rolling `latest` release (replaced on every master push that touches the
# installer); tags publish their own release.
RELEASES="https://github.com/HBlanqueto/nixdots/releases/download/latest"
WORK="/tmp/nixdots-installer"

# Every nix invocation below may use flakes.
NIX_CONFIG="experimental-features = nix-command flakes"
export NIX_CONFIG

# 1. Root only; relaunch through sudo otherwise.
if [ "$(id -u)" -ne 0 ]; then
    exec sudo -- "$0" "$@"
fi

# Directory this script lives in, to read the palette file and flake.lock.
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

# 3. nixpkgs pinned to the revision the flake is built against, so the tools
#    match (falls back to the registry if flake.lock is unavailable).
rev=""
if [ -r "$here/flake.lock" ]; then
    rev=$(nix eval --impure --raw --expr \
        "(builtins.fromJSON (builtins.readFile $here/flake.lock)).nodes.nixpkgs.locked.rev" \
        2>/dev/null || true)
fi
if [ -n "$rev" ]; then
    nixpkgs="github:nixos/nixpkgs/$rev"
else
    nixpkgs="nixpkgs"
fi

# Tools the static binary needs that the live ISO lacks (see the header).
# shellcheck disable=SC2086
tool_pkgs="$nixpkgs#disko $nixpkgs#mkpasswd $nixpkgs#sbctl"

# 4. Download and verify the precompiled binary.
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
    echo "Se puede compilar desde el código con Nix: bajará el toolchain de Rust" >&2
    echo "y ~280 crates, puede tardar bastante y ocupar varios GiB en RAM." >&2
    printf '¿Compilarlo ahora con Nix y ejecutarlo? [s/N] ' >&2
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
                    "$nixpkgs#nerd-fonts.jetbrains-mono" 2>/dev/null || true)
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

                # Clear the markers so a leftover from a previous run is never
                # mistaken for this one.
                rm -f "$WORK/started" "$WORK/exit-code"

                # shellcheck disable=SC2086
                nix --extra-experimental-features 'nix-command flakes' \
                    shell "$nixpkgs#kmscon" "$nixpkgs#nerd-fonts.jetbrains-mono" $tool_pkgs -c \
                    kmscon --vt auto --switchvt $hwaccel \
                    --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                    $palette --login -- "$binary" --install "$@" || true

                # If the installer started inside kmscon, kmscon did not fail:
                # reuse the installer's exit code instead of relaunching it.
                if [ -e "$WORK/started" ]; then
                    code=$(cat "$WORK/exit-code" 2>/dev/null || true)
                    case "$code" in
                        '' | *[!0-9]*) code=1 ;;
                    esac
                    exit "$code"
                fi
                echo "kmscon no pudo iniciar; se usa el modo ASCII." >&2
            fi
            ;;
    esac
fi

# 6. Anything else: run the binary with the tools on its PATH.
# shellcheck disable=SC2086
exec nix --extra-experimental-features 'nix-command flakes' \
    shell $tool_pkgs -c "$binary" --install "$@"
