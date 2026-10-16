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
            # La evaluación tiene que ser impura para leer user.json:
            # `nix flake check`/`nix eval` puros no ven /persist/nixdots y darían
            # por hecho un primer arranque (borrarían la cuenta). En los hosts
            # reales eso es un error (ver mkHost); firstboot-vm no se ve afectado.
            impure = builtins ? currentSystem;

            # Los datos personales no se versionan: viven en
            # ${NIXDOTS_ROOT}/persist/nixdots/user.json (los escribe el asistente
            # o el instalador). `settings` = settings.nix del repo + esos datos.
            userData =
                let
                    root = builtins.getEnv "NIXDOTS_ROOT";
                    file = root + "/persist/nixdots/user.json";
                in
                if impure && builtins.pathExists file
                then builtins.fromJSON (builtins.readFile file)
                else { };

            hasUser = userData ? username;

            settings = import ./settings.nix // userData;

            # Primer arranque en un host real: solo impuro y sin user.json. Se
            # pasa como specialArg, así en evaluación pura `firstBoot` es false
            # y nunca puede activar el asistente.
            firstBootHost = impure && !hasUser;

            mkHost = import ./lib/mkHost.nix {
                inherit inputs settings impure;
                firstBoot = firstBootHost;
            };

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
                    # En la VM el asistente siempre arranca hasta que el propio
                    # asistente escriba user.json (entonces el rebuild interno ya
                    # crea la cuenta).
                    firstBoot = !hasUser;
                } // import ./lib/specialArgs.nix { inherit settings; };

                modules = [
                    ./modules/desktop
                    ./modules/common/first-boot.nix
                    ./modules/common/user.nix
                    { system.stateVersion = settings.stateVersion; }
                    # Solo en la VM de prueba: root con contraseña para poder
                    # depurar desde tty2. Nunca en los hosts reales.
                    { users.users.root.password = "test"; }

                    # La config base ES la VM (qemu-vm.nix importado aquí, no
                    # como virtualisation.vmVariant): así el `nixos-rebuild
                    # boot` interno construye el mismo sistema que arranca, con
                    # sus fileSystems y su bootloader.
                    (nixpkgs.outPath + "/nixos/modules/virtualisation/qemu-vm.nix")

                    ({ pkgs, ... }: {
                        virtualisation = {
                            memorySize = 4096;
                            cores = 4;

                            # Sin bootloader, QEMU arranca el toplevel original
                            # (kernel/initrd directos) y un reinicio tras
                            # `nixos-rebuild boot` no vería la nueva generación.
                            useBootLoader = true;
                            useEFIBoot = true;
                        };

                        # El bootloader que instala en la imagen de la VM.
                        boot.loader.systemd-boot.enable = true;

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

            # Asistente gráfico (Slint): llama al CLI nixdots-setup.
            #
            # Licencia: Slint es GPL-3.0-only (o comercial/royalty-free). El
            # flake declara gpl2Only y GPLv2-only es incompatible con GPLv3, así
            # que mientras se use Slint hay que elegir la vía royalty-free/
            # comercial de Slint o subir nixdots a algo compatible con GPLv3.
            # Ver setup-gui/Cargo.toml.
            mkSetupGui = system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                pkgs.rustPlatform.buildRustPackage {
                    pname = "nixdots-setup-gui";
                    version = "0.1.0";

                    src = ./.;
                    buildAndTestSubdir = "setup-gui";
                    cargoLock.lockFile = ./Cargo.lock;

                    nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
                    buildInputs = with pkgs; [
                        fontconfig
                        freetype
                        libxkbcommon
                        libglvnd
                        wayland
                        libx11
                    ];

                    postInstall = ''
                        wrapProgram $out/bin/nixdots-setup-gui \
                            --prefix PATH : ${pkgs.lib.makeBinPath [ setups.${system} ]}
                    '';

                    meta = {
                        description = "Graphical first-boot assistant for the nixdots configuration (Slint)";
                        mainProgram = "nixdots-setup-gui";
                        license = pkgs.lib.licenses.gpl2Only;
                        platforms = supportedSystems;
                    };
                };

            setupsGui = nixpkgs.lib.genAttrs supportedSystems mkSetupGui;

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
                setup-gui = setupsGui.${system};
                launcher = launchers.${system};
                default = launchers.${system};

                # The kmscon `--palette*` arguments, generated from the theme.
                # `install.sh` reads the committed copy at
                # share/themes/kmscon-palette; regenerate it with
                # `nix build .#kmscon-palette --print-out-paths`.
                kmscon-palette =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/kmscon-palette.nix { };

                # Tokens de Neptunia para Slint, generados desde share/themes.
                # El archivo vive versionado en setup-gui/ui/neptunia-tokens.slint;
                # regenéralo con `nix build .#neptunia-slint --print-out-paths`.
                neptunia-slint =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/neptunia-slint.nix { };

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

            # El archivo versionado setup-gui/ui/neptunia-tokens.slint debe
            # coincidir con lo que genera share/themes/neptunia.nix.
            checks = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                    generated = pkgs.callPackage ./packages/neptunia-slint.nix { };
                in
                {
                    neptunia-slint = pkgs.runCommand "check-neptunia-slint" { } ''
                        if ! ${pkgs.diffutils}/bin/diff -u ${./setup-gui/ui/neptunia-tokens.slint} ${generated}; then
                            echo "" >&2
                            echo "setup-gui/ui/neptunia-tokens.slint está desactualizado. Regéneralo con:" >&2
                            echo "  cp \$(nix build .#neptunia-slint --print-out-paths) setup-gui/ui/neptunia-tokens.slint" >&2
                            exit 1
                        fi
                        ${pkgs.coreutils}/bin/touch $out
                    '';
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
                            gtk4
                            libadwaita
                        ];

                        # The installer embeds these tables at compile time with
                        # `include_str!(env!(...))`, so a bare `cargo build` in
                        # the dev shell needs the same variables as the package.
                        NIXDOTS_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
                        NIXDOTS_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
                        NIXDOTS_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
                        NIXDOTS_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
                        NIXDOTS_GTK_CSS = "${pkgs.callPackage ./packages/neptunia-css.nix { }}";
                    };
                    # Shell del asistente Slint: compila (winit + femtovg) y abre
                    # la UI; slint-viewer/slint-lsp para editar .slint. Incluye el
                    # CLI nixdots-setup para que «Aplicar» (--dry-run) funcione.
                    setup-gui = pkgs.mkShell {
                        packages = with pkgs; [
                            cargo
                            rustc
                            rust-analyzer
                            clippy
                            watchexec
                            pkg-config
                            slint-viewer
                            slint-lsp
                            fontconfig
                            freetype
                            libxkbcommon
                            libglvnd
                            wayland
                            libx11
                            setups.${system}
                        ];
                        # Tablas que `lists` embebe: así `cargo clippy --workspace`
                        # compila el workspace entero también desde este shell.
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
