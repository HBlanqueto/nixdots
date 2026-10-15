{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        home.inputs.nixpkgs.follows = "nixpkgs";

        impermanence.url = "github:nix-community/impermanence";
        ucodenix.url = "github:e-tho/ucodenix";

        lanzaboote.url = "github:nix-community/lanzaboote";
        lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

        brave-previews.url = "github:drishal/brave-browser-flake";

        sf-mono-liga-src.url = "github:shaunsingh/SFMono-Nerd-Font-Ligaturized";
        sf-mono-liga-src.flake = false;

        disko.url = "github:nix-community/disko";
        disko.inputs.nixpkgs.follows = "nixpkgs";
    };

    outputs = inputs@{ nixpkgs, ... }:
        let
            # Los datos personales no se versionan: viven en
            # ${NIXDOTS_ROOT}/persist/nixdots/user.json (los escribe el asistente
            # o el instalador). `settings` = settings.nix del repo + esos datos;
            # sin el archivo no hay campos de usuario y first-boot.nix abre el
            # asistente. `getEnv`/`pathExists` van guardados para no romper la
            # evaluación pura (`nix flake check`).
            userData =
                let
                    root = builtins.getEnv "NIXDOTS_ROOT";
                    file = root + "/persist/nixdots/user.json";
                in
                if (builtins ? currentSystem) && builtins.pathExists file
                then builtins.fromJSON (builtins.readFile file)
                else { };

            settings = import ./settings.nix // userData;
            mkHost = import ./lib/mkHost.nix { inherit inputs settings; };

            # Every directory under hosts/ with a host.nix is a configuration.
            # host.nix declares the per-host `profile` and `system`; the flake
            # output name is the directory name and equals networking.hostName.
            hostNames = builtins.attrNames (
                nixpkgs.lib.filterAttrs (_: type: type == "directory") (builtins.readDir ./hosts)
            );

            mkHostFromDir = name:
                let
                    meta = import (./hosts + "/${name}/host.nix");
                in
                mkHost {
                    hostName = name;
                    inherit (meta) system;
                    modules = [ (./hosts + "/${name}") ];
                };

            hosts = builtins.listToAttrs (
                map (name: { inherit name; value = mkHostFromDir name; }) hostNames
            );

            # VM de prueba del primer arranque, en ambas arquitecturas. Vive en
            # el flake, no en hosts/, porque toda carpeta bajo hosts/ se
            # convierte en un host real. Solo desktop + first-boot + user.
            #
            # Se construye con NIXDOTS_ROOT vacío (ver run-firstboot-vm.sh), así
            # el flake no ve ningún user.json, firstBoot = !(settings ? username)
            # es true y arranca el asistente. El asistente escribe user.json,
            # reconstruye y el reinicio ya crea la cuenta.
            mkFirstBootVm = { system, hostName }: nixpkgs.lib.nixosSystem {
                inherit system;

                specialArgs = {
                    inherit inputs hostName;
                } // import ./lib/specialArgs.nix { inherit settings; };

                modules = [
                    ./modules/desktop
                    ./modules/common/first-boot.nix
                    ./modules/common/user.nix
                    { system.stateVersion = settings.stateVersion; }
                    # Solo en la VM de prueba: root con contraseña para poder
                    # depurar desde tty2. Nunca en los hosts reales.
                    { users.users.root.password = "test"; }
                    ({ pkgs, ... }: {
                        virtualisation.vmVariant = {
                            virtualisation = {
                                memorySize = 4096;
                                cores = 4;

                                # Sin bootloader, QEMU arranca el toplevel
                                # original (kernel/initrd directos) y un reinicio
                                # tras `nixos-rebuild boot` no vería la nueva
                                # generación. Con bootloader, la generación queda
                                # en el qcow2 y el reinicio la arranca.
                                useBootLoader = true;
                                useEFIBoot = true;
                            };

                            # El bootloader que instala en la imagen de la VM.
                            boot.loader.systemd-boot.enable = true;
                        };

                        # El asistente reconstruye desde esta copia del repo.
                        # Solo se siembra si no existe (persiste en el qcow2).
                        systemd.services.nixdots-seed = {
                            wantedBy = [ "multi-user.target" ];
                            before = [ "greetd.service" ];
                            serviceConfig.Type = "oneshot";
                            path = with pkgs; [ git coreutils ];
                            script = ''
                                if [ -e /persist/nixdots/repo ]; then exit 0; fi
                                mkdir -p /persist/nixdots
                                cp -a ${inputs.self} /persist/nixdots/repo
                                chmod -R u+w /persist/nixdots/repo
                                git -C /persist/nixdots/repo init -q
                                git -C /persist/nixdots/repo add -A
                                git -C /persist/nixdots/repo \
                                    -c user.name=seed -c user.email=seed@localhost \
                                    commit -qm seed
                            '';
                        };

                        # Datos de prueba que el asistente aplica al arrancar.
                        environment.etc."nixdots-setup.json".text = builtins.toJSON {
                            fullName = "Prueba";
                            username = "prueba";
                            locale = "es_MX.UTF-8";
                            keyboardLayout = "es";
                            keyboardVariant = "";
                            timeZone = "America/Merida";
                        };

                        # El asistente corre solo (cage) con estos datos; la
                        # contraseña de prueba llega por stdin. Los hosts reales
                        # conservan el placeholder de first-boot.nix.
                        nixdots.firstBoot.command =
                            "${pkgs.writeShellScript "firstboot-vm-setup" ''
                                printf 'test1234\n' | ${setups.${system}}/bin/nixdots-setup \
                                    apply /etc/nixdots-setup.json --repo /persist/nixdots/repo
                            ''}";
                    })
                ];
            };

            # The installer is built for both architectures the ISO is offered
            # on; it only orchestrates other tools.
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
                        cryptsetup
                        systemd
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
                    # buildAndTestSubdir makes the hooks run cargo inside
                    # installer/; the Cargo.lock lives at the workspace root.
                    buildAndTestSubdir = "installer";
                    cargoLock.lockFile = ./Cargo.lock;

                    nativeBuildInputs = [ pkgs.makeWrapper ];

                    # Data tables embedded at compile time with `include_str!`
                    # so the installer never shells out to `locale -a`,
                    # `localectl` or `timedatectl` for its choices.
                    NIXDOTS_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
                    # Source locale files (country_name / lang_name) for the
                    # generated readable names; only the full glibc ships them.
                    NIXDOTS_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
                    NIXDOTS_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
                    NIXDOTS_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";

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

            # El asistente de primer arranque: mismo patrón que el instalador,
            # pero solo orquesta mkpasswd, git, nix y nixos-rebuild.
            mkSetup = system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};

                    runtimeInputs = with pkgs; [
                        git
                        nix
                        nixos-rebuild
                        mkpasswd
                        coreutils
                    ];
                in
                pkgs.rustPlatform.buildRustPackage {
                    pname = "nixdots-setup";
                    version = "0.1.0";

                    src = ./.;
                    buildAndTestSubdir = "setup";
                    cargoLock.lockFile = ./Cargo.lock;

                    nativeBuildInputs = [ pkgs.makeWrapper ];

                    postInstall = ''
                        wrapProgram $out/bin/nixdots-setup \
                            --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
                    '';

                    meta = {
                        description = "First-boot assistant CLI for the nixdots configuration";
                        mainProgram = "nixdots-setup";
                        license = pkgs.lib.licenses.gpl2Only;
                        platforms = supportedSystems;
                    };
                };

            setups = nixpkgs.lib.genAttrs supportedSystems mkSetup;

            # `nix run .#installer` wrapper: opens kmscon (Nerd Fonts + the
            # neptunia palette) on a real VT, then runs the installer inside it.
            launchers = nixpkgs.lib.genAttrs supportedSystems (system:
                nixpkgs.legacyPackages.${system}.callPackage ./packages/installer-launcher.nix {
                    installer = installers.${system};
                });
        in
        {
            nixosConfigurations = hosts // {
                firstboot-vm = mkFirstBootVm {
                    system = "x86_64-linux";
                    hostName = "firstboot-vm";
                };
                firstboot-vm-aarch64 = mkFirstBootVm {
                    system = "aarch64-linux";
                    hostName = "firstboot-vm-aarch64";
                };
            };

            packages = nixpkgs.lib.genAttrs supportedSystems (system: {
                installer = installers.${system};
                setup = setups.${system};
                launcher = launchers.${system};
                default = launchers.${system};

                # The kmscon `--palette*` arguments, generated from the theme.
                # `install.sh` reads the committed copy at
                # share/themes/kmscon-palette; regenerate it with
                # `nix build .#kmscon-palette --print-out-paths`.
                kmscon-palette =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/kmscon-palette.nix { };

                # Safe wrapper around nixos-rebuild (packages/nixdots-rebuild.*).
                nixdots-rebuild =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/nixdots-rebuild.nix { };
            });

            apps = nixpkgs.lib.genAttrs supportedSystems (system: {
                installer = {
                    type = "app";
                    program = "${launchers.${system}}/bin/nixdots-installer";
                };
                default = {
                    type = "app";
                    program = "${launchers.${system}}/bin/nixdots-installer";
                };
            });

            # Development shell for the live WezTerm window: cargo + rehearse
            # on every save with watchexec.
            devShells = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                {
                    installer = pkgs.mkShell {
                        packages = with pkgs; [
                            cargo
                            rustc
                            rust-analyzer
                            clippy
                            watchexec
                            gcc
                            # The released musl binaries link with clang + lld
                            # (see installer/.cargo/config.toml).
                            clang
                            lld
                            pkg-config
                        ];

                        # The installer embeds these tables at compile time with
                        # `include_str!(env!(...))`, so a bare `cargo build` in
                        # the dev shell needs the same variables as the package.
                        NIXDOTS_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
                        NIXDOTS_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
                        NIXDOTS_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
                        NIXDOTS_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
                    };
                    default = pkgs.mkShell {
                        packages = with pkgs; [ cargo rustc gcc pkg-config ];
                    };
                });
        };
}
