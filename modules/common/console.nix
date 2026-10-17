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
    # siempre la XNU (la consola de texto de macOS, 8x16, solo Latin-1) con el
    # motor psf, que lee el mismo .psf que el kernel: tabla Unicode, escalado
    # entero y «?» en los glifos que faltan. Con el motor psf font-size es la
    # altura en píxeles y se redondea a un múltiplo entero de 16 (32 = doble).
    # console.font, console.colors y earlySetup siguen en user.nix para el
    # initrd, la consola de emergencia y Plymouth.
    config.services.kmscon = {
        enable = true;
        useXkbConfig = true;
        config = {
            hwaccel = config.nixdots.console.hwRender;
            font-engine = "psf";
            font-name = "${pkgs.xnu-console-font}/share/consolefonts/xnu-console.psf";
            font-size = 16;
        } // builtins.listToAttrs (
            map (e: { name = palette.name e.suffix; inherit (e) value; }) palette.entries
        );
    };
}
