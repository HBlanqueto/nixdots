# nixdots-rebuild — safe wrapper around nixos-rebuild.
#
# Usage: nixdots-rebuild [sw|rb|build]
#   sw     build, run the safety checks and activate (default)
#   rb     same, but stage for the next boot instead of switching
#   build  only build; never activate
#
# The canonical repo path is ~/nixdots (override with NIXDOTS_REPO). The
# configuration is always #nixdots.

mode="${1:-sw}"
case "$mode" in
    sw | switch) mode="switch" ;;
    rb | boot) mode="boot" ;;
    build) mode="build" ;;
    *)
        echo "uso: nixdots-rebuild [sw|rb|build]" >&2
        exit 2
        ;;
esac

repo="${NIXDOTS_REPO:-$HOME/nixdots}"

if [ ! -e "$repo/flake.nix" ]; then
    echo "No existe $repo/flake.nix (usa NIXDOTS_REPO para cambiar la ruta canónica)." >&2
    exit 1
fi

cd "$repo"

ref="$repo#nixdots"

# 1. Always build first; only activate once the checks below pass.
echo "== nixos-rebuild build ($ref) =="
nixos-rebuild build --impure --flake "$ref"

if [ "$mode" = "build" ]; then
    exit 0
fi

# 2. Compare the normal users of the running system (uid >= 1000) with the new
#    configuration, and make sure every declared normal user has its hash file.
new_users="$(nix eval --impure --json "$repo#nixosConfigurations.nixdots.config.users.users" --apply '
    users: builtins.mapAttrs (_: u: {
        isNormalUser = u.isNormalUser or false;
        hashedPasswordFile =
            if (u.hashedPasswordFile or null) == null
            then null
            else toString u.hashedPasswordFile;
    }) users')"

disappearing=()
while IFS= read -r user; do
    [ -z "$user" ] && continue
    if ! jq -e --arg user "$user" '.[$user].isNormalUser == true' >/dev/null <<<"$new_users"; then
        disappearing+=("$user")
    fi
done < <(awk -F: '$3 >= 1000 && $3 < 60000 && $1 !~ /^nixbld/ && $1 != "nobody" { print $1 }' /etc/passwd)

if [ "${#disappearing[@]}" -gt 0 ]; then
    echo "Estos usuarios normales desaparecerían de la configuración: ${disappearing[*]}" >&2
    exit 1
fi

while IFS=$'\t' read -r user hashfile; do
    [ -z "$user" ] && continue
    if [ -z "$hashfile" ] || [ "$hashfile" = "null" ]; then
        echo "El usuario '$user' no declara hashedPasswordFile." >&2
        exit 1
    fi
    if [ ! -e "$hashfile" ]; then
        echo "Falta el hash de '$user': $hashfile" >&2
        echo "Créalo con: mkpasswd -m yescrypt | sudo tee $hashfile" >&2
        exit 1
    fi
done < <(jq -r 'to_entries[] | select(.value.isNormalUser) | "\(.key)\t\(.value.hashedPasswordFile)"' <<<"$new_users")

# 3. Activate.
echo "== nixos-rebuild $mode ($ref) =="
sudo nixos-rebuild "$mode" --impure --flake "$ref"
