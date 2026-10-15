{ config, pkgs, inputs, hostName, username, userdescription, hashedPasswordFile, timeZone, defaultLocale, keyboardLayout, keyboardVariant, ... }:

let
    theme = import ../../share/themes;

    # The installed system checks the final path; the installer evaluates from
    # the ISO, so it points NIXDOTS_ROOT at the target (e.g. "/mnt"), where the
    # file lives. On a normal system the variable is unset ("").
    root = builtins.getEnv "NIXDOTS_ROOT";

    # Every declared normal user must point at an existing password hash file.
    # With mutableUsers = false a missing file leaves the account without a
    # password on activation. Evaluated with --impure so the real path can be
    # inspected.
    normalUsers = builtins.filter (u: u.isNormalUser) (builtins.attrValues config.users.users);
    missingHashes = builtins.filter
        (u: (u.hashedPasswordFile or null) != null && !(builtins.pathExists "${root}${u.hashedPasswordFile}"))
        normalUsers;
in

{
    assertions = [
        {
            assertion = missingHashes == [ ];
            message =
                "Falta el archivo de contraseña de: "
                + builtins.concatStringsSep ", "
                    (map (u: "${u.name} -> ${root}${toString u.hashedPasswordFile}") missingHashes)
                + ". Créalo con `mkpasswd -m yescrypt` antes de reconstruir.";
        }
    ];

    time = {
        timeZone = timeZone;
    };

    i18n = {
        defaultLocale = defaultLocale;
    };

    networking = {
        hostName = hostName;
    };

    users = {
        defaultUserShell = pkgs.fish;
        mutableUsers = false;
        users.${username} = {
            isNormalUser = true;
            description = userdescription;
            extraGroups = [ "networkmanager" "wheel" "video" "seat" ];
            createHome = true;
            hashedPasswordFile = hashedPasswordFile;
        };
    };

    console = {
        earlySetup = true;
        colors = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ] ++ [ lbg c9 c10 c11 c12 c13 c14 c15 ];
        font = "Lat2-Terminus16";
        # The console uses the same xkb layout as the desktop.
        useXkbConfig = true;
    };

    services.xserver.xkb = {
        layout = keyboardLayout;
        variant = keyboardVariant;
    };

    programs = {
        fish = {
            enable = true;
        };
    };

    environment = {
        binsh = "${pkgs.dash}/bin/dash";

        # Cross-platform CLI tools only. Desktop/GPU tools live in
        # modules/desktop, laptop-only tools in modules/hardware/thinkpad.nix.
        systemPackages = with pkgs; [
            comma

            wget
            curl
            luajit
            lua51Packages.luacheck
            poppler-utils

            neovim
            unzip
            tree

            fastfetch
            eza
            mpc
            ffmpeg
            jq
            gh
            python3
        ];
    };
}
