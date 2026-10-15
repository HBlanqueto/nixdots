#!/usr/bin/env bash
#
# run-firstboot-vm.sh — construye y arranca la VM del asistente de primer
# arranque (firstboot-vm).
#
# La VM necesita un settings.nix SIN usuario para que arranque el asistente
# (firstBoot = (settings.username or null) == null), así que este script copia
# el repo a un temporal, quita los campos de usuario y construye la VM desde
# ahí. Dentro, el asistente escribe el usuario, reconstruye y reinicia.
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

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cp -a "$REPO/." "$work/"
rm -rf "$work/target" "$work/result"

# Sin usuario: el asistente se abre en el primer arranque.
sed -i -E \
    '/^[[:space:]]*(username|userdescription|hashedPasswordFile|gitName|gitEmail)[[:space:]]*=/d' \
    "$work/settings.nix"

# El flake solo ve lo que git conoce; hay que dejar el settings.nix staged.
git -C "$work" add -A
git -C "$work" -c user.name=firstboot -c user.email=firstboot@localhost \
    commit -qm "firstboot-vm: settings sin usuario" --allow-empty

nixos-rebuild build-vm --flake "$work#$host"

cat <<EOF

VM construida. Arráncala con:

    ./result/bin/run-$host-vm

Dentro, el asistente corre solo (cage) con datos de prueba en
/etc/nixdots-setup.json: escribe /persist/nixdots/settings.nix, hace
'nixos-rebuild boot' y reinicia. Tras el reinicio arranca GDM con el usuario
"prueba" (contraseña de prueba: test1234).

Si cambias la config del host, borra ./$host.qcow2 para partir de cero (las
generaciones del rebuild viven ahí).
EOF
