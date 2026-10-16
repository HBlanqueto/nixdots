{ pkgs, installer }:

let
    theme = import ../share/themes/neptunia.nix;
    nerd = pkgs.nerd-fonts.jetbrains-mono;
    fontsConf = pkgs.makeFontsConf { fontDirectories = [ nerd ]; };
in
pkgs.writeShellApplication {
    name = "nixdots-installer";
    runtimeInputs = [ pkgs.kmscon nerd ];
    text = ''
        installer=${installer}/bin/nixdots-installer
        fonts_conf=${fontsConf}

        # "RRGGBB" -> "R,G,B" for kmscon's custom palette.
        rgb() { printf '%d,%d,%d' "0x''${1:0:2}" "0x''${1:2:2}" "0x''${1:4:2}"; }

        palette=(
            --palette custom
            --palette-black "$(rgb ${theme.c0})"
            --palette-red "$(rgb ${theme.c1})"
            --palette-green "$(rgb ${theme.c2})"
            --palette-yellow "$(rgb ${theme.c3})"
            --palette-blue "$(rgb ${theme.c4})"
            --palette-magenta "$(rgb ${theme.c5})"
            --palette-cyan "$(rgb ${theme.c6})"
            --palette-light-grey "$(rgb ${theme.c7})"
            --palette-dark-grey "$(rgb ${theme.c8})"
            --palette-light-red "$(rgb ${theme.c9})"
            --palette-light-green "$(rgb ${theme.c10})"
            --palette-light-yellow "$(rgb ${theme.c11})"
            --palette-light-blue "$(rgb ${theme.c12})"
            --palette-light-magenta "$(rgb ${theme.c13})"
            --palette-light-cyan "$(rgb ${theme.c14})"
            --palette-white "$(rgb ${theme.c15})"
            --palette-foreground "$(rgb ${theme.fg})"
            --palette-background "0,0,0"
        )

        # On a real VT, outside kmscon, with a render node (what --hwaccel
        # needs), and unless opted out, open kmscon with Nerd Fonts and the
        # neptunia palette, then run the installer inside it.
        if [ "''${TERM:-}" = linux ] \
            && [ -z "''${NIXDOTS_IN_KMSCON:-}" ] \
            && [ -z "''${NIXDOTS_NO_KMSCON:-}" ] \
            && [ -c "$(tty 2>/dev/null || true)" ] \
            && [ -e /dev/dri/renderD128 ]; then
            if FONTCONFIG_FILE="$fonts_conf" NIXDOTS_IN_KMSCON=1 \
                kmscon --vt auto --switchvt --hwaccel \
                    --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                    "''${palette[@]}" --login -- "$installer" "$@"; then
                exit 0
            fi
            echo "kmscon no pudo iniciar (¿sin DRM?); se usa el modo ASCII." >&2
        fi

        exec "$installer" "$@"
    '';
}
