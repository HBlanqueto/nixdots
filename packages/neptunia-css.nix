# Renders the neptunia palette as GTK/libadwaita CSS (`@define-color` over the
# libadwaita named colors) plus a full-screen blurred background, generated from
# `share/themes/neptunia.nix` — the same source of truth as `kmscon-palette`.
{ pkgs }:

let
  t = import ../share/themes/neptunia.nix;
  hex = name: "#${t.${name}}";

  # Fondo a pantalla completa, generado en tiempo de build (gradiente de la
  # paleta, desenfocado y teñido con el color base).
  wallpaper =
    pkgs.runCommand "nixdots-wallpaper.png"
      {
        nativeBuildInputs = [ pkgs.imagemagick ];
      }
      ''
        magick -size 1920x1080 gradient:"${hex "lbg"}"-"${hex "dbg"}" \
            -blur 0x30 -fill "${hex "bg"}" -colorize 35% "$out"
      '';

  lines = [
    "@define-color window_bg_color ${hex "bg"};"
    "@define-color window_fg_color ${hex "fg"};"
    "@define-color view_bg_color ${hex "c0"};"
    "@define-color view_fg_color ${hex "fg"};"
    "@define-color headerbar_bg_color ${hex "c0"};"
    "@define-color headerbar_fg_color ${hex "fg"};"
    "@define-color sidebar_bg_color ${hex "c0"};"
    "@define-color sidebar_fg_color ${hex "fg"};"
    "@define-color card_bg_color ${hex "c0"};"
    "@define-color card_fg_color ${hex "fg"};"
    "@define-color popover_bg_color ${hex "c0"};"
    "@define-color popover_fg_color ${hex "fg"};"
    "@define-color accent_color ${hex "c12"};"
    "@define-color accent_bg_color ${hex "c4"};"
    "@define-color accent_fg_color ${hex "bg"};"
    "@define-color destructive_color ${hex "c1"};"
    "@define-color destructive_bg_color ${hex "c1"};"
    "@define-color destructive_fg_color ${hex "bg"};"
    "@define-color success_color ${hex "c2"};"
    "@define-color warning_color ${hex "c3"};"
    "@define-color error_color ${hex "c1"};"
    "@define-color borders ${hex "lbg"};"
    "@define-color dim_label_color ${hex "c8"};"
    ''
      window {
          background-image: url("file://${wallpaper}");
          background-size: cover;
          background-position: center;
      }''
  ];
in
pkgs.writeText "neptunia.css" (builtins.concatStringsSep "\n" lines + "\n")
