{
  pkgs,
  installer,
  theme,
}:

let
  palette = import ../share/neptunia/palette.nix { inherit theme; };
  nerd = pkgs.nerd-fonts.jetbrains-mono;
  fontsConf = pkgs.makeFontsConf { fontDirectories = [ nerd ]; };

  # "R,G,B" values straight from the shared palette, as kmscon CLI args.
  paletteArgs = pkgs.lib.concatMap (e: [
    "--${palette.name e.suffix}"
    e.value
  ]) palette.entries;
in
pkgs.writeShellApplication {
  name = "nixdots-installer";
  runtimeInputs = [
    pkgs.kmscon
    nerd
  ];
  text = ''
    installer=${installer}/bin/nixdots-installer
    fonts_conf=${fontsConf}

    # On a real VT, outside kmscon, with a render node (what --hwaccel
    # needs), and unless opted out, open kmscon with Nerd Fonts and the
    # neptunia palette, then run the installer inside it.
    if [ "''${TERM:-}" = linux ] \
        && [ -z "''${KEEL_IN_KMSCON:-}" ] \
        && [ -z "''${KEEL_NO_KMSCON:-}" ] \
        && [ -c "$(tty 2>/dev/null || true)" ] \
        && [ -e /dev/dri/renderD128 ]; then
        if FONTCONFIG_FILE="$fonts_conf" KEEL_IN_KMSCON=1 \
            kmscon --vt auto --switchvt --hwaccel \
                --font-name "JetBrainsMono Nerd Font Mono" --font-size 14 \
                ${pkgs.lib.escapeShellArgs paletteArgs} --login -- "$installer" "$@"; then
            exit 0
        fi
        echo "kmscon no pudo iniciar (¿sin DRM?); se usa el modo ASCII." >&2
    fi

    exec "$installer" "$@"
  '';
}
