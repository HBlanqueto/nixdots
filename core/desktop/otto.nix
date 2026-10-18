# Sesión Otto (nongio/otto), experimental y apagada por defecto. Cuando se
# activa, añade el paquete a GDM como una sesión elegible en el engranaje y su
# portal, sin tocar la de GNOME (el portal usa UseIn=otto y una config aparte).
#
# Se activa sola en hardware real x86_64 con GPU (el backend udev de Otto usa
# DRM/Vulkan; la VM aarch64 UTM con virtio-gpu no tiene Vulkan).
{ config, lib, pkgs, ... }:

let
    cfg = config.nixdots.desktop.otto;

    autoEnable =
        pkgs.stdenv.hostPlatform.isx86_64
        && config.hardware.facter.enable
        && config.hardware.facter.detected.virtualisation.none.enable;
in
{
    options.nixdots.desktop.otto.enable =
        lib.mkEnableOption "sesión Otto (experimental)";

    config = lib.mkMerge [
        { nixdots.desktop.otto.enable = lib.mkDefault autoEnable; }

        (lib.mkIf cfg.enable {
            environment.systemPackages = [ pkgs.otto ];

            # Unidad de usuario del portal (share/systemd/user), para que la
            # activación D-Bus de org.freedesktop.impl.portal.desktop.otto
            # funcione.
            systemd.packages = [ pkgs.otto ];

            # Aparece en el engranaje de GDM.
            services.displayManager.sessionPackages = [ pkgs.otto ];

            xdg.portal = {
                enable = true;
                extraPortals = [ pkgs.otto ];
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
                "${pkgs.otto}/etc/otto/config.toml";

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
