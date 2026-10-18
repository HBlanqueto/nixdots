# Sesión Otto (HBlanqueto/otto), experimental y apagada por defecto. Cuando se
# activa, añade el paquete a GDM como una sesión elegible en el engranaje y su
# portal, sin tocar la de GNOME (el portal usa UseIn=otto y una config aparte).
#
# Otto se compila desde los forks privados de HBlanqueto (packages/otto.nix), así
# que exige el token de GitHub del nix-daemon (/persist/secrets/nix-github.env,
# ver core/nix.nix). Sin token queda apagado: una instalación nueva no se rompe.
#
# Se activa sola en hardware real x86_64 con GPU y con token (el backend udev de
# Otto usa DRM/Vulkan; la VM aarch64 UTM con virtio-gpu no tiene Vulkan) y
# machine.json puede forzarla con `"otto": true`/`false`, que manda sobre la
# detección; forzarla sin token es un error de configuración.
{ config, lib, pkgs, data, ... }:

let
    cfg = config.nixdots.desktop.otto;

    tokensFile = "/persist/secrets/nix-github.env";

    autoEnable =
        pkgs.stdenv.hostPlatform.isx86_64
        && config.hardware.facter.enable
        && config.hardware.facter.detected.virtualisation.none.enable
        && data.exists tokensFile;
in
{
    options.nixdots.desktop.otto = {
        enable = lib.mkEnableOption "sesión Otto (experimental)";

        package = lib.mkOption {
            type = lib.types.package;
            default = pkgs.otto;
            defaultText = lib.literalExpression "pkgs.otto";
            description = "Paquete de Otto que usa la sesión.";
        };
    };

    config = lib.mkMerge [
        # machine.json manda sobre la detección; si no trae `otto`, se usa
        # autoEnable (que ya exige el token).
        { nixdots.desktop.otto.enable = lib.mkDefault (data.machine.otto or autoEnable); }

        (lib.mkIf cfg.enable {
            # Otto compila los forks privados de HBlanqueto: el nix-daemon
            # necesita el token o la build fallará al descargarlos.
            assertions = [
                {
                    assertion = data.exists tokensFile;
                    message = ''
                        Otto se compila desde los forks privados de HBlanqueto y el
                        nix-daemon necesita un token de GitHub. Créalo como root
                        (root:root, 0600) con las dos variables:
                            ${tokensFile}
                                NIX_GITHUB_PRIVATE_USERNAME=<usuario>
                                NIX_GITHUB_PRIVATE_PASSWORD=<token>
                        y reinicia nix-daemon (systemctl restart nix-daemon). También
                        puedes apagar Otto con `"otto": false` en machine.json.
                    '';
                }
            ];

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
