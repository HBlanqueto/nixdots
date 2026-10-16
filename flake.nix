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

            # Igual que userData, pero per-machine: qué disco/cifrado/Secure
            # Boot y qué periféricos. Sin el archivo cae a los defaults seguros
            # del módulo (sin cifrado, sin Secure Boot, extras apagados).
            machineData =
                let
                    root = builtins.getEnv "NIXDOTS_ROOT";
                    file = root + "/persist/nixdots/machine.json";
                in
                if impure && builtins.pathExists file
                then builtins.fromJSON (builtins.readFile file)
                else { };

            mkHost = import ./lib/mkHost.nix {
                inherit inputs settings impure;
                firstBoot = firstBootHost;
            };

            # Módulos exclusivos del perfil único (asumen el layout de disko y/o
            # un informe de facter); no los importa ningún host con hardware
            # propio. lanzaboote y luks-tpm solo se importan si la máquina pide
            # Secure Boot (secure-boot.nix no puede importarlos condicionalmente,
            # y luks-tpm define boot.lanzaboote, que necesita ese módulo).
            genericModules =
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
            mkGeneric = { system, requireImpure ? true, ciOnly ? false }: mkHost {
                hostName = machineData.hostName or "nixdots";
                inherit system machineData requireImpure;
                modules = genericModules
                    ++ nixpkgs.lib.optional ciOnly {
                        # Las salidas puras de CI no tienen cuenta (no hay
                        # user.json), así que satisfacen la assertion de NixOS
                        # "ningún usuario con contraseña". Nunca se arrancan.
                        users.allowNoPasswordLogin = true;
                    };
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
                    extra.meta.description = "First-boot assistant CLI for the nixdots configuration";
                });

            # Asistente gráfico (GTK4 + libadwaita): llama al CLI nixdots-setup.
            setupsGui = nixpkgs.lib.genAttrs supportedSystems (system:
                let
                    pkgs = nixpkgs.legacyPackages.${system};
                in
                mkRustTool system {
                    pname = "nixdots-setup-gui";
                    subdir = "setup-gui";
                    runtimeInputs = [ setups.${system} ];
                    extra = (tablesEnv pkgs) // {
                        NIXDOTS_GTK_CSS = "${pkgs.callPackage ./packages/neptunia-css.nix { }}";
                        nativeBuildInputs = [ pkgs.pkg-config pkgs.wrapGAppsHook4 ];
                        buildInputs = [ pkgs.gtk4 pkgs.libadwaita ];
                        meta.description = "Graphical first-boot assistant for the nixdots configuration";
                    };
                });

            # `nix run .#installer` wrapper: opens kmscon (Nerd Fonts + the
            # neptunia palette) on a real VT, then runs the installer inside it.
            launchers = nixpkgs.lib.genAttrs supportedSystems (system:
                nixpkgs.legacyPackages.${system}.callPackage ./packages/installer-launcher.nix {
                    installer = installers.${system};
                });
        in
        {
            nixosConfigurations = hosts // {
                # Perfil único: hardware detectado por facter + machine.json. La
                # forma impura resuelve la arquitectura con builtins.currentSystem.
                nixdots = mkGeneric {
                    system = if impure then builtins.currentSystem else "x86_64-linux";
                    requireImpure = true;
                };

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
                        NIXDOTS_GTK_CSS = "${pkgs.callPackage ./packages/neptunia-css.nix { }}";
                    });
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
