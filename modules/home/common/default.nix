{ config, pkgs, ... }:

{
    imports = [
        ./apps.nix
        ./style.nix
    ];

    home = {
        sessionVariables = {
            # Brave is x86_64-only (the brave-previews input publishes no
            # aarch64 output); fall back to the always-present Firefox on ARM.
            BROWSER =
                if pkgs.stdenv.hostPlatform.isx86_64
                then "${config.programs.brave.package}/bin/brave-origin-beta"
                else "${pkgs.firefox}/bin/firefox";
            TERMINAL = "wezterm";
            EDITOR = "nvim";
            NO_AT_BRIDGE = "1";
            NIXOS_OZONE_WL = "1";
        };

        file = {
            ".config/chime/chime.wav".source = ../../../share/chime.wav;
        };
    };

    xdg = {
        enable = true;

        userDirs = {
            enable = true;
            createDirectories = true;

            projects = "${config.home.homeDirectory}/Proyectos";
            publicShare = "${config.home.homeDirectory}/Público";
            templates = "${config.home.homeDirectory}/Plantillas";
            desktop = "${config.home.homeDirectory}/Escritorio";
            documents = "${config.home.homeDirectory}/Documentos";
            music = "${config.home.homeDirectory}/Música";
            pictures = "${config.home.homeDirectory}/Imágenes";
            videos = "${config.home.homeDirectory}/Videos";
            download = "${config.home.homeDirectory}/Descargas";
        };
    };
}
