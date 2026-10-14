{ pkgs, ... }:

{
    imports = [
        ./gnome.nix
        ./somewm.nix
        ./quickshell.nix
    ];

    # GL/VA-API userspace. Required by somewm (its SceneFX integration needs
    # GLES2 at runtime) on both hosts; host-specific drivers are added in
    # modules/hardware/*.nix.
    hardware.graphics.enable = true;

    environment.systemPackages = with pkgs; [
        quickshell
        somewm
        gsettings-desktop-schemas
        libnotify
        wireplumber
        playerctl
        wev
        grim
        slurp
        wl-clipboard

        xwayland
        desktop-file-utils
        wayland-utils
        libinput
        drm_info
        vulkan-tools
        mesa-demos
    ];
}
