# Renders the neptunia palette as GTK/libadwaita CSS (`@define-color` over the
# libadwaita named colors), generated from `share/themes/neptunia.nix` — the
# same source of truth as `kmscon-palette`.
{ pkgs }:

let
    t = import ../share/themes/neptunia.nix;
    hex = name: "#${t.${name}}";
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
    ];
in
pkgs.writeText "neptunia.css" (builtins.concatStringsSep "\n" lines + "\n")
