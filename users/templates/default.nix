# Módulos de home-manager que el flake exporta como `homeModules`. Inyectan el
# tema del repo para que el home gestionado por NixOS y el home independiente
# (templates.home) compartan la misma definición sin que el consumidor tenga que
# pasar `theme`.
let
    theme = import ../../share/themes;
in
{
    nixdots = { ... }: {
        imports = [ ./nixdots ];
        _module.args.theme = theme;
    };

    minimal = { ... }: {
        imports = [ ./minimal ];
        _module.args.theme = theme;
    };
}
