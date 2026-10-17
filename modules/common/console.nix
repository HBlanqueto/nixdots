{ config, lib, pkgs, theme, ... }:

let
    palette = import ../../lib/palette.nix { inherit theme; };
in
{
    options.nixdots.console.hwRender = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Aceleración 3D (DRM) en kmscon. Falso en UTM/virtio, donde no aporta.";
    };

    # kmscon reemplaza los gettys de las VT (alias autovt@.service). La TTY usa
    # siempre la XNU (la consola de texto de macOS, 8x16, solo Latin-1); kmscon
    # fija el DPI en 72, así que font-size son píxeles del em y hay que pedir 16
    # para que no la escale. console.font, console.colors y earlySetup siguen en
    # user.nix para el initrd, la consola de emergencia y Plymouth.
    config.services.kmscon = {
        enable = true;
        useXkbConfig = true;
        config = {
            hwaccel = config.nixdots.console.hwRender;
            font-name = "XNU Console";
            font-size = 16;
        } // builtins.listToAttrs (
            map (e: { name = palette.name e.suffix; inherit (e) value; }) palette.entries
        );
    };

    config.fonts.packages = [ pkgs.xnu-console-font ];
}
