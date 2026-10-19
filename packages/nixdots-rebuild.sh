# nixdots-rebuild — safe wrapper around nixos-rebuild.
#
# Usage: nixdots-rebuild [sw|rb|build]
#   sw     build, run the safety checks and activate (default)
#   rb     same, but stage for the next boot instead of switching
#   build  only build; never activate
#
# The canonical repo path is ~/dotfiles (override with KEEL_REPO). The
# configuration is always #keel.

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

repo="${KEEL_REPO:-${NIXDOTS_REPO:-$HOME/dotfiles}}"
# shortcut: acepta ~/nixdots mientras se mueve el repo a ~/dotfiles.
if [ "$repo" = "$HOME/dotfiles" ] && [ ! -e "$repo/flake.nix" ] && [ -e "$HOME/nixdots/flake.nix" ]; then
    repo="$HOME/nixdots"
fi

if [ ! -e "$repo/flake.nix" ]; then
    echo "No existe $repo/flake.nix (usa KEEL_REPO para cambiar la ruta canónica)." >&2
    exit 1
fi

cd "$repo"

ref="$repo#keel"

# 1. Always build first; only activate once the checks below pass.
echo "== nixos-rebuild build ($ref) =="
nixos-rebuild build --impure --flake "$ref"

if [ "$mode" = "build" ]; then
    exit 0
fi

# 2. Compare the normal users of the running system (uid >= 1000) with the new
#    configuration, so no account silently disappears.
new_users="$(nix eval --impure --json "$repo#nixosConfigurations.keel.config.users.users" --apply '
    users: builtins.mapAttrs (_: u: {
        isNormalUser = u.isNormalUser or false;
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

# 3. Activate.
echo "== nixos-rebuild $mode ($ref) =="
sudo nixos-rebuild "$mode" --impure --flake "$ref"
