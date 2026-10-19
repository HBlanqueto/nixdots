# Módulos de home-manager que el flake exporta como `homeModules`. Inyectan el
# tema del repo para que el home gestionado por NixOS y el home independiente
# (templates.home) compartan la misma definición sin que el consumidor tenga que
# pasar `theme`.
let
    theme = import ../../share/neptunia;

    # Identidad de git del usuario: se rellena en home.nix.
    gitOptions = { lib, ... }: {
        # shortcut: el home antiguo aún escribe nixdots.git; retíralo tras
        # migrar ~/.config/home-manager con `steward home prepare`.
        imports = [ (lib.mkAliasOptionModule [ "nixdots" "git" ] [ "keel" "git" ]) ];

        options.keel.git = {
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
rec {
    deck = { ... }: {
        imports = [ gitOptions ./deck ];
        _module.args.theme = theme;
    };

    raft = { ... }: {
        imports = [ gitOptions ./raft ];
        _module.args.theme = theme;
    };

    # shortcut: alias para ~/.config/home-manager antes de migrarlo. Retíralos
    # cuando el home use deck/raft.
    nixdots = deck;
    minimal = raft;
}
