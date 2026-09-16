{ pkgs, lib, inputs, hostName, gitName, gitEmail, stateVersion, theme, ... }:

let
    yaziConfig = import ./config/yazi;
    starshipSettings = import ./config/starship.nix { inherit (theme) colors; };

    hexToRgb = hex:
        let
            digit = {
                "0" = 0; "1" = 1; "2" = 2; "3" = 3; "4" = 4;
                "5" = 5; "6" = 6; "7" = 7; "8" = 8; "9" = 9;
                "a" = 10; "b" = 11; "c" = 12; "d" = 13; "e" = 14; "f" = 15;
            };
            value = s: lib.foldl' (acc: ch: acc * 16 + digit.${ch}) 0 (lib.stringToCharacters s);
        in
        map value [
            (lib.substring 0 2 hex)
            (lib.substring 2 2 hex)
            (lib.substring 4 2 hex)
        ];

    rgba = hex: alpha:
        "rgba(${lib.concatStringsSep "," ((map toString (hexToRgb hex)) ++ [ alpha ])})";

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
        stateVersion = stateVersion;

        file = {
            ".config/Code/User/settings.json".text = builtins.toJSON vscodeSettings;

            ".vscode/extensions/neptunia-theme/package.json".text = theme.vscode.packageJson;
            ".vscode/extensions/neptunia-theme/themes/neptunia-color-theme.json".text = theme.vscode.themeJson;

            ".config/fastfetch/logo.txt".source = ./config/fastfetch/logo.txt;
            ".config/fastfetch/config.jsonc".source = ./config/fastfetch/config.jsonc;
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
            fastfetch
            eza
            mpc
            ffmpeg
            # firefox is provided (wrapped with fx-autoconfig) by ../firefox-islands
            jq
            gh
            python3
        
            trezor-suite
            vscode
            nautilus

            karere
            telegram-desktop
            bitwarden-desktop

            onlyoffice-desktopeditors
            foot
        ];
    };

    programs = {
        brave = {
            enable = true;
            package = inputs.brave-previews.packages.${pkgs.stdenv.hostPlatform.system}.brave-origin-beta;
        };

        git = {
            enable = true;
            settings = {
                user = {
                    name = gitName;
                    email = gitEmail;
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
            
            settings = yaziConfig.settings;
            theme = yaziConfig.theme;
        };

        ncmpcpp = {
            enable = true;
            package = pkgs.ncmpcpp.override { 
                visualizerSupport = true; 
            };
            
            settings = import ./config/ncmpcpp.nix;
        };

        wezterm = {
            enable = true;
            extraConfig = builtins.readFile ./config/wezterm.lua;
        };

        fish = {
            enable = true;
            interactiveShellInit = ''
                set -g fish_greeting 
                set -g fish_color_command --bold green
            '';
            shellAliases = {
                delgen = "sudo nix-collect-garbage --delete-older-than 1d && sudo nix-store --gc && sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old";
                nix-update = "sudo nixos-rebuild switch";
                flake-update-rb = "sudo nixos-rebuild boot --flake .#${hostName} --impure";
                flake-update-sw = "sudo nixos-rebuild switch --flake .#${hostName} --impure";

                g = "git";
                c = "clear";
                ls = "eza --color=auto --icons";
                l = "ls -l";
                la = "ls -a";
                lla = "ls -la";
                lt = "ls --tree";
            };
        };

        starship = {
            enable = true;
            settings = starshipSettings;
        };
    };

    services = {
        mopidy = {
            enable = true;
            extensionPackages = with pkgs; [
                mopidy-local
                mopidy-mpd
            ];
            settings = import ./config/mopidy.nix;
        };
    };
}