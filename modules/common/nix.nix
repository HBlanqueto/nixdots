{ pkgs, inputs, stateVersion, ... }:

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
