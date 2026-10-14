{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        home.inputs.nixpkgs.follows = "nixpkgs";

        somewm.url = "github:HBlanqueto/somewm";

        nix-cachyos-kernel.url = "github:xddxdd/nix-cachyos-kernel/release";
        impermanence.url = "github:nix-community/impermanence";
        ucodenix.url = "github:e-tho/ucodenix";

        lanzaboote.url = "github:nix-community/lanzaboote";
        lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

        brave-previews.url = "github:drishal/brave-browser-flake";
        mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";

        sf-mono-liga-src.url = "github:shaunsingh/SFMono-Nerd-Font-Ligaturized";
        sf-mono-liga-src.flake = false;
    };

    outputs = inputs@{ nixpkgs, ... }:
        let
            settings = import ./settings.nix;
            mkHost = import ./lib/mkHost.nix { inherit inputs settings; };
        in
        {
            nixosConfigurations = {
                # Physical ThinkPad L14 Gen 2a (AMD, x86_64).
                thinkpad = mkHost {
                    hostName = "thinkpad";
                    system = "x86_64-linux";
                    modules = [ ./hosts/thinkpad ];
                };

                # UTM/QEMU VM on an Apple M4 Mac (native aarch64).
                utm = mkHost {
                    hostName = "utm";
                    system = "aarch64-linux";
                    modules = [ ./hosts/utm ];
                };
            };
        };
}
