# Primer arranque derivado de los datos: si settings.nix no trae `username`,
# todavía no hay cuenta y en su lugar se abre el asistente. Con usuario, el
# módulo no hace nada.
{ config, lib, pkgs, settings, ... }:

let
    cfg = config.nixdots.firstBoot;
    firstBoot = !(settings ? username);
in

{
    options.nixdots.firstBoot.command = lib.mkOption {
        type = lib.types.str;
        default = "${pkgs.gnome-text-editor}/bin/gnome-text-editor";
        description = "Comando que se ejecuta en el primer arranque (placeholder del asistente).";
    };

    config = lib.mkIf firstBoot {
        # GDM cede el sitio al asistente; vuelve al reconstruir con usuario.
        services.displayManager.gdm.enable = lib.mkForce false;

        services.greetd = {
            enable = true;

            settings = {
                # Autologin de una sola pasada: el asistente abre a pantalla
                # completa sobre una sesión Wayland mínima (cage) como root.
                initial_session = {
                    command = "${pkgs.cage}/bin/cage -s -- ${cfg.command}";
                    user = "root";
                };

                # Respaldo por si el asistente termina o falla: login de texto.
                default_session = {
                    command = "${pkgs.greetd}/bin/agreety --cmd ${pkgs.bash}/bin/bash";
                };
            };
        };
    };
}
