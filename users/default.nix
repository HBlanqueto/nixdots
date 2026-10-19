# Capa de cuentas: crea `users.users.<nombre>` a partir de
# /persist/keel/users/<nombre>.toml (vía lib/data.nix → data.users). No
# conoce ningún usuario concreto ni rutas de /home: todo sale de data.users.
#
# El home ya no vive aquí: home-manager es independiente (templates.home y
# `~/.config/home-manager`), y el sistema solo aporta el paquete
# `home-manager` (core/user.nix) para aplicarlo.
{ lib, data, ... }:

let
    mkUser = name: u: {
        isNormalUser = true;
        description = u.fullName or name;
        # wheel solo para admins; el resto de grupos son del sistema.
        extraGroups = [ "networkmanager" "video" ]
            ++ lib.optional (u.admin or false) "wheel";
        createHome = true;
        # La ruta de siempre; uid y gid los asigna NixOS (persistidos en
        # /var/lib/nixos).
        hashedPasswordFile = "/persist/secrets/${name}.hash";
    };
in
{
    users.users = lib.mapAttrs mkUser data.users;
}
