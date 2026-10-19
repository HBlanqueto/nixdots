# Servicio de gestión de cuentas (nixdots-userd) y preparación del home en el
# primer inicio de sesión (nixdots-home-init). También publica la referencia
# exacta del flake instalado para que userd reconstruya esa versión y no el repo
# de trabajo.
{ config, lib, pkgs, inputs, ... }:

let
    system = pkgs.stdenv.hostPlatform.system;

    userd = inputs.self.packages.${system}.userd;
    homeInit = inputs.self.packages.${system}.home-init;
    setupGui = inputs.self.packages.${system}.setup-gui;

    # Si ya hay home, el script termina enseguida (y borra la semilla); si no,
    # muestra la pantalla «Preparando tu escritorio» mientras lo prepara.
    homeInitSession = pkgs.writeShellScript "nixdots-home-init-session" ''
        if [ -f "$HOME/.config/home-manager/flake.nix" ]; then
            exec ${homeInit}/bin/nixdots-home-init
        fi
        exec ${setupGui}/bin/nixdots-setup-gui --home-init
    '';

    polkitPolicy = pkgs.writeText "org.nixdots.users.manage.policy" ''
        <?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
          "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
        <policyconfig>
          <vendor>nixdots</vendor>
          <action id="org.nixdots.users.manage">
            <description>Gestionar cuentas de nixdots</description>
            <message>Se requiere autenticación de administrador para gestionar cuentas</message>
            <defaults>
              <allow_any>auth_admin</allow_any>
              <allow_inactive>auth_admin</allow_inactive>
              <allow_active>auth_admin</allow_active>
            </defaults>
          </action>
        </policyconfig>
    '';
in
{
    # Referencia exacta de la versión instalada. `flake-ref` es el source del
    # flake; `rev` es el commit (o "dirty" si se construyó con cambios).
    environment.etc."nixdots/flake-ref".text = inputs.self.outPath;
    environment.etc."nixdots/rev".text = inputs.self.rev or "dirty";

    # Que el GC no borre la fuente desde la que userd reconstruye.
    system.extraDependencies = [ inputs.self.outPath ];

    security.polkit.enable = true;
    environment.etc."polkit-1/actions/org.nixdots.users.manage.policy".source = polkitPolicy;

    systemd.services.nixdots-userd = {
        description = "nixdots user management service";
        wantedBy = [ "multi-user.target" ];
        after = [ "dbus.service" ];
        requires = [ "dbus.service" ];

        # nixos-rebuild, mkpasswd y nix se usan desde el servicio.
        path = with pkgs; [ nixos-rebuild nix git mkpasswd coreutils ];

        environment = {
            NIXDOTS_ROOT = "/persist";
            NIX_CONFIG = "experimental-features = nix-command flakes";
        };

        serviceConfig = {
            ExecStart = "${userd}/bin/nixdots-userd";
            Restart = "on-failure";
            RestartSec = 2;
        };
    };

    # Servicio de usuario: en cada inicio de sesión comprueba si falta el home.
    # Si ya existe, ignora y borra la semilla; si no, lo prepara.
    systemd.user.services.nixdots-home-init = {
        description = "Prepara el home de nixdots";
        wantedBy = [ "default.target" ];
        after = [ "graphical-session.target" ];
        wants = [ "graphical-session.target" ];

        path = [ homeInit pkgs.home-manager pkgs.systemd pkgs.git pkgs.coreutils ];

        serviceConfig = {
            # La GUI vive hasta que termina; no bloquea el target de usuario.
            Type = "simple";
            ExecStart = "${homeInitSession}";
        };
    };
}
