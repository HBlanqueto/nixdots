{ config, pkgs, ... }:

{
  services = {
    gnome.core-utilities.enable = false;
    xserver = {
      desktopManager = {
        gnome = {
          enable = true;
        };
      };
    };
  };

  environment.sessionVariables = {
    MOZ_ENABLE_WAYLAND = "1";
    QT_QPA_PLATFORM="wayland";
    SDL_VIDEODRIVER= "wayland";
    CLUTTER_BACKEND = "wayland";
    #QT_WAYLAND_DISABLE_WINDOWDECORATION= "1";
    #NIXOS_OZONE_WL="1";
  }; 

}
