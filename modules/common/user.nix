{ config, lib, pkgs, data, theme, ... }:

let
    palette = import ../../lib/palette.nix { inherit theme; };

    # The installed system checks the final path; the installer evaluates from
    # the ISO, so lib/data.nix points NIXDOTS_ROOT at the target (e.g. "/mnt"),
    # where the file lives. On a normal system the variable is unset ("").
    inherit (data) root;

    # Todo lo personal sale de data.settings (settings.nix + user.json).
    inherit (data.settings) username userdescription timeZone defaultLocale keyboardLayout keyboardVariant;

    # Derivado del usuario: no se guarda en ningún archivo.
    hashedPasswordFile =
        if username == null then null else "/persist/secrets/${username}.hash";

    # Every declared normal user must point at an existing password hash file.
    # With mutableUsers = false a missing file leaves the account without a
    # password on activation. Evaluated with --impure so the real path can be
    # inspected.
    normalUsers = builtins.filter (u: u.isNormalUser) (builtins.attrValues config.users.users);
    missingHashes = builtins.filter
        (u: (u.hashedPasswordFile or null) != null && !(data.exists u.hashedPasswordFile))
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
        inherit timeZone;
    };

    i18n = {
        inherit defaultLocale;
    };

    networking = {
        hostName = data.hostName;
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
            extraGroups = [ "networkmanager" "wheel" "video" ];
            createHome = true;
            inherit hashedPasswordFile;
        };
    };

    console = {
        earlySetup = true;
        # Los 16 colores ANSI (con el fondo negro puro) salen de lib/palette.nix.
        colors = palette.consoleColors;
        # "xnu" usa la fuente bitmap que incrusta xnu (el kernel de macOS) en
        # initrd, consola de emergencia y arranque de Plymouth. La VM de primer
        # arranque no importa console.nix y cae a "sfmono".
        font =
            if (config.nixdots.console.font or "sfmono") == "xnu"
            then "${pkgs.xnu-console-font}/share/consolefonts/xnu-console.psf"
            else "Lat2-Terminus16";
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
