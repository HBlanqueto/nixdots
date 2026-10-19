{
  description = "Welcome ~/*. Watch your step, it vanishes on boot.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    home.url = "github:nix-community/home-manager";
    home.inputs.nixpkgs.follows = "nixpkgs";

    impermanence = {
      url = "github:nix-community/impermanence";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.home-manager.follows = "home";
    };

    ucodenix.url = "github:e-tho/ucodenix";

    lanzaboote.url = "github:nix-community/lanzaboote";
    lanzaboote.inputs.nixpkgs.follows = "nixpkgs";

    brave-previews.url = "github:drishal/brave-browser-flake";
    brave-previews.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ nixpkgs, ... }:
    let
      # Único punto de lectura de user.json/machine.json/facter.json: en
      # evaluación pura (nix flake check sin --impure) no ve /persist y
      # todo cae a los valores seguros, por eso la config real exige
      # impure (ver mkHost).
      data = import ./lib/data.nix { };

      # Variante sin datos externos: las salidas de CI no deben leer
      # /persist ni con --impure (no tienen user.json/machine.json/facter).
      emptyData = import ./lib/data.nix { impure = false; };

      # El tema (colores, filecolors, vscode) y el diseño se importan una
      # sola vez y bajan por specialArgs/extraSpecialArgs a los módulos,
      # y como argumento a los paquetes que los usan.
      theme = import ./kits/neptunia;

      mkHost = import ./lib/mkHost.nix { inherit inputs theme; };

      # Módulos exclusivos del perfil único (asumen el layout de disko y/o
      # un informe de facter). lanzaboote y luks-tpm se importan siempre:
      # secure-boot.nix decide con mkIf y luks-tpm define boot.lanzaboote.
      genericModules = [
        ./core/facter.nix
        ./core/filesystems.nix
        ./core/rollback.nix
        # Igual que lanzaboote: el módulo se importa siempre y
        # hardware-detect.nix activa ucodenix solo si el informe de
        # facter ve una CPU AMD en bare-metal.
        inputs.ucodenix.nixosModules.default
        inputs.lanzaboote.nixosModules.lanzaboote
        ./core/luks-tpm.nix
      ];

      # El perfil único detecta el hardware en tiempo de evaluación, así
      # que necesita impure (builtins.currentSystem + los .json en
      # /persist). En evaluación pura cae a x86_64-linux para que
      # `nix flake check` al menos liste la salida.
      mkGeneric =
        {
          system,
          requireImpure ? true,
          data,
        }:
        mkHost {
          inherit system requireImpure data;
          modules =
            genericModules
            ++ nixpkgs.lib.optional (!data.hasUsers) {
              # Las salidas sin cuenta (CI, o un equipo sin user.json)
              # satisfacen la assertion de NixOS "ningún usuario con
              # contraseña". Las de CI nunca se arrancan.
              users.allowNoPasswordLogin = true;
            };
        };

      # VM de prueba del primer arranque, en ambas arquitecturas (ver
      # lib/firstboot-vm.nix).
      mkFirstBootVm = import ./lib/firstboot-vm.nix {
        inherit
          nixpkgs
          inputs
          theme
          data
          ;
      };

      # The installer is built for both architectures the ISO is offered
      # on; it only orchestrates other tools.
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      # Una salida por arquitectura: f recibe { system, pkgs }.
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            inherit system;
            pkgs = nixpkgs.legacyPackages.${system};
          }
        );

      # Data tables embedded at compile time with `include_str!` so the
      # binaries never shell out to `locale -a`, `localectl` or
      # `timedatectl` for their choices. Shared by the installer, the
      # setup GUI and the devShell.
      tablesEnv = pkgs: {
        KEEL_LOCALES = "${pkgs.glibcLocales}/share/i18n/SUPPORTED";
        # Source locale files (country_name / lang_name) for the
        # generated readable names; only the full glibc ships them.
        KEEL_LOCALE_DIR = "${pkgs.glibc}/share/i18n/locales";
        KEEL_XKB_BASE_LST = "${pkgs.xkeyboard_config}/share/X11/xkb/rules/base.lst";
        KEEL_ZONE1970_TAB = "${pkgs.tzdata}/share/zoneinfo/zone1970.tab";
      };

      # Constructor común de los binarios Rust (ver kits/nix/rustTool.nix).
      mkRustTool =
        system:
        import ./kits/nix/rustTool.nix {
          pkgs = nixpkgs.legacyPackages.${system};
          root = ./.;
          inherit supportedSystems;
        };

      # Instalador, lanzador de kmscon y paleta (ver installer/default.nix).
      installerPackages = forAllSystems (
        { system, pkgs }:
        import ./installer {
          inherit pkgs theme tablesEnv;
          mkRustTool = mkRustTool system;
        }
      );

      # El asistente de primer arranque: solo orquesta mkpasswd, git, nix
      # y nixos-rebuild.
      setups = forAllSystems (
        { system, pkgs }:
        mkRustTool system {
          pname = "nixdots-setup";
          subdir = "tools/setup";
          runtimeInputs = with pkgs; [
            git
            nix
            nixos-rebuild
            mkpasswd
            coreutils
          ];
          description = "First-boot assistant CLI for the Keel configuration";
          # Tablas que `localekit` embebe para validar locale y zona.
          extra = tablesEnv pkgs;
        }
      );

      # Asistente gráfico (Slint): llama al CLI nixdots-setup.
      #
      # Licencia: Slint es GPL-3.0-only (o comercial/royalty-free). El
      # flake declara gpl2Only y GPLv2-only es incompatible con GPLv3.
      # Ver tools/setup-gui/Cargo.toml.
      setupsGui = forAllSystems (
        { system, pkgs }:
        mkRustTool system {
          pname = "nixdots-setup-gui";
          subdir = "tools/setup-gui";
          description = "Graphical first-boot assistant for the Keel configuration (Slint)";
          runtimeInputs = [ setups.${system} ];
          # Tablas que `localekit` embebe (locales) para la lista
          # de idiomas del asistente.
          extra = (tablesEnv pkgs) // {
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.makeWrapper
            ];
            buildInputs = with pkgs; [
              fontconfig
              freetype
              libxkbcommon
              libglvnd
              wayland
              libx11
            ];
          };
        }
      );

      # Servicio D-Bus de gestión de cuentas y preparación del home.
      userd = forAllSystems (
        { system, ... }:
        mkRustTool system {
          pname = "nixdots-userd";
          subdir = "tools/nixdots-userd";
          description = "D-Bus user management service for Keel";
        }
      );

      homeInit = forAllSystems (
        { system, pkgs }:
        mkRustTool system {
          pname = "nixdots-home-init";
          subdir = "tools/nixdots-home-init";
          description = "First-login home setup for Keel";
          runtimeInputs = [
            pkgs.home-manager
            pkgs.systemd
            pkgs.git
            pkgs.coreutils
          ];
        }
      );

      # Perfil único: hardware detectado por facter + machine.json. La
      # forma impura resuelve la arquitectura con builtins.currentSystem.
      keel = mkGeneric {
        system = if data.impure then builtins.currentSystem else "x86_64-linux";
        requireImpure = true;
        inherit data;
      };
    in
    {
      nixosConfigurations = {
        inherit keel;

        # Variantes puras para CI (`nix flake check` / `nix build`
        # sin --impure): sin user.json/machine.json/facter.json, no
        # arrancan, solo comprueban que el árbol evalúa en ambas
        # arquitecturas.
        keel-x86_64-linux = mkGeneric {
          system = "x86_64-linux";
          requireImpure = false;
          data = emptyData;
        };
        keel-aarch64-linux = mkGeneric {
          system = "aarch64-linux";
          requireImpure = false;
          data = emptyData;
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
      # `nixos-rebuild --flake ~/dotfiles` sin `#…` también funcione.
      // nixpkgs.lib.optionalAttrs (data.machine ? hostName) {
        ${data.machine.hostName} = keel;
      };

      # Overlay que el flake `home` importa para tener brave-origin-beta
      # (el sistema ya lo aplica en core/nix.nix).
      overlays.brave = inputs.brave-previews.overlays.default;

      # Módulos de home-manager reutilizables: `deck` (el home completo
      # de este repo) y `minimal` (fish, starship, git y el tema). El
      # template `home` genera un flake de usuario que los importa.
      homeModules = import ./users/templates;

      templates = {
        home = {
          path = ./templates/home;
          description = "Home independiente de Keel (home-manager)";
        };
      };

      # install.sh lee la copia versionada de la paleta sin pasar por
      # Nix; este check falla si dejó de coincidir con lo que genera
      # installer/nix/kmscon-palette.nix.
      checks = forAllSystems (
        { pkgs, ... }:
        let
          palette = pkgs.callPackage ./installer/nix/kmscon-palette.nix { inherit theme; };
        in
        {
          # Las copias versionadas de generated/ deben coincidir con
          # lo que generan default.nix e installer/nix/kmscon-palette.nix.
          neptunia-generated = pkgs.runCommand "check-neptunia-generated" { } ''
            ok=1
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/neptunia.slint} ${pkgs.writeText "g.slint" theme.slint} || ok=0
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/neptunia.css} ${pkgs.writeText "g.css" theme.css} || ok=0
            ${pkgs.diffutils}/bin/diff -u ${./kits/neptunia/generated/kmscon-palette} ${palette} || ok=0
            if [ "$ok" = 0 ]; then
                echo "" >&2
                echo "kits/neptunia/generated está desactualizado. Regéneralo con:" >&2
                echo "  nix eval --raw -f kits/neptunia slint > kits/neptunia/generated/neptunia.slint" >&2
                echo "  nix eval --raw -f kits/neptunia css   > kits/neptunia/generated/neptunia.css" >&2
                echo "  cp \$(nix build .#kmscon-palette --print-out-paths) kits/neptunia/generated/kmscon-palette" >&2
                exit 1
            fi
            ${pkgs.coreutils}/bin/touch $out
          '';
        }
      );

      packages = forAllSystems (
        { system, pkgs }: {
          installer = installerPackages.${system}.installer;
          setup = setups.${system};
          assist-setup = setupsGui.${system};
          crewd = userd.${system};
          home-init = homeInit.${system};
          launcher = installerPackages.${system}.launcher;
          default = installerPackages.${system}.launcher;

          # The kmscon `--palette*` arguments, generated from the theme.
          # `install.sh` reads the committed copy at
          # kits/neptunia/generated/kmscon-palette; regenerate it with
          # `nix build .#kmscon-palette --print-out-paths`.
          kmscon-palette = installerPackages.${system}.kmscon-palette;

          # Safe wrapper around nixos-rebuild (packages/nixdots-rebuild.*).
          nixdots-rebuild = pkgs.callPackage ./packages/nixdots-rebuild.nix { };

          # The macOS XNU console font, as PSF2 (kernel and kmscon).
          xnu-console-font = pkgs.callPackage ./packages/xnu-console-font.nix { };
        }
      );

      apps = forAllSystems (
        { system, ... }: {
          installer = {
            type = "app";
            program = "${installerPackages.${system}.launcher}/bin/nixdots-installer";
          };
          default = {
            type = "app";
            program = "${installerPackages.${system}.launcher}/bin/nixdots-installer";
          };
        }
      );

      # Development shell for the live WezTerm window: cargo + rehearse
      # on every save with watchexec.
      # `nix fmt`: el formateador oficial (RFC 166).
      formatter = forAllSystems ({ pkgs, ... }: pkgs.nixfmt);

      # Un único devShell para todo el repo (ver kits/nix/devShells.nix).
      devShells = forAllSystems (
        { system, pkgs, ... }:
        import ./kits/nix/devShells.nix {
          inherit pkgs tablesEnv;
          setup = setups.${system};
        }
      );
    };
}
