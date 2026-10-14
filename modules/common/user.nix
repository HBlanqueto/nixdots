{ pkgs, inputs, hostName, username, userdescription, hashedPasswordFile, timeZone, defaultLocale, ... }:

let
    theme = import ../../share/themes;
in

{
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
        colors = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ] ++ [ lbg c9 c10 c11 c12 c13 c14 c15 ];
        font = "Lat2-Terminus16";
        keyMap = "es";
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
