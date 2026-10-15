# Primer arranque derivado de los datos: si settings.nix no trae `username`,
# todavía no hay cuenta y en su lugar se abre el asistente. Con usuario, el
# módulo no hace nada.
{ config, lib, pkgs, settings, ... }:

let
    cfg = config.nixdots.firstBoot;
    firstBoot = !(settings ? username);

    mark = "/persist/nixdots/.firstboot-done";

    # El asistente se lanza a pantalla completa (cage) como root; si termina
    # bien deja una marca en /persist, que sobrevive al rollback. Con la marca
    # puesta y una cuenta ya creada no se relanza (greetd cae a su
    # default_session); sin cuenta se relanza para recuperarse.
    firstBootSession = pkgs.writeShellScript "nixdots-first-boot" ''
        mark=${mark}

        # ¿Hay al menos una cuenta normal? (uid >= 1000, sin cuentas de sistema)
        has_normal_user() {
            ${pkgs.gawk}/bin/awk -F: '$3 >= 1000 && $3 < 60000 && $1 !~ /^nixbld/ && $1 != "nobody" { found = 1 } END { exit !found }' /etc/passwd
        }

        if [ -e "$mark" ] && has_normal_user; then
            exit 0
        fi

        ${pkgs.cage}/bin/cage -s -- ${cfg.command}
        status=$?

        if [ "$status" -eq 0 ]; then
            ${pkgs.coreutils}/bin/mkdir -p /persist/nixdots
            ${pkgs.coreutils}/bin/touch "$mark"
        fi

        exit "$status"
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

                # Respaldo cuando el asistente termina o falla: login de texto.
                default_session = {
                    command = "${pkgs.greetd}/bin/agreety --cmd ${pkgs.bash}/bin/bash";
                };
            };
        };
    };
}
