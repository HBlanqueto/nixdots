{ pkgs, theme, ... }:

let
    # Mismas claves que la tabla Lua esperaba: 4 nombres fijos + c0..c15.
    themeKeys = [ "dbg" "lbg" "fg" "bg" ] ++ map (n: "c${toString n}") (pkgs.lib.range 0 15);
    themeLua =
        "return {\n"
        + pkgs.lib.concatMapStrings (k: "    ${k} = \"#${theme.colors.${k}}\",\n") themeKeys
        + "}\n";
in

{
    home = {
        pointerCursor = {
            enable = true;
            name = "Adwaita";
            package = pkgs.adwaita-icon-theme;
            size = 24;
            gtk.enable = false;
            x11.enable = true;
        };

        sessionVariables = {
            QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
            QT_QPA_PLATFORMTHEME = "xdgdesktopportal";
        };
    };

    gtk = {
        enable = false;
    };

    xdg = {
        configFile = {
            "lua-theme/theme.lua".text = themeLua;
        };
    };

    fonts = {
        fontconfig = {
            enable = true;
        };
    };
}