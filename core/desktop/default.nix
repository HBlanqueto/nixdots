{ pkgs, ... }:

{
    imports = [
        ./gnome.nix
    ];

    # GL/VA-API userspace, used by GNOME's Wayland session on both hosts;
    # host-specific drivers are added in core/hardware-detect.nix.
    hardware.graphics.enable = true;

    environment.systemPackages = with pkgs; [
        libnotify
        playerctl
        wl-clipboard

        desktop-file-utils
        wayland-utils
        libinput
    ];
}
