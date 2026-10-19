{ config, lib, pkgs, theme, ... }:

let
    palette = import ../share/neptunia/palette.nix { inherit theme; };

    # La XNU (la consola de texto de macOS, 8x16, solo Latin-1) es la única
    # fuente de la TTY: el mismo .psf para el kernel (initrd, consola de
    # emergencia y Plymouth) y para kmscon con el motor psf.
    xnuFont = "${pkgs.xnu-console-font}/share/consolefonts/xnu-console.psf";
in
{
    options.nixdots.console.hwRender = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Aceleración 3D (DRM) en kmscon. Falso en UTM/virtio, donde no aporta.";
    };

    config.console = {
        earlySetup = true;
        # Los 16 colores ANSI (con el fondo negro puro) salen de share/neptunia/palette.nix.
        colors = palette.consoleColors;
        font = xnuFont;
        # The console uses the same xkb layout as the desktop.
        useXkbConfig = true;
    };

    # kmscon reemplaza los gettys de las VT (alias autovt@.service). Con el motor
    # psf font-size es la altura en píxeles y se redondea a un múltiplo entero
    # de 16 (32 = doble); aquí 16 = la altura nativa.
    config.services.kmscon = {
        enable = true;
        useXkbConfig = true;
        config = {
            hwaccel = config.nixdots.console.hwRender;
            font-engine = "psf";
            font-name = xnuFont;
            font-size = 16;
        } // builtins.listToAttrs (
            map (e: { name = palette.name e.suffix; inherit (e) value; }) palette.entries
        );
    };
}
