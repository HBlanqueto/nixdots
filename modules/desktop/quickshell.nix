{ pkgs, ... }:

{
    # Video backdrop for the Quickshell player: yt-dlp resolves YouTube
    # streams, ffmpeg-full is what QtMultimedia's ffmpeg backend links,
    # libva-utils ships vainfo.
    environment.systemPackages = with pkgs; [
        yt-dlp
        ffmpeg-full
        libva-utils
    ];

    environment.sessionVariables = {
        QT_MEDIA_BACKEND = "ffmpeg";
        # LIBVA_DRIVER_NAME is host-specific (radeonsi on the ThinkPad, unset on
        # the VM where there is no VA-API driver) and is defined per host.
    };

    # QtMultimedia is a QML module (QtQuick: import QtMultimedia), so the
    # quickshell package must see it at build time: wrapQtAppsHook derives
    # QML2_IMPORT_PATH from buildInputs. Overridden at the nixpkgs package
    # level (quickshell comes from nixpkgs, not from home-manager or an
    # overlay) so the one copy modules/desktop/default.nix installs gets it.
    nixpkgs.overlays = [
        (final: prev: {
            quickshell = prev.quickshell.overrideAttrs (old: {
                buildInputs = (old.buildInputs or [ ]) ++ [ final.qt6.qtmultimedia ];
            });
        })
    ];
}
