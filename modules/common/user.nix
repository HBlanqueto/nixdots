{ config, lib, pkgs, data, theme, ... }:

let
    palette = import ../../lib/palette.nix { inherit theme; };

    # The installed system checks the final path; the installer evaluates from
    # the ISO, so lib/data.nix points NIXDOTS_ROOT at the target (e.g. "/mnt"),
    # where the file lives. On a normal system the variable is unset ("").
    inherit (data) root;

    # Todo lo personal sale de data.settings (settings.nix + user.json). En las
    # variantes de CI no hay user.json, así que los campos personales caen a null.
    inherit (data.settings) timeZone defaultLocale keyboardLayout keyboardVariant;
    username = data.settings.username or null;
    userdescription = data.settings.userdescription or null;
    # Región y formatos, zona horaria automática y accesibilidad.
    region = data.settings.region or null;
    timeZoneAuto = data.settings.timeZoneAuto or false;
    autoLogin = data.settings.autoLogin or false;
    a11y = data.settings.a11y or { };

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
        # Con la zona automática, timeZone queda sin definir (lo pone geoclue).
        timeZone = lib.mkIf (!timeZoneAuto) timeZone;
    };

    services.automatic-timezoned.enable = timeZoneAuto;

    i18n = {
        inherit defaultLocale;
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

    # Entrada automática al escritorio si el asistente la pidió.
    services.displayManager.autoLogin = lib.mkIf (autoLogin && username != null) {
        enable = true;
        user = username;
    };

    networking = {
        inherit (data) hostName;
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
        # La fuente bitmap que incrusta xnu (el kernel de macOS) en initrd, la
        # consola de emergencia y el arranque de Plymouth.
        font = "${pkgs.xnu-console-font}/share/consolefonts/xnu-console.psf";
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
        # modules/desktop, laptop-only tools in modules/common/hardware-detect.nix.
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
