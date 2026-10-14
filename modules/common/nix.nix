{ pkgs, inputs, stateVersion, ... }:

{
    # Impermanence roots common to both hosts. ThinkPad-only persistent dirs
    # (cups, sbctl, auto-cryptenroll) are appended by modules/hardware/thinkpad.nix.
    environment = {
        persistence."/persist" = {
            hideMounts = true;

            directories = [
                "/var/log"
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
            substituters = [
                "https://attic.xuyh0120.win/lantian"
            ];
            trusted-public-keys = [
                "lantian:EeAUQ+W+6r7EtwnmYjeVwx5kOGEBpjlBfPlzGlTNvHc="
            ];

            # Kept off: some derivations here (e.g. the pinned opencode) rely on
            # pre-existing store paths and would not build in a sandbox.
            sandbox = false;
        };

        optimise.automatic = true;
    };

    nixpkgs = {
        config = {
            allowUnfree = true;
        };
        overlays = [
            # The CachyOS kernel overlay is x86_64-only and is added by the
            # ThinkPad host, not here.
            (import ../../overlays { inherit inputs; })
        ];
    };

    documentation = {
        nixos.enable = false;
        man.enable = false;
    };

    system.stateVersion = stateVersion;
}
