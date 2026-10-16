{ data, lib, ... }:

let
    a = data.settings.a11y or { };
    on = k: a.${k} or false;
    timeZoneAuto = data.settings.timeZoneAuto or false;
in

{
    # Desktop-user bits shared by both hosts: the WezTerm configuration.
    programs.wezterm = {
        enable = true;
        extraConfig = builtins.readFile ./wezterm.lua;
    };

    # Ajustes de GNOME del usuario. Sólo se escriben las claves activadas.
    dconf.settings = {
        "org/gnome/desktop/interface" =
            lib.optionalAttrs (on "largeText") { text-scaling-factor = 1.25; }
            // lib.optionalAttrs (on "reduceMotion") { enable-animations = false; };
        "org/gnome/desktop/a11y/interface" =
            lib.optionalAttrs (on "highContrast") { high-contrast = true; };
        "org/gnome/desktop/a11y/applications" =
            lib.optionalAttrs (on "screenReader") { screen-reader-enabled = true; }
            // lib.optionalAttrs (on "screenKeyboard") { screen-keyboard-enabled = true; };
        "org/gnome/desktop/datetime" = {
            automatic-timezone = timeZoneAuto;
        };
    };
}
