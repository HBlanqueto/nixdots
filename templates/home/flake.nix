{
  description = "Mi home de nixdots";

  inputs = {
    # Fijado a la revisión del sistema; `nixdots-home-init` lo ajusta al crearlo.
    nixdots.url = "github:HBlanqueto/nixdots";

    nixpkgs.follows = "nixdots/nixpkgs";

    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixdots/nixpkgs";
    };
  };

  outputs = { home-manager, nixpkgs, nixdots, ... }:
    let
      system = "x86_64-linux";
    in
    {
      homeConfigurations.default = home-manager.lib.homeManagerConfiguration {
        pkgs = import nixpkgs {
          inherit system;

          config.allowUnfree = true;

          # brave-previews publica brave-origin-beta usando `xorg.libxcb`, que
          # nixpkgs renombró; el alias evita el aviso y el overlay da el paquete.
          overlays = [
            nixdots.overlays.brave
            (_final: prev: {
              xorg = prev.xorg // { inherit (prev) libxcb; };
            })
          ];
        };

        # `nixdots` (el flake) lo usa apps.nix para el paquete nixdots-rebuild.
        # `prefs` sale de ./prefs.toml (apariencia y accesibilidad), que
        # `nixdots-home-init` copia aquí; sin él, valores por defecto.
        extraSpecialArgs = {
          inherit nixdots;
          prefs =
            if builtins.pathExists ./prefs.toml
            then builtins.fromTOML (builtins.readFile ./prefs.toml)
            else { };
        };

        modules = [
          # Cambia esta línea por nixdots.homeModules.minimal si prefieres el
          # home mínimo (fish, starship, git y el tema neptunia).
          nixdots.homeModules.nixdots
          ./home.nix
        ];
      };
    };
}
