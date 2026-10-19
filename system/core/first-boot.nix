# Primer arranque: `firstBoot` lo calcula system/lib/data.nix (impuro y sin cuentas
# en un host real). Con `firstBoot` false el módulo no hace nada.
{
  config,
  lib,
  pkgs,
  inputs,
  data,
  ...
}:

let
  cfg = config.keel.assistSetup;

  # Asistente gráfico (Slint) bajo cage, como root, con un bus de sesión
  # propio.
  guiCommand = pkgs.writeShellScript "assist-setup-gui" ''
    export XDG_RUNTIME_DIR=/run/user/0
    exec ${pkgs.dbus}/bin/dbus-run-session -- \
        ${inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.assist-setup}/bin/nixdots-setup-gui
  '';

  # Contrato del asistente: salir con código 0 significa que ya escribió
  # las cuentas y ejecutó `nixos-rebuild boot`; entonces se reinicia para
  # arrancar con GDM. Mientras salga != 0 se relanza (con una espera de un
  # segundo para no girar en vacío si cage no llega a arrancar). El bucle
  # nunca deja salir a la sesión de greetd: greetd no ve un greeter que muere
  # y no se cae. El propio asistente ofrece «Apagar» y «Reiniciar».
  firstBootSession = pkgs.writeShellScript "assist-setup" ''
    until ${pkgs.cage}/bin/cage -s -- ${cfg.command}; do
        ${pkgs.coreutils}/bin/sleep 1
    done
    ${pkgs.systemd}/bin/systemctl reboot
  '';
in

{
  options.keel.assistSetup.command = lib.mkOption {
    type = lib.types.str;
    default = "${guiCommand}";
    description = "Comando que se ejecuta en el primer arranque (el asistente gráfico).";
  };

  config = lib.mkIf data.firstBoot {
    # Sin cuenta todavía no hay contraseña; el autologin de greetd entra
    # como root. Al declarar el usuario se vuelve a reconstruir sin esto.
    users.allowNoPasswordLogin = true;

    # GDM cede el sitio al asistente; vuelve al reconstruir con usuario.
    services.displayManager.gdm.enable = lib.mkForce false;

    # El bus de sesión del asistente (root) necesita /run/user/0.
    systemd.tmpfiles.rules = [ "d /run/user/0 0700 root root -" ];

    services.greetd = {
      enable = true;

      settings = {
        # Autologin de una sola pasada: el asistente abre a pantalla
        # completa sobre una sesión Wayland mínima (cage) como root.
        initial_session = {
          command = "${firstBootSession}";
          user = "root";
        };

        # Respaldo si greetd arranca sin initial_session (p. ej. tras un
        # reinicio suyo): login de texto, no el asistente.
        default_session = {
          command = "${pkgs.greetd}/bin/agreety --cmd ${pkgs.bash}/bin/bash";
        };
      };
    };
  };
}
