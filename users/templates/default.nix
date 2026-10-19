# Módulos de home-manager que el flake exporta como `homeModules`. Inyectan el
# tema del repo para que el home gestionado por NixOS y el home independiente
# (templates.home) compartan la misma definición sin que el consumidor tenga que
# pasar `theme`.
let
    theme = import ../../share/themes;

    # Identidad de git del usuario: se rellena en home.nix.
    gitOptions = { lib, ... }: {
        options.nixdots.git = {
            name = lib.mkOption {
                type = lib.types.str;
                default = "";
                description = "Nombre de git del usuario.";
            };
            email = lib.mkOption {
                type = lib.types.str;
                default = "";
                description = "Correo de git del usuario.";
            };
        };
    };
in
{
    nixdots = { ... }: {
        imports = [ gitOptions ./nixdots ];
        _module.args.theme = theme;
    };

    minimal = { ... }: {
        imports = [ gitOptions ./minimal ];
        _module.args.theme = theme;
    };
}
