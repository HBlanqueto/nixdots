{ inputs, stateVersion, ... }:

{
    imports = [
        inputs.impermanence.nixosModules.impermanence
        inputs.home.nixosModules.home-manager
        inputs.ucodenix.nixosModules.default
        inputs.lanzaboote.nixosModules.lanzaboote

        ../hardware-configuration.nix
        ../boot

        ./global.nix
        ./services.nix
        ./uutils.nix
        ./fonts.nix

        ./desktop/gnome.nix
    ];

    environment = {
        persistence."/persist" = {
            hideMounts = true;
                directories = [
                    "/var/log"
                    "/var/lib/bluetooth"
                    "/var/lib/nixos"
                    "/var/lib/sbctl"
                    "/var/lib/auto-cryptenroll"
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
            cores = 0;
            max-jobs = "auto";

            experimental-features = [ "nix-command" "flakes" ];
            substituters = [
                "https://attic.xuyh0120.win/lantian" # CachyOS Kernel
                "https://hyprland.cachix.org" # Hyprland
            ];
            trusted-public-keys = [
                "lantian:EeAUQ+W+6r7EtwnmYjeVwx5kOGEBpjlBfPlzGlTNvHc="
                "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
            ];
        };
    };

    nixpkgs = {
        config = {
            allowUnfree = true;
        };
        overlays = [
            inputs.nix-cachyos-kernel.overlays.pinned
            (import ../overlays { inherit inputs; })
        ];
    };

    documentation = {
        nixos = {
            enable = false;
        };

        man = {
            enable = false;

            cache = {
                enable = false;
            };
        };
    };

    hardware = {
        enableRedistributableFirmware = true;

        graphics = {
            enable = true;
        };

        amdgpu = {
            initrd = {
                enable = true;
            };
        };

        bluetooth = {
            enable = true;
            powerOnBoot = true;
        };
    };

    system = {
        stateVersion = stateVersion;
    };
}