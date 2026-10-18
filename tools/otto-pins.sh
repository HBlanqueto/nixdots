#!/usr/bin/env bash
# Actualiza los `pins` de packages/otto.nix (Otto, layers y smithay-otto), todos
# de los forks privados de HBlanqueto.
#
# Los revs de layers y smithay NO son el HEAD de sus forks: se leen del
# Cargo.lock del commit de Otto, que es quien los fija junto con su `ref`. Así
# los pines siempre coinciden con lo que compila Otto.
#
#   tools/otto-pins.sh                 # HEAD de HBlanqueto/otto (rama neptunia)
#   tools/otto-pins.sh <rev>           # ese commit de Otto
#   tools/otto-pins.sh <rev> <rama>    # resuelve <rev> en <rama>
#
# Los hashes salen de `nix flake prefetch` por git+ssh (usa tu llave SSH, sin
# token). Con OTTO_PINS_HTTPS=1 usa github:; como `nix flake prefetch` corre como
# tu usuario, eso exige `access-tokens = github.com=<token>` en
# ~/.config/nix/nix.conf (el token del nix-daemon, /persist/secrets/nix-github.env,
# no sirve aquí). Necesita nix, git y jq.
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
nix_file="$root/packages/otto.nix"

otto_rev=${1:-}
otto_branch=${2:-neptunia}

# nix flake prefetch -> {locked.rev, hash, storePath}. Por defecto git+ssh (mismo
# hash NAR sin token); github: (OTTO_PINS_HTTPS=1) usa el access-token del usuario.
prefetch() {
    local repo=$1 ref=$2 query
    if [ "${OTTO_PINS_HTTPS:-0}" = 1 ]; then
        nix flake prefetch "github:HBlanqueto/$repo/$ref" --json 2>/dev/null \
            || {
                echo "otto-pins: no pude leer HBlanqueto/$repo por HTTPS." >&2
                echo "  nix flake prefetch corre como tu usuario: añade" >&2
                echo "  access-tokens = github.com=<token> a ~/.config/nix/nix.conf" >&2
                echo "  (el token del nix-daemon, /persist/secrets/nix-github.env, no sirve)." >&2
                echo "  O ejecuta sin OTTO_PINS_HTTPS para usar git+ssh." >&2
                exit 1
            }
    else
        if [[ $ref =~ ^[0-9a-f]{40}$ ]]; then query="rev=$ref"; else query="ref=$ref"; fi
        nix flake prefetch "git+ssh://git@github.com/HBlanqueto/$repo?$query" --json
    fi
}

otto_json=$(prefetch otto "${otto_rev:-$otto_branch}")
otto_rev=$(jq -r .locked.rev <<<"$otto_json")
otto_hash=$(jq -r .hash <<<"$otto_json")
otto_lock=$(jq -r .storePath <<<"$otto_json")/Cargo.lock
[ -f "$otto_lock" ] || { echo "otto-pins: no hay Cargo.lock en $otto_lock" >&2; exit 1; }

# repo del fork para cada dependencia (smithay se llama smithay-otto).
repo_of() {
    case "$1" in
        smithay) echo smithay-otto ;;
        *) echo "$1" ;;
    esac
}

declare -A ref rev hash
for dep in layers smithay; do
    fork=$(repo_of "$dep")
    line=$(grep -m1 -oE "github\.com/HBlanqueto/$fork\?[^#\" ]+#[0-9a-f]+" "$otto_lock" || true)
    [ -n "$line" ] || { echo "otto-pins: no encuentro HBlanqueto/$fork en el Cargo.lock de Otto" >&2; exit 1; }
    [[ $line =~ github\.com/HBlanqueto/$fork\?([^#]+)#([0-9a-f]+) ]] \
        || { echo "otto-pins: línea de $dep inesperada: $line" >&2; exit 1; }
    ref[$dep]=${BASH_REMATCH[1]}
    rev[$dep]=${BASH_REMATCH[2]}
    hash[$dep]=$(prefetch "$fork" "${rev[$dep]}" | jq -r .hash)
done

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
        print "            rev = \"" otto_rev "\";"
        print "            hash = \"" otto_hash "\";"
        print "        };"
        print "        layers = {"
        print "            repo = \"layers\";"
        print "            rev = \"" layers_rev "\";"
        print "            ref = \"" layers_ref "\";"
        print "            hash = \"" layers_hash "\";"
        print "        };"
        print "        smithay = {"
        print "            repo = \"smithay-otto\";"
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

mv "$out" "$nix_file"
diff -u "$before" "$nix_file" || true
rm -f "$before"

echo "otto-pins: otto=${otto_rev:0:12} layers=${rev[layers]:0:12} smithay=${rev[smithay]:0:12}"
