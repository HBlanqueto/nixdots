# keel-userd (F0): esqueleto del servicio. Desactivado por defecto para que
# ningún equipo cambie hasta que F1 sustituya a crewd.
{
  config,
  lib,
  pkgs,
  inputs,
  ...
}:

let
  cfg = config.keel.userd;
  system = pkgs.stdenv.hostPlatform.system;

  dbusPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.Keel.Userd1.conf";
    destination = "/share/dbus-1/system.d/io.github.HBlanqueto.Keel.Userd1.conf";
    text = ''
      <?xml version="1.0"?>
      <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
        "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
      <busconfig>
        <policy user="root">
          <allow own="io.github.HBlanqueto.Keel.Userd1"/>
        </policy>
        <policy context="default">
          <allow send_destination="io.github.HBlanqueto.Keel.Userd1"/>
          <allow receive_sender="io.github.HBlanqueto.Keel.Userd1"/>
        </policy>
      </busconfig>
    '';
  };

  polkitPolicy = pkgs.writeTextFile {
    name = "io.github.HBlanqueto.keel.userd.policy";
    destination = "/share/polkit-1/actions/io.github.HBlanqueto.keel.userd.policy";
    text = builtins.readFile ../services/userd/polkit/io.github.HBlanqueto.keel.userd.policy;
  };
in
{
  options.keel.userd = {
    enable = lib.mkEnableOption "keel-userd, gestión de usuarios en runtime";

    package = lib.mkOption {
      type = lib.types.package;
      default = inputs.self.packages.${system}.keel-userd;
      description = "Paquete del daemon keel-userd.";
    };

    stateDir = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/keel-userd";
      description = "Directorio de estado (KDL de usuarios y journal de jobs).";
    };

    persistence.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Añadir stateDir a environment.persistence.\"/persist\".";
    };

    identity.priority = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "homed"
          "userdb"
          "declared"
        ]
      );
      default = [
        "homed"
        "userdb"
        "declared"
      ];
      description = "Orden de preferencia de los backends de identidad.";
    };

    packages.priority = lib.mkOption {
      type = lib.types.listOf (
        lib.types.enum [
          "nix-profile"
          "home-manager"
          "flatpak"
        ]
      );
      default = [
        "nix-profile"
        "home-manager"
        "flatpak"
      ];
      description = "Orden de preferencia de los backends de paquetes.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.etc."keel/userd.json".text = builtins.toJSON {
      state_dir = cfg.stateDir;
      userdb_dir = "/etc/userdb";
      identity_priority = cfg.identity.priority;
      package_priority = cfg.packages.priority;
    };

    systemd.services.keel-userd = {
      description = "keel-userd user management daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "dbus.service" ];
      requires = [ "dbus.service" ];

      serviceConfig = {
        Type = "dbus";
        BusName = "io.github.HBlanqueto.Keel.Userd1";
        ExecStart = "${cfg.package}/bin/keel-userd";
        Restart = "on-failure";
        RestartSec = 2;
      };
    };

    systemd.tmpfiles.rules = [
      "d ${cfg.stateDir} 0700 root root -"
    ];

    services.dbus.packages = [ dbusPolicy ];
    security.polkit.enable = true;
    environment.systemPackages = [ polkitPolicy ];

    # F1 debe añadir también /var/lib/systemd/home y /etc/userdb.
    environment.persistence."/persist".directories = lib.mkIf cfg.persistence.enable [
      {
        directory = cfg.stateDir;
        user = "root";
        group = "root";
        mode = "0700";
      }
    ];
  };
}
