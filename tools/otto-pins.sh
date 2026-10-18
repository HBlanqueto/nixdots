#!/usr/bin/env bash
# Actualiza los `pins` de packages/otto.nix (Otto, layers y el fork de smithay).
#
# Los revs de layers y smithay NO son el HEAD de sus forks: se leen del
# Cargo.lock del commit de Otto, que es quien los fija junto con su `ref`. Así
# los pines siempre coinciden con lo que compila Otto.
#
#   tools/otto-pins.sh                 # HEAD de HBlanqueto/otto (rama main)
#   tools/otto-pins.sh <rev>           # ese commit de Otto
#   tools/otto-pins.sh <rev> <rama>    # resuelve <rev> en <rama>
#
# Cada hash sale de `nix flake prefetch`; si la copia privada no es accesible,
# cae a nongio (mismo hash NAR). Necesita nix, git y jq.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
nix_file="$root/packages/otto.nix"

otto_rev=${1:-}
otto_branch=${2:-main}

# repo de HBlanqueto para cada dependencia (smithay se llama smithay-otto).
mirror_of() {
    case "$1" in
        smithay) echo smithay-otto ;;
        *) echo "$1" ;;
    esac
}

# nix flake prefetch github:... -> {locked.rev, hash, storePath}. Si la copia
# privada falla por falta de acceso, nongio da el mismo contenido y hash.
prefetch() {
    local mirror_repo=$1 nongio_repo=$2 ref=$3
    nix flake prefetch "github:HBlanqueto/$mirror_repo/$ref" --json 2>/dev/null \
        || nix flake prefetch "github:nongio/$nongio_repo/$ref" --json
}

otto_json=$(prefetch otto otto "${otto_rev:-$otto_branch}")
otto_rev=$(jq -r .locked.rev <<<"$otto_json")
otto_hash=$(jq -r .hash <<<"$otto_json")
otto_lock=$(jq -r .storePath <<<"$otto_json")/Cargo.lock
[ -f "$otto_lock" ] || { echo "otto-pins: no hay Cargo.lock en $otto_lock" >&2; exit 1; }

declare -A owner ref rev hash
for dep in layers smithay; do
    line=$(grep -m1 -oE "github\.com/[A-Za-z0-9_.-]+/$dep\?[^#\" ]+#[0-9a-f]+" "$otto_lock" || true)
    [ -n "$line" ] || { echo "otto-pins: no encuentro $dep en el Cargo.lock de Otto" >&2; exit 1; }
    [[ $line =~ github\.com/([^/]+)/$dep\?([^#]+)#([0-9a-f]+) ]] \
        || { echo "otto-pins: línea de $dep inesperada: $line" >&2; exit 1; }
    owner[$dep]=${BASH_REMATCH[1]}
    ref[$dep]=${BASH_REMATCH[2]}
    rev[$dep]=${BASH_REMATCH[3]}
    hash[$dep]=$(prefetch "$(mirror_of "$dep")" "$dep" "${rev[$dep]}" | jq -r .hash)
done

if [ "${owner[layers]}" != "${owner[smithay]}" ]; then
    echo "otto-pins: layers y smithay apuntan a owners distintos; revisa el Cargo.lock" >&2
    exit 1
fi

before=$(mktemp)
out=$(mktemp)
cp "$nix_file" "$before"

awk \
    -v otto_rev="$otto_rev" -v otto_hash="$otto_hash" \
    -v layers_rev="${rev[layers]}" -v layers_ref="${ref[layers]}" -v layers_hash="${hash[layers]}" \
    -v smithay_rev="${rev[smithay]}" -v smithay_ref="${ref[smithay]}" -v smithay_hash="${hash[smithay]}" '
    /^    pins = \{$/ {
        print "    pins = {"
        print "        otto = {"
        print "            repo = \"otto\";"
        print "            mirror = \"otto\";"
        print "            rev = \"" otto_rev "\";"
        print "            hash = \"" otto_hash "\";"
        print "        };"
        print "        layers = {"
        print "            repo = \"layers\";"
        print "            mirror = \"layers\";"
        print "            rev = \"" layers_rev "\";"
        print "            ref = \"" layers_ref "\";"
        print "            hash = \"" layers_hash "\";"
        print "        };"
        print "        smithay = {"
        print "            repo = \"smithay\";"
        print "            mirror = \"smithay-otto\";"
        print "            rev = \"" smithay_rev "\";"
        print "            ref = \"" smithay_ref "\";"
        print "            hash = \"" smithay_hash "\";"
        print "        };"
        print "    };"
        skip = 1
        next
    }
    skip && /^    \};$/ { skip = 0; next }
    !skip { print }
' "$nix_file" > "$out"

# gitSource construye la línea `source` del Cargo.lock con el owner de nongio;
# si el lock apunta a otro owner, hay que seguirlo para que el replace case.
if [ "${owner[layers]}" != nongio ]; then
    sed -i "s#git+https://github\.com/nongio/#git+https://github.com/${owner[layers]}/#" "$out"
fi

mv "$out" "$nix_file"
diff -u "$before" "$nix_file" || true
rm -f "$before"

echo "otto-pins: otto=${otto_rev:0:12} layers=${rev[layers]:0:12} smithay=${rev[smithay]:0:12}"
