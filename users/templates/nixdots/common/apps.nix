{ pkgs, lib, inputs, userData, osConfig, theme, ... }:

let
    git = userData.git or null;

    yaziConfig = import ./config/yazi { inherit theme; };

    # rgba() a partir del rgb de la paleta (misma fuente que la consola).
    palette = import ../../../../lib/palette.nix { inherit theme; };
    rgba = hex: alpha: "rgba(${palette.rgb hex},${alpha})";

    # starship sin iconos (preset oficial `plain-text-symbols`) para la TTY, cuya
    # fuente XNU solo tiene los 256 glifos Latin-1.
    starshipPlain = pkgs.runCommand "starship-plain-text-symbols.toml" { } ''
        ${pkgs.starship}/bin/starship preset plain-text-symbols -o $out
    '';

    vscodeSettings = {
        "editor.fontFamily" = "'Liga SFMono Nerd Font', 'Twitter Color Emoji', monospace";
        "editor.fontLigatures" = true;
        "editor.fontSize" = 12;
        "editor.lineHeight" = 18;
        "editor.wordWrap" = "on";
        "workbench.editor.tabSizing" = "shrink";
        "workbench.editor.showTabs" = "multiple";
        "editor.centeredLayout" = false;
        "editor.stickyScroll.enabled" = false;

        "terminal.integrated.fontFamily" = "'Liga SFMono Nerd Font'";
        "terminal.integrated.fontSize" = 12;
        "workbench.colorTheme" = "Neptunia";
        "workbench.settings.applyToAllProfiles" = [ ];

        "explorer.confirmDelete" = false;
        "explorer.confirmDragAndDrop" = false;

        "files.exclude" = {
            "**/.git" = false;
            "**/.svn" = false;
            "**/.hg" = false;
            "**/.DS_Store" = false;
            "**/node_modules" = true;
            "**/dist" = true;
        };

        "git.autofetch" = true;
        "git.confirmSync" = false;

        "indentRainbow.indicatorStyle" = "light";
        "indentRainbow.lightIndicatorStyleLineWidth" = 2;
        "indentRainbow.colors" = with theme.colors; [
            (rgba c11 "0.3")
            (rgba c10 "0.3")
            (rgba c13 "0.3")
            (rgba c14 "0.3")
        ];

        "continue.showInlineTip" = false;
        "claudeCode.preferredLocation" = "panel";
        "workbench.iconTheme" = "file-icons";
    };
in

