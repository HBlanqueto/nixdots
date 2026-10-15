# Primer arranque derivado de los datos: si settings.nix no trae `username`,
# todavía no hay cuenta y en su lugar se abre el asistente. Con usuario, el
# módulo no hace nada.
{ config, lib, pkgs, settings, ... }:

let
    cfg = config.nixdots.firstBoot;
    firstBoot = (settings.username or null) == null;

    # Contrato del asistente: salir con código 0 significa que ya escribió
    # settings.nix con un usuario y ejecutó `nixos-rebuild switch`, lo que
    # desactiva este módulo y activa GDM. Mientras salga != 0 se relanza; el
    # propio asistente debe ofrecer «Apagar» y «Reiniciar» (`systemctl poweroff`
    # / `reboot`), porque con esta sesión no hay un login de rescate. Entre
    # intentos se espera un segundo para no girar en vacío si cage no arranca.
    firstBootSession = pkgs.writeShellScript "nixdots-first-boot" ''
        while true; do
            ${pkgs.cage}/bin/cage -s -- ${cfg.command}
            status=$?
            if [ "$status" -eq 0 ]; then
                exit 0
            fi
            ${pkgs.coreutils}/bin/sleep 1
        done
    '';
in

{
    options.nixdots.firstBoot.command = lib.mkOption {
        type = lib.types.str;
        default = "${pkgs.gnome-text-editor}/bin/gnome-text-editor";
        description = "Comando que se ejecuta en el primer arranque (placeholder del asistente).";
    };

    config = lib.mkIf firstBoot {
        # Sin cuenta todavía no hay contraseña; el autologin de greetd entra
        # como root. Al declarar el usuario se vuelve a reconstruir sin esto.
        users.allowNoPasswordLogin = true;

        # GDM cede el sitio al asistente; vuelve al reconstruir con usuario.
        services.displayManager.gdm.enable = lib.mkForce false;

        services.greetd = {
            enable = true;

            settings = {
                # Autologin de una sola pasada: el asistente abre a pantalla
                # completa sobre una sesión Wayland mínima (cage) como root.
                initial_session = {
                    command = "${firstBootSession}";
                    user = "root";
                };

                # Si greetd se reinicia, vuelve al asistente (y no a un login
                # inútil sin cuentas). El propio asistente da apagar/reiniciar.
                default_session = {
                    command = "${firstBootSession}";
                    user = "root";
                };
            };
        };
    };
}
