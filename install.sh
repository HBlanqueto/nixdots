#!/bin/sh
# Punto de entrada estable; el instalador vive en installer/.
exec "$(dirname -- "$0")/installer/install.sh" "$@"
