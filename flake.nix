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

        disko.url = "github:nix-community/disko";
        disko.inputs.nixpkgs.follows = "nixpkgs";
    };

    outputs = inputs@{ self, nixpkgs, ... }:
        let
            settings = import ./settings.nix;

            # The installer is built for both architectures the ISO is offered
            # on. It only orchestrates other tools, so the same source compiles
            # on either.
            supportedSystems = [ "x86_64-linux" "aarch64-linux" ];

            mkInstaller = system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};

                    # Everything the installer shells out to. makeWrapper puts
                    # these on its PATH so the binary works from a minimal ISO
                    # without pulling them in by hand.
                    runtimeInputs = with pkgs; [
                        inputs.disko.packages.${system}.disko
                        btrfs-progs
                        dosfstools
                        util-linux
                        mkpasswd
                        sbctl
                        less
                        git
                        nix
                    ];
                in
                pkgs.rustPlatform.buildRustPackage {
                    pname = "nixdots-installer";
                    version = "0.1.0";

                    # Whole repo so the palette test can read
                    # share/themes/neptunia.nix; the crate lives in installer/.
                    src = ./.;
                    # cargoRoot locates Cargo.lock/Cargo.toml; buildAndTestSubdir
                    # makes the hooks actually run cargo inside installer/.
                    cargoRoot = "installer";
                    buildAndTestSubdir = "installer";
                    cargoLock.lockFile = ./installer/Cargo.lock;

                    nativeBuildInputs = [ pkgs.makeWrapper ];

                    postInstall = ''
                        wrapProgram $out/bin/nixdots-installer \
                            --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
                    '';

                    meta = {
                        description = "Text-mode installer for the nixdots NixOS configuration";
                        mainProgram = "nixdots-installer";
                        license = pkgs.lib.licenses.gpl2Only;
                        platforms = supportedSystems;
                    };
                };

            installers = nixpkgs.lib.genAttrs supportedSystems mkInstaller;
        in
        {
            nixosConfigurations.${settings.hostName} = nixpkgs.lib.nixosSystem {
                system = settings.system;

                specialArgs = {
                    inherit inputs;
                    inherit (settings) system hostName timeZone defaultLocale username userdescription hashedPasswordFile gitName gitEmail stateVersion;
                };

                modules = [ ./etc ];
            };

            packages = nixpkgs.lib.genAttrs supportedSystems (system: {
                installer = installers.${system};
                default = installers.${system};
            });

            apps = nixpkgs.lib.genAttrs supportedSystems (system: {
                installer = {
                    type = "app";
                    program = "${installers.${system}}/bin/nixdots-installer";
                };
                default = {
                    type = "app";
                    program = "${installers.${system}}/bin/nixdots-installer";
                };
            });
        };
}