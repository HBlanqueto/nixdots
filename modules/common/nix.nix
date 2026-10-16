{ lib, inputs, data, ... }:

{
    nix = {
        settings = {
            experimental-features = [ "nix-command" "flakes" ];
        };

        optimise.automatic = true;

        # Semanalmente recoge lo que no se use en 14 días; sin esto /nix (y las
        # generaciones que lanzaboote deja en la ESP) crecen sin límite.
        gc = {
            automatic = true;
            dates = "weekly";
            options = "--delete-older-than 14d";
        };
    };

    # Mantén vivos los sources de los inputs del flake (fuentes, temas...),
    # para que el GC semanal no obligue a re-descargarlos.
    system.extraDependencies =
        lib.mapAttrsToList (_: i: i.outPath) (lib.filterAttrs (n: _: n != "self") inputs);

    nixpkgs = {
        config = {
            allowUnfree = true;
        };
        overlays = [
            (import ../../overlays { inherit inputs; })
        ];
    };

    documentation = {
        nixos.enable = false;
        man.enable = false;
    };

    system.stateVersion = data.settings.stateVersion;
}