{
    xdg = {
        configFile = {
            "eza/theme.yml".source = (pkgs.formats.yaml { }).generate "eza-theme.yml" {
                filekinds = {
                    directory = {
                        foreground = theme.filecolors.directory;
                        is_bold = true;
                    };
                    executable = {
                        foreground = theme.filecolors.executable;
                        is_bold = true;
                    };
                    symlink = {
                        foreground = theme.filecolors.symlink;
                    };
                };
                links = {
                    normal = {
                        foreground = theme.filecolors.symlink;
                    };
                    multi_link = {
                        foreground = theme.filecolors.symlink;
                    };
                    broken = {
                        foreground = theme.filecolors.orphan;
                    };
                };
                broken_symlink = {
                    foreground = theme.filecolors.orphan;
                };
            };
        };
    };

    home = {
        stateVersion = osConfig.system.stateVersion;

        file = {
            ".config/Code/User/settings.json".text = builtins.toJSON vscodeSettings;

            ".vscode/extensions/neptunia-theme/package.json".text = theme.vscode.packageJson;
            ".vscode/extensions/neptunia-theme/themes/neptunia-color-theme.json".text = theme.vscode.themeJson;

            ".config/fastfetch/logo.txt".source = ./config/fastfetch/logo.txt;
            ".config/fastfetch/config.jsonc".source = ./config/fastfetch/config.jsonc;

            "Pictures/Screenshots/.gitkeep".text = "";
        };

        activation = {
            registerNeptuniaExtension = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
                ${pkgs.python3}/bin/python3 - <<'PYEOF'
                import json
                import os

                dir = os.path.expanduser("~/.vscode/extensions/neptunia-theme")
                path = os.path.expanduser("~/.vscode/extensions/extensions.json")

                if not os.path.exists(os.path.join(dir, "package.json")):
                    raise SystemExit

                try:
                    with open(path) as f:
                        data = json.load(f)
                except (OSError, ValueError):
                    data = []

                entry = {
                    "identifier": {"id": "nix.neptunia-theme"},
                    "version": "1.0.0",
                    "location": {"$mid": 1, "path": os.path.abspath(dir), "scheme": "file"},
                    "relativeLocation": "neptunia-theme",
                    "metadata": {"source": "gallery", "installedTimestamp": 1786954366021},
                }

                if not any(e.get("identifier", {}).get("id") == "nix.neptunia-theme" for e in data):
                    data.append(entry)
                    with open(path + ".nixbak", "w") as f:
                        json.dump(data, f, indent=2)
                        f.write("\n")
                    os.replace(path + ".nixbak", path)
                PYEOF
            '';
        };

        packages = with pkgs; [
            # Herramientas de terminal que antes vivían en systemPackages.
            fastfetch
            eza
            luajit
            lua51Packages.luacheck
            mpc
            ffmpeg
            poppler-utils
            gh
            python3

            cava
            firefox
            trezor-suite
            vscode
            nautilus

            karere
            telegram-desktop
            bitwarden-desktop

            imagemagick
            quickemu

            # Safe nixos-rebuild wrapper (flake package); also the backend of
            # the flake-update-* aliases below.
            inputs.self.packages.${pkgs.stdenv.hostPlatform.system}.nixdots-rebuild
        ] ++ lib.optionals pkgs.stdenv.hostPlatform.isx86_64 [
            # OnlyOffice ships no aarch64 build in nixpkgs.
            onlyoffice-desktopeditors
        ] ++ lib.optional (userData.a11y.screenReader or false) pkgs.orca;
    };

    programs = {
        # Brave is only published for x86_64-linux by the brave-previews input;
        # the aarch64 VM uses the shared Firefox above instead.
        brave = lib.mkIf pkgs.stdenv.hostPlatform.isx86_64 {
            enable = true;
            # The drishal brave-origin-beta wrapper drops the ozone hint (its
            # nested ${NIXOS_OZONE_WL:+${WAYLAND_DISPLAY:+...}} expansion is
            # stripped by makeShellWrapper), so Brave would fall back to
            # XWayland. Force native Wayland here.
            package = pkgs.brave-origin-beta.overrideAttrs (old: {
                preFixup = (old.preFixup or "") + "\ngappsWrapperArgs+=(\"--add-flags\" \"--ozone-platform-hint=auto\")\n";
            });
        };

        # Solo se configura git si el asistente recogió nombre y correo.
        git = lib.mkIf (git != null) {
            enable = true;
            settings = {
                user = {
                    name = git.name;
                    email = git.email;
                };
            };
        };

        ssh = {
            enable = true;
            enableDefaultConfig = false;
            settings = {
                "*" = {
                    ForwardAgent = false;
                    AddKeysToAgent = "no";
                    Compression = false;
                    ServerAliveInterval = 0;
                    ServerAliveCountMax = 3;
                    HashKnownHosts = false;
                    UserKnownHostsFile = "~/.ssh/known_hosts";
                    ControlMaster = "no";
                    ControlPath = "~/.ssh/master-%r@%n:%p";
                    ControlPersist = "no";
                };
                "github.com" = {
                    User = "git";
                    IdentityFile = "~/.ssh/id_ed25519_github";
                    IdentitiesOnly = true;
                };
            };
        };

        yazi = {
            enable = true;
            enableFishIntegration = true;
            plugins = {
                yatline = pkgs.yaziPlugins.yatline.overrideAttrs (o: {
                    patches = (o.patches or [ ]) ++ [ ./config/yazi/yatline-icon.patch ];
                });

                full-border = pkgs.yaziPlugins.full-border;
            };
            initLua = builtins.readFile ./config/yazi/init.lua;
            
            inherit (yaziConfig) settings theme;
        };

        # mpv exposes MPRIS (and mpris:artUrl from embedded/folder art) only
        # through the mpv-mpris plugin.
        mpv = {
            enable = true;
            scripts = [ pkgs.mpvScripts.mpris ];
        };

        fish = {
            enable = true;
            interactiveShellInit = ''
                set -g fish_greeting
                set -g fish_color_command --bold green

                # La TTY del sistema instalado usa la XNU (solo Latin-1): ni el
                # prompt ni eza llevan iconos Nerd Font. En la terminal gráfica
                # todo sigue igual.
                if test "$XDG_SESSION_TYPE" = tty
                    set -gx STARSHIP_CONFIG ${starshipPlain}
                    set -g eza_icons
                else
                    set -g eza_icons --icons
                end
                alias ls "eza --color=auto $eza_icons"
                alias l "ls -l"
                alias la "ls -a"
                alias lla "ls -la"
                alias lt "ls --tree"
            '';
            shellAliases = {
                delgen = "sudo nix-collect-garbage --delete-older-than 1d && sudo nix-store --gc && sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old";
                nix-update = "nixdots-rebuild sw";
                flake-update-rb = "nix flake update --flake $HOME/nixdots && nixdots-rebuild rb";
                flake-update-sw = "nix flake update --flake $HOME/nixdots && nixdots-rebuild sw";

                g = "git";
                c = "clear";
            };
        };

        starship = {
            enable = true;
        };
    };
}