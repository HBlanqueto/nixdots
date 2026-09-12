{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        home.inputs.nixpkgs.follows = "nixpkgs";

        somewm.url = "github:trip-zip/somewm";

        nix-cachyos-kernel.url = "github:xddxdd/nix-cachyos-kernel/release";
        impermanence.url = "github:nix-community/impermanence";
        ucodenix.url = "github:e-tho/ucodenix";

        lanzaboote.url = "github:nix-community/lanzaboote";
        lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

        opencode.url = "github:dan-online/opencode-nix";
        brave-previews.url = "github:drishal/brave-browser-flake";
        mac-style.url = "github:SergioRibera/s4rchiso-plymouth-theme";

        sf-mono-liga-src.url = "github:shaunsingh/SFMono-Nerd-Font-Ligaturized";
        sf-mono-liga-src.flake = false;
    };

    outputs = inputs@{ self, nixpkgs, ... }:
        let
            settings = import ./settings.nix;
        in
        {
            nixosConfigurations.${settings.hostName} = nixpkgs.lib.nixosSystem {
                system = settings.system;

                specialArgs = {
                    inherit inputs;
                    inherit (settings) system hostName timeZone defaultLocale username userdescription hashedpassword gitName gitEmail stateVersion;
                };

                modules = [ ./etc ];
            };
        };
}