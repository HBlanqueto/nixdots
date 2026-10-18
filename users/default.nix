# Capa de cuentas: crea `users.users.<nombre>` a partir de
# /persist/nixdots/users/<nombre>.json (vía lib/data.nix → data.users) y cablea
# home-manager desde users/templates/<template>. No conoce ningún usuario
# concreto ni rutas de /home: todo sale de data.users.
{ config, lib, inputs, theme, data, ... }:

let
    cfg = config.nixdots.users;

    templates = {
        nixdots = ./templates/nixdots;
        minimal = ./templates/minimal;
    };

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

    mkHome = _: u: {
        imports = [
            (templates.${u.template} or (throw "nixdots: plantilla desconocida '${u.template}'"))
        ];
        _module.args.userData = u;
    };
in

{
    options.nixdots.users.homeManager.enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
            Configura home-manager para las cuentas de data.users. La VM de
            primer arranque lo desactiva (crea la cuenta sin plantilla).
        '';
    };

    imports = [ inputs.home.nixosModules.home-manager ];

    config = {
        users.users = lib.mapAttrs mkUser data.users;

        home-manager = lib.mkIf cfg.homeManager.enable {
            backupFileExtension = "backup";
            useGlobalPkgs = true;
            useUserPackages = true;

            # `userData` (los datos del propio usuario) entra por `_module.args`
            # en cada cuenta; el tema y los inputs son comunes.
            extraSpecialArgs = { inherit inputs theme; };

            users = lib.mapAttrs mkHome data.users;
        };
    };
}
