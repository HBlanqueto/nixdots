{ pkgs, lib, inputs, stateVersion, ... }:

{
    # Impermanence roots common to both hosts. ThinkPad-only persistent dirs
    # (cups, sbctl, auto-cryptenroll) are appended by modules/hardware/thinkpad.nix.
    environment = {
        persistence."/persist" = {
            hideMounts = true;

            directories = [
                "/var/lib/bluetooth"
                "/var/lib/nixos"
                {
                    directory = "/var/lib/ollama/models";
                    user = "ollama";
                    group = "ollama";
                }
                "/etc/NetworkManager/system-connections"
                "/var/lib/AccountsService"
                "/etc/nixos"
            ];

            files = [
                "/etc/machine-id"
            ];
        };
    };

    nix = {
        settings = {
            experimental-features = [ "nix-command" "flakes" ];

            # Kept off: some derivations here rely on pre-existing store paths
            # and would not build in a sandbox.
            sandbox = false;

            # Conserva las dependencias de build de las generaciones vivas:
            # sin esto el GC semanal borra los outputs de los inputs del flake
            # y hay que volver a descargarlos.
            keep-outputs = true;
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

    system.stateVersion = stateVersion;
}
