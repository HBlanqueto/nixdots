{ config, lib, ... }:

let
    palette = import ../../lib/kmscon-palette.nix;
in
{
    options.nixdots.console.hwRender = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Aceleración 3D (DRM) en kmscon. Falso en UTM/virtio, donde no aporta.";
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
            font-name = "Liga SFMono Nerd Font";
            font-size = 14;
        } // builtins.listToAttrs (
            map (e: { name = palette.name e.suffix; inherit (e) value; }) palette.entries
        );
    };
}
