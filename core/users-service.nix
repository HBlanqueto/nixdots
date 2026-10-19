# Servicio de gestión de cuentas (crewd) y preparación del home en el primer
# inicio de sesión (steward-home). También publica la referencia exacta del
# flake instalado para que crewd reconstruya esa versión y no el repo de trabajo.
{ pkgs, inputs, ... }:

let
  system = pkgs.stdenv.hostPlatform.system;

  userd = inputs.self.packages.${system}.crewd;
  homeInit = inputs.self.packages.${system}.home-init;
  setupGui = inputs.self.packages.${system}.assist-setup;

  # Si ya hay home, el script termina enseguida (y borra la semilla); si no,
  # muestra la pantalla «Preparando tu escritorio» mientras lo prepara.
  homeInitSession = pkgs.writeShellScript "steward-home-session" ''
    if [ -f "$HOME/.config/home-manager/flake.nix" ]; then
        exec ${homeInit}/bin/nixdots-home-init
    fi
    exec ${setupGui}/bin/nixdots-setup-gui --home-init
  '';

  # Polkit lee las acciones de XDG_DATA_DIRS (`share/polkit-1`), no de
  # /etc/polkit-1/actions: se instala como paquete.
  polkitPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.crew.manage.policy";
    destination = "/share/polkit-1/actions/io.github.HBlanqueto.Keel.crew.manage.policy";
    text = ''
      <?xml version="1.0" encoding="UTF-8"?>
      <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
      <policyconfig>
        <vendor>Keel</vendor>
        <action id="io.github.HBlanqueto.Keel.crew.manage">
          <description>Gestionar cuentas de Keel</description>
          <message>Se requiere autenticación de administrador para gestionar cuentas</message>
          <defaults>
            <allow_any>auth_admin</allow_any>
            <allow_inactive>auth_admin</allow_inactive>
            <allow_active>auth_admin</allow_active>
          </defaults>
        </action>
      </policyconfig>
    '';
  };

  # Política del bus del sistema: root es el dueño del nombre y cualquier
  # usuario puede llamar (polkit decide si se autoriza).
  dbusPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.Crew1.conf";
    destination = "/share/dbus-1/system.d/io.github.HBlanqueto.Keel.Crew1.conf";
    text = ''
      <?xml version="1.0"?>
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <policy user="root">
          <allow own="io.github.HBlanqueto.Keel.Crew1"/>
        </policy>
        <policy context="default">
          <allow send_destination="io.github.HBlanqueto.Keel.Crew1"/>
          <allow receive_sender="io.github.HBlanqueto.Keel.Crew1"/>
        </policy>
      </busconfig>
    '';
  };
in
{
  # Referencia exacta de la versión instalada. `source` es el source del
  # flake; `release` es TOML con el commit (o "dirty" si se construyó con
  # cambios). version/codename se definirán más adelante.
  environment = {
    etc."keel/source".text = inputs.self.outPath;
    etc."keel/release".text = ''
      commit = "${inputs.self.rev or "dirty"}"
      version = ""
      codename = ""
    '';

    systemPackages = [ polkitPolicy ];
  };

  # Que el GC no borre la fuente desde la que crewd reconstruye.
  system.extraDependencies = [ inputs.self.outPath ];

  security.polkit.enable = true;
  services.dbus.packages = [ dbusPolicy ];

  systemd.services.crewd = {
    description = "Keel user management service";
    wantedBy = [ "multi-user.target" ];
    after = [ "dbus.service" ];
    requires = [ "dbus.service" ];

    # nixos-rebuild, mkpasswd y nix se usan desde el servicio.
    path = with pkgs; [
      nixos-rebuild
      nix
      git
      mkpasswd
      coreutils
    ];

    environment = {
      # Vacío = /persist (las rutas son ${KEEL_ROOT}/persist/keel).
      KEEL_ROOT = "";
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
  systemd.user.services.steward-home = {
    description = "Prepara el home de Keel";
    wantedBy = [ "default.target" ];
    after = [ "graphical-session.target" ];
    wants = [ "graphical-session.target" ];

    path = [
      homeInit
      pkgs.home-manager
      pkgs.systemd
      pkgs.git
      pkgs.coreutils
    ];

    serviceConfig = {
      # La GUI vive hasta que termina; no bloquea el target de usuario.
      Type = "simple";
      ExecStart = "${homeInitSession}";
    };
  };
}
