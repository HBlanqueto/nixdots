{
    description = "Welcome ~/*. Watch your step, it vanishes on boot.";

    inputs = {
        nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

        home.url = "github:nix-community/home-manager";
        home.inputs.nixpkgs.follows = "nixpkgs";

        impermanence.url = "github:nix-community/impermanence";
        impermanence.inputs.nixpkgs.follows = "nixpkgs";
        impermanence.inputs.home-manager.follows = "home";

        ucodenix.url = "github:e-tho/ucodenix";

        lanzaboote.url = "github:nix-community/lanzaboote";
        lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

        brave-previews.url = "github:drishal/brave-browser-flake";
        brave-previews.inputs.nixpkgs.follows = "nixpkgs";
    };

    outputs = inputs@{ nixpkgs, ... }:
        let
            # Único punto de lectura de user.json/machine.json/facter.json: en
            # evaluación pura (nix flake check sin --impure) no ve /persist y
            # todo cae a los valores seguros, por eso la config real exige
            # impure (ver mkHost).
            data = import ./lib/data.nix { };

            # Variante sin datos externos: las salidas de CI no deben leer
            # /persist ni con --impure (no tienen user.json/machine.json/facter).
            emptyData = import ./lib/data.nix { impure = false; };

            # El tema (colores, filecolors, vscode) se importa una sola vez y
            # baja por specialArgs/extraSpecialArgs a los módulos, y como
            # argumento a los paquetes que lo usan.
            theme = import ./share/themes;

            mkHost = import ./lib/mkHost.nix { inherit inputs theme; };

            # Módulos exclusivos del perfil único (asumen el layout de disko y/o
            # un informe de facter). lanzaboote y luks-tpm solo se importan si la
            # máquina pide Secure Boot (secure-boot.nix no puede importarlos
            # condicionalmente, y luks-tpm define boot.lanzaboote, que necesita
            # ese módulo).
            genericModules = machineData:
                [
                    ./modules/common/facter.nix
                    ./modules/common/filesystems-labels.nix
                    ./modules/common/rollback.nix
                    # Igual que lanzaboote: el módulo se importa siempre y
                    # hardware-detect.nix activa ucodenix solo si el informe de
                    # facter ve una CPU AMD en bare-metal.
                    inputs.ucodenix.nixosModules.default
                ]
                ++ nixpkgs.lib.optionals (machineData.secureBoot or false) [
                    inputs.lanzaboote.nixosModules.lanzaboote
                    ./modules/common/luks-tpm.nix
                ];

            # El perfil único detecta el hardware en tiempo de evaluación, así
            # que necesita impure (builtins.currentSystem + los .json en
            # /persist). En evaluación pura cae a x86_64-linux para que
            # `nix flake check` al menos liste la salida.
            mkGeneric = { system, requireImpure ? true, ciOnly ? false }:
                let
                    # CI (ciOnly) usa un data vacío: sin user.json, sin
                    # machine.json y sin facter, aunque se evalúe con --impure.
                    hostData = if ciOnly then emptyData else data;
                in
                mkHost {
                    # Nombre del equipo por defecto según la arquitectura detectada.
                    hostName = hostData.machine.hostName or (
                        if system == "aarch64-linux" then "UTM-aarch64" else "ThinkPad-L14-Gen2"
                    );
                    inherit system requireImpure;
                    machineData = hostData.machine;
                    data = hostData;
                    modules = genericModules hostData.machine
                        ++ nixpkgs.lib.optional ciOnly {
                            # Las salidas puras de CI no tienen cuenta (no hay
                            # user.json), así que satisfacen la assertion de NixOS
                            # "ningún usuario con contraseña". Nunca se arrancan.
                            users.allowNoPasswordLogin = true;
                        };
                };

            # VM de prueba del primer arranque, en ambas arquitecturas. Vive
            # solo en el flake, no en la config real. Solo desktop + first-boot
            # + user.
            #
            # Se construye con NIXDOTS_ROOT vacío (ver run-firstboot-vm.sh), así
            # el flake no ve ningún user.json, firstBoot = !(settings ? username)
            # es true y arranca el asistente. El asistente escribe user.json,
            # reconstruye y el reinicio ya crea la cuenta.
            mkFirstBootVm = { system, hostName }: nixpkgs.lib.nixosSystem {
                inherit system;

                specialArgs = {
                    inherit inputs theme;
                    # hostName es lo único per-config que los módulos necesitan
                    # además de data; se inyecta en data para no tener más
                    # specialArgs sueltos.
                    data = data // { inherit hostName; };
                };

                modules = [
                    ./modules/desktop
                    # nix.nix aporta el overlay (pkgs.xnu-console-font);
                    # console.nix define nixdots.console.font, que user.nix usa
                    # directamente (sin el parche `or "sfmono"`).
                    ./modules/common/nix.nix
                    ./modules/common/console.nix
                    ./modules/common/first-boot.nix
                    ./modules/common/user.nix
                    { system.stateVersion = data.settings.stateVersion; }
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

            # Data tables embedded at compile time with `include_str!` so the
            # binaries never shell out to `locale -a`, `localectl` or
            # `timedatectl` for their choices. Shared by the installer, the
            # setup GUI and the devShell.
            tablesEnv = pkgs: {
                NIXDOTS_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
                # Source locale files (country_name / lang_name) for the
                # generated readable names; only the full glibc ships them.
                NIXDOTS_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
                NIXDOTS_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
                NIXDOTS_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
            };

            # One rust package builder for the three binaries. `runtimeInputs`
            # are put on PATH by makeWrapper; `extra` overrides or adds any
            # buildRustPackage attribute (buildInputs, env vars, meta…).
            mkRustTool = system: { pname, subdir, runtimeInputs ? [ ], extra ? { } }:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                    base = {
                        inherit pname;
                        version = "0.1.0";

                        src = ./.;
                        # buildAndTestSubdir makes the hooks run cargo inside the
                        # crate dir; the Cargo.lock lives at the workspace root.
                        buildAndTestSubdir = subdir;
                        cargoLock.lockFile = ./Cargo.lock;

                        nativeBuildInputs = [ pkgs.makeWrapper ];

                        postInstall = ''
                            wrapProgram $out/bin/${pname} \
                                --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
                        '';

                        meta = {
                            mainProgram = pname;
                            license = pkgs.lib.licenses.gpl2Only;
                            platforms = supportedSystems;
                        };
                    };
                in
                pkgs.rustPlatform.buildRustPackage (
                    base
                    // extra
                    // nixpkgs.lib.optionalAttrs (extra ? meta) {
                        meta = base.meta // extra.meta;
                    }
                );

            # Everything the installer shells out to; makeWrapper puts these on
            # its PATH so the binary works from a minimal ISO.
            installers = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                mkRustTool system {
                    pname = "nixdots-installer";
                    subdir = "installer";
                    runtimeInputs = with pkgs; [
                        disko
                        btrfs-progs
                        dosfstools
                        util-linux
                        cryptsetup
                        systemd
                        mkpasswd
                        sbctl
                        nixos-facter
                        less
                        git
                        nix
                    ];
                    extra = (tablesEnv pkgs) // {
                        meta.description = "Text-mode installer for the nixdots NixOS configuration";
                    };
                });

            # El asistente de primer arranque: solo orquesta mkpasswd, git, nix
            # y nixos-rebuild.
            setups = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                mkRustTool system {
                    pname = "nixdots-setup";
                    subdir = "setup";
                    runtimeInputs = with pkgs; [
                        git
                        nix
                        nixos-rebuild
                        mkpasswd
                        coreutils
                    ];
                    # Tablas que `nixdots-lists` embebe para validar locale y zona.
                    extra = (tablesEnv pkgs) // {
                        meta.description = "First-boot assistant CLI for the nixdots configuration";
                    };
                });

            # Asistente gráfico (Slint): llama al CLI nixdots-setup.
            #
            # Licencia: Slint es GPL-3.0-only (o comercial/royalty-free). El
            # flake declara gpl2Only y GPLv2-only es incompatible con GPLv3.
            # Ver setup-gui/Cargo.toml.
            setupsGui = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                mkRustTool system {
                    pname = "nixdots-setup-gui";
                    subdir = "setup-gui";
                    runtimeInputs = [ setups.${system} ];
                    extra = {
                        nativeBuildInputs = [ pkgs.pkg-config pkgs.makeWrapper ];
                        buildInputs = with pkgs; [
                            fontconfig
                            freetype
                            libxkbcommon
                            libglvnd
                            wayland
                            libx11
                        ];
                        meta.description = "Graphical first-boot assistant for the nixdots configuration (Slint)";
                    };
                });

            # `nix run .#installer` wrapper: opens kmscon (Nerd Fonts + the
            # neptunia palette) on a real VT, then runs the installer inside it.
            launchers = nixpkgs.lib.genAttrs supportedSystems (system:
                nixpkgs.legacyPackages.${system}.callPackage ./packages/installer-launcher.nix {
                    installer = installers.${system};
                    inherit theme;
                });
        in
        let
            # Perfil único: hardware detectado por facter + machine.json. La
            # forma impura resuelve la arquitectura con builtins.currentSystem.
            nixdots = mkGeneric {
                system = if data.impure then builtins.currentSystem else "x86_64-linux";
                requireImpure = true;
            };
        in
        {
            nixosConfigurations = {
                inherit nixdots;

                # Variantes puras para CI (`nix flake check` / `nix build`
                # sin --impure): sin user.json/machine.json/facter.json, no
                # arrancan, solo comprueban que el árbol evalúa en ambas
                # arquitecturas.
                nixdots-x86_64-linux = mkGeneric {
                    system = "x86_64-linux";
                    requireImpure = false;
                    ciOnly = true;
                };
                nixdots-aarch64-linux = mkGeneric {
                    system = "aarch64-linux";
                    requireImpure = false;
                    ciOnly = true;
                };

                firstboot-vm = mkFirstBootVm {
                    system = "x86_64-linux";
                    hostName = "firstboot-vm";
                };
                firstboot-vm-aarch64 = mkFirstBootVm {
                    system = "aarch64-linux";
                    hostName = "firstboot-vm-aarch64";
                };
            }
            # El mismo sistema expuesto con el hostname del equipo, para que
            # `nixos-rebuild --flake ~/nixdots` sin `#…` también funcione.
            // nixpkgs.lib.optionalAttrs (data.machine ? hostName) {
                ${data.machine.hostName} = nixdots;
            };

            # install.sh lee la copia versionada de la paleta sin pasar por
            # Nix; este check falla si dejó de coincidir con lo que genera
            # packages/kmscon-palette.nix.
            checks = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                    generated = pkgs.callPackage ./packages/kmscon-palette.nix { inherit theme; };
                in
                {
                    kmscon-palette = pkgs.runCommand "check-kmscon-palette" { } ''
                        if ! ${pkgs.diffutils}/bin/diff -u ${./share/themes/kmscon-palette} ${generated}; then
                            echo "" >&2
                            echo "share/themes/kmscon-palette está desactualizado. Regéneralo con:" >&2
                            echo "  cp \$(nix build .#kmscon-palette --print-out-paths) share/themes/kmscon-palette" >&2
                            exit 1
                        fi
                        ${pkgs.coreutils}/bin/touch $out
                    '';

                    # El archivo versionado setup-gui/ui/neptunia-tokens.slint
                    # debe coincidir con lo que genera share/themes/neptunia.nix.
                    neptunia-slint = pkgs.runCommand "check-neptunia-slint" { } ''
                        if ! ${pkgs.diffutils}/bin/diff -u ${./setup-gui/ui/neptunia-tokens.slint} ${pkgs.callPackage ./packages/neptunia-slint.nix { }}; then
                            echo "" >&2
                            echo "setup-gui/ui/neptunia-tokens.slint está desactualizado. Regéneralo con:" >&2
                            echo "  cp \$(nix build .#neptunia-slint --print-out-paths) setup-gui/ui/neptunia-tokens.slint" >&2
                            exit 1
                        fi
                        ${pkgs.coreutils}/bin/touch $out
                    '';
                });

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
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/kmscon-palette.nix { inherit theme; };

                # Safe wrapper around nixos-rebuild (packages/nixdots-rebuild.*).
                nixdots-rebuild =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/nixdots-rebuild.nix { };

                # The macOS XNU console font, as PSF2 (kernel) and OTB (kmscon).
                xnu-console-font =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/xnu-console-font.nix { };

                # Tokens de Neptunia para Slint, generados desde share/themes.
                # El archivo vive versionado en setup-gui/ui/neptunia-tokens.slint;
                # regenéralo con `nix build .#neptunia-slint --print-out-paths`.
                neptunia-slint =
                    nixpkgs.legacyPackages.${system}.callPackage ./packages/neptunia-slint.nix { };
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
                    installer = pkgs.mkShell ((tablesEnv pkgs) // {
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

                        # A bare `cargo build` in the dev shell needs the same
                        # tables as the package (include_str!(env!(...))).
                        NIXDOTS_GTK_CSS = "${pkgs.callPackage ./packages/neptunia-css.nix { inherit theme; }}";
                    });
                    # Shell del asistente Slint: compila (winit + femtovg) y abre
                    # la UI; slint-viewer/slint-lsp para editar .slint. Incluye el
                    # CLI nixdots-setup para que «Aplicar» (--dry-run) funcione.
                    setup-gui = pkgs.mkShell {
                        packages = with pkgs; [
                            cargo
                            rustc
                            rustfmt
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
                        # Tablas que `lists` embebe (para `cargo clippy --workspace`).
                        NIXDOTS_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
                        NIXDOTS_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
                        NIXDOTS_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
                        NIXDOTS_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
                        # winit/femtovg cargan libwayland, libEGL… con dlopen, que
                        # no mira el PATH: hacen falta en el loader path.
                        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
                            wayland
                            libxkbcommon
                            libglvnd
                            fontconfig
                            freetype
                            libx11
                        ]);
                    };
                    default = pkgs.mkShell {
                        packages = with pkgs; [
                            cargo
                            rustc
                            gcc
                            pkg-config
                            nixfmt-rfc-style
                            statix
                            deadnix
                            nil
                        ];
                    };
                });
        };
}
