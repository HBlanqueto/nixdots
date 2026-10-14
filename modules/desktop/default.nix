{ pkgs, ... }:

{
    imports = [
        ./gnome.nix
    ];

    # GL/VA-API userspace, used by GNOME's Wayland session on both hosts;
    # host-specific drivers are added in modules/hardware/*.nix.
    hardware.graphics.enable = true;

    environment.systemPackages = with pkgs; [
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
