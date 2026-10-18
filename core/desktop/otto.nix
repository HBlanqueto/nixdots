# Sesión Otto (nongio/otto), experimental y apagada por defecto. Cuando se
# activa, añade el paquete a GDM como una sesión elegible en el engranaje y su
# portal, sin tocar la de GNOME (el portal usa UseIn=otto y una config aparte).
#
# Se activa sola en hardware real x86_64 con GPU (el backend udev de Otto usa
# DRM/Vulkan; la VM aarch64 UTM con virtio-gpu no tiene Vulkan) y machine.json
# puede forzarla con `"otto": true`/`false` (manda sobre la detección).
#
# Fuentes: si existe el archivo con el token de GitHub del nix-daemon (ver
# core/nix.nix), se descargan de las copias privadas de HBlanqueto; si no, de
# nongio. Así una instalación nueva, sin token todavía, sigue funcionando. Los
# commits son los mismos: cambiar de origen no recompila nada.
{ config, lib, pkgs, data, ... }:

let
    cfg = config.nixdots.desktop.otto;

    tokensFile = "/persist/secrets/nix-github.env";

    autoEnable =
        pkgs.stdenv.hostPlatform.isx86_64
        && config.hardware.facter.enable
        && config.hardware.facter.detected.virtualisation.none.enable;
in
{
    options.nixdots.desktop.otto = {
        enable = lib.mkEnableOption "sesión Otto (experimental)";

        source = lib.mkOption {
            type = lib.types.enum [ "mirror" "upstream" ];
            default = if data.exists tokensFile then "mirror" else "upstream";
            defaultText = lib.literalMD
                "`\"mirror\"` si existe `${tokensFile}`; si no, `\"upstream\"`";
            description = ''
                De dónde salen las fuentes de Otto, layers y smithay (mismos
                commits): «mirror» = copias privadas de HBlanqueto, que exigen
                un token de GitHub en ${tokensFile}; «upstream» = nongio.
            '';
        };

        package = lib.mkOption {
            type = lib.types.package;
            default = pkgs.otto.override { useMirror = cfg.source == "mirror"; };
            defaultText = lib.literalExpression
                "pkgs.otto.override { useMirror = cfg.source == \"mirror\"; }";
            description = "Paquete de Otto que usa la sesión.";
        };
    };

    config = lib.mkMerge [
        # machine.json manda sobre la detección; si no trae `otto`, se usa
        # autoEnable (y false en cualquier otro caso).
        { nixdots.desktop.otto.enable = lib.mkDefault (data.machine.otto or autoEnable); }

        (lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];

            # Unidad de usuario del portal (share/systemd/user), para que la
            # activación D-Bus de org.freedesktop.impl.portal.desktop.otto
            # funcione.
            systemd.packages = [ cfg.package ];

            # Aparece en el engranaje de GDM.
            services.displayManager.sessionPackages = [ cfg.package ];

            xdg.portal = {
                enable = true;
                extraPortals = [ cfg.package ];
                # Config solo para la sesión «otto» (otto-portals.conf): GNOME
                # conserva la suya.
                config.otto = {
                    default = "gtk";
                    "org.freedesktop.impl.portal.ScreenCast" = "otto";
                    "org.freedesktop.impl.portal.Settings" = "otto";
                    "org.freedesktop.impl.portal.Access" = "otto";
                    "org.freedesktop.impl.portal.Screenshot" = "otto";
                };
            };

            programs.xwayland.enable = true;

            # Valores por defecto del paquete: tema claro, Inter, scroll natural
            # (y arranque de barra e islas).
            environment.etc."otto/config.toml".source =
                "${cfg.package}/etc/otto/config.toml";

            # La pantalla de bloqueo autentica contra /etc/pam.d/otto-lock.
            security.pam.services.otto-lock.text = ''
                auth required pam_unix.so
                account required pam_unix.so
                session required pam_unix.so
                password required pam_unix.so
            '';
        })
    ];
}
