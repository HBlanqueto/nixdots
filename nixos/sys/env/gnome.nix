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
    MOZ_DISABLE_RDD_SANDBOX="1";
    QT_WAYLAND_DISABLE_WINDOWDECORATION= "1";
    QT_QPA_PLATFORM="wayland";
    SDL_VIDEODRIVER= "wayland";
  }; 

  nixpkgs.config.chromium.commandLineArgs = "--enable-features=UseOzonePlatform --ozone-platform=wayland";

}

}