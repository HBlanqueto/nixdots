{
  description = "Mi home de Keel";

  inputs = {
    # Fijado a la revisión del sistema; `steward home prepare` lo ajusta al crearlo.
    keel.url = "github:HBlanqueto/dotfiles";

    nixpkgs.follows = "keel/nixpkgs";

    # La misma revisión de home-manager que usa Keel.
    home-manager.follows = "keel/home";
  };

  outputs =
    {
      home-manager,
      nixpkgs,
      keel,
      ...
    }:
    let
      # steward home prepare sustituye SYSTEM por la arquitectura real del sistema.
      system = "SYSTEM";
    in
    {
      homeConfigurations.default = home-manager.lib.homeManagerConfiguration {
        pkgs = import nixpkgs {
          inherit system;

          config.allowUnfree = true;

          # brave-previews publica brave-origin-beta usando `xorg.libxcb`, que
          # nixpkgs renombró; el alias evita el aviso y el overlay da el paquete.
          overlays = [
            keel.overlays.brave
            (_final: prev: {
              xorg = prev.xorg // {
                inherit (prev) libxcb;
              };
            })
          ];
        };

        # `keel` (el flake) lo usa apps.nix para las herramientas del sistema.
        # `prefs` sale de ./prefs.toml (apariencia y accesibilidad), que
        # `steward home prepare` copia aquí; sin él, valores por defecto.
        extraSpecialArgs = {
          inherit keel;
          prefs =
            if builtins.pathExists ./prefs.toml then
              builtins.fromTOML (builtins.readFile ./prefs.toml)
            else
              { };
        };

        modules = [
          # Cambia esta línea por keel.homeModules.raft si prefieres el home
          # mínimo (fish, starship, git y el tema neptunia).
          keel.homeModules.deck
          ./home.nix
        ];
      };
    };
}
