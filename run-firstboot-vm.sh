#!/usr/bin/env bash
#
# run-firstboot-vm.sh — construye y arranca la VM del asistente de primer
# arranque (firstboot-vm).
#
# Con NIXDOTS_ROOT apuntando a un directorio vacío, el flake no ve ningún
# user.json → firstBoot = !(settings ? username) es true → la VM arranca el
# asistente. (Sin esto, el build leería el user.json de la máquina anfitriona.)
# El repo lo siembra la propia VM en /persist/nixdots/repo.
#
# Uso:
#   ./run-firstboot-vm.sh            # x86_64
#   ./run-firstboot-vm.sh aarch64    # aarch64
#
set -euo pipefail

REPO="$(cd "$(dirname "$0")" && pwd)"

arch="${1:-x86_64}"
case "$arch" in
    x86_64) host="firstboot-vm" ;;
    aarch64 | arm64) host="firstboot-vm-aarch64" ;;
    *)
        echo "uso: $0 [x86_64|aarch64]" >&2
        exit 2
        ;;
esac

# Directorio vacío: el flake no encuentra user.json y la VM arranca el asistente.
NIXDOTS_ROOT="$(mktemp -d)"
export NIXDOTS_ROOT

nixos-rebuild build-vm --flake "$REPO#$host"

cat <<EOF

VM construida. Arráncala con:

    ./result/bin/run-$host-vm

Dentro, el asistente corre solo (cage) con datos de prueba en
/etc/nixdots-setup.json: escribe /persist/nixdots/user.json, hace
'nixos-rebuild boot' y reinicia. Tras el reinicio arranca GDM con el usuario
"prueba" (contraseña de prueba: test1234).

Si cambias la config del host, borra ./$host.qcow2 para partir de cero (las
generaciones del rebuild viven ahí).
EOF
