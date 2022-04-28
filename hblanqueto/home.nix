 config, pkgs, ... }:

{
  home = {
    username = "Humberto Blanqueto";
    homeDirectory = "/home/hblanqueto";

    sessionVariables = {
      #NIXOS_OZONE_WL="1";
    };

    packages = with pkgs; [

      #wpsoffice
      #libreoffice-fresh-unwrapped
      
      tdesktop
      #element-desktop
      obs-studio
      gnome.gnome-tweaks
      #gnome-builder

      neofetch
      
      #neovide
      
      gnumake
      #gcc
      clang
      llvm
      
      #rustc
      #cargo
      #python39
      #lua5_4
      #luarocks
      
      xdg-desktop-portal-wlr
      xdg_utils
      qt5.qtwayland

    ];

    stateVersion = "22.05";
  };

  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  programs = {
    home-manager.enable = true;
    
    google-chrome = {
      enable = true;
      package = pkgs.google-chrome;
      commandLineArgs = [ 
      "--enable-features=UseOzonePlatform"
      "--ozone-platform=wayland --ignore-gpu-blocklist" 
      "--enable-gpu-rasterization --enable-zero-copy" 
      "--disable-gpu-driver-bug-workarounds"
      "--enable-accelerated-video-decode"
      "--enable-features=VaapiVideoDecoder"
      "--use-gl=egl" ];
    };
    
    vscode = {
      enable = true;
      package = pkgs.vscode;
    };

    git = {
      enable = true;
      userName = "Mr. HBlanqueto";
      userEmail = "humbertoblanqueto@outlook.com";
    };
  };

}