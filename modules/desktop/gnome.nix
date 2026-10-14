{ pkgs, ... }:

{
    services = {
        desktopManager = {
            gnome = {
                enable = true;
            };
        };

        gnome = {
            core-apps.enable = false;
            core-developer-tools.enable = false;
            games.enable = false;
        };

        xserver = {
            enable = true;
            excludePackages = [ pkgs.xterm ];

            xkb = {
                layout = "es";
                variant = "";
            };
        };

        displayManager = {
            gdm.enable = true;
        };
    };

    environment.gnome.excludePackages = with pkgs; [
        gnome-tour
    ];
}
