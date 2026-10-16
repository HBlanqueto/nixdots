{ config, lib, pkgs, theme, ... }:

let
    palette = import ../../lib/kmscon-palette.nix { inherit theme; };

    # kmscon fija el DPI en 72, así que font-size son los píxeles del em: la
    # fuente de la consola de macOS es 8x16 y hay que pedirla a 16 para que no
    # la escale.
    xnu = config.nixdots.console.font == "xnu";
in
{
    options.nixdots.console.hwRender = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Aceleración 3D (DRM) en kmscon. Falso en UTM/virtio, donde no aporta.";
    };

    options.nixdots.console.font = lib.mkOption {
        type = lib.types.enum [ "xnu" "sfmono" ];
        default = "xnu";
        description = ''
            Fuente de la TTY. "xnu": la consola de texto de macOS (8x16, solo
            Latin-1). "sfmono": Lat2-Terminus16 en el kernel y SF Mono en kmscon.
        '';
    };

    # kmscon reemplaza los gettys de las VT (alias autovt@.service); el font
    # viene de fonts.nix (fonts.packages). console.font, console.colors y
    # earlySetup siguen en user.nix para el initrd, la consola de emergencia y
    # Plymouth.
    config.services.kmscon = {
        enable = true;
        useXkbConfig = true;
        config = {
            hwaccel = config.nixdots.console.hwRender;
            font-name = if xnu then "XNU Console" else "Liga SFMono Nerd Font";
            font-size = if xnu then 16 else 14;
        } // builtins.listToAttrs (
            map (e: { name = palette.name e.suffix; inherit (e) value; }) palette.entries
        );
    };

    # Los glifos que no están en la XNU (Nerd Font, CJK, color emoji) los
    # resuelve la caída de pango/fontconfig al resto de fonts.packages.
    config.fonts.packages = lib.optional xnu pkgs.xnu-console-font;
}
