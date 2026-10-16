{ config, lib, pkgs, inputs, hostName, username, userdescription, hashedPasswordFile, timeZone, timeZoneAuto, defaultLocale, region, keyboardLayout, keyboardVariant, a11y, ... }:

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
            # Pure evaluation (nix flake check, the ISO's own build) has no
            # getEnv and no filesystem access, so skip the check there.
            assertion = !(builtins ? currentSystem) || missingHashes == [ ];
            message =
                "Falta el archivo de contraseña de: "
                + builtins.concatStringsSep ", "
                    (map (u: "${u.name} -> ${root}${toString u.hashedPasswordFile}") missingHashes)
                + ". Créalo con `mkpasswd -m yescrypt` antes de reconstruir.";
        }
    ];

    time = {
        # Con la zona automática, timeZone queda sin definir (lo pone geoclue).
        timeZone = lib.mkIf (!timeZoneAuto) timeZone;
    };

    services.automatic-timezoned.enable = timeZoneAuto;

    i18n = {
        defaultLocale = defaultLocale;
        # Región y formatos sólo si difiere del idioma base.
        extraLocaleSettings = lib.mkIf (region != null && region != defaultLocale) {
            LC_TIME = region;
            LC_MONETARY = region;
            LC_NUMERIC = region;
            LC_MEASUREMENT = region;
            LC_PAPER = region;
            LC_ADDRESS = region;
            LC_TELEPHONE = region;
        };
    };

    # dconf para los ajustes de accesibilidad de GNOME (home-manager los escribe).
    programs.dconf.enable = true;

    networking = {
        hostName = hostName;
    };

    users = {
        defaultUserShell = pkgs.fish;
        mutableUsers = false;
    } // lib.optionalAttrs (username != null) {
        users.${username} = {
            isNormalUser = true;
            # Fijo para que el asistente (que corre antes de existir la cuenta)
            # pueda hacer el chown del repo a 1000:users. Ver setup/src/main.rs.
            uid = 1000;
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
        ] ++ lib.optional (a11y.screenReader or false) pkgs.orca;
    };
}
