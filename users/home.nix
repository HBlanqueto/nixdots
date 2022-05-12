{ config, pkgs, ... }:

{

  home = {
    username = "Humberto Blanqueto";
    homeDirectory = "/home/hblanqueto";

  file = {

    # Wezterm
    ".config/wezterm/wezterm.lua".text = import ./programs/wezterm.nix { }; 

    # Gnome terminal padding
    ".config/gtk-3.0/gtk.css".text = import ./programs/gtk.nix { };
  };

    sessionVariables = {
      MOZ_DISABLE_RDD_SANDBOX="1";
      NO_AT_BRIDGE = "1";
  };

  packages = with pkgs; [
      
      google-chrome
      tdesktop
      obs-studio
      #vlc
      #wpsoffice
      #gnome.gnome-tweaks
      polkit_gnome
      vscode
      #gnome.gedit
      #neovide

      neofetch
      htop      
      
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
      xdg-desktop-portal-gtk
      xdg-utils
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

    bat = {
      enable = true;
      config = {
        pager = "never";
      };
    };

    exa = {
      enable = true;
      enableAliases = false;
    };


    git = {
      enable = true;
      userName = "Mr. HBlanqueto";
      userEmail = "humbertoblanqueto@outlook.com";
    };

    starship = {
      enable = true;
      settings = import ./programs/starship.nix;
    };

    zsh = {
      enable = true;
      autocd = true;
      enableAutosuggestions = true;
      enableCompletion = true;
      dotDir = ".config/zsh";

      history = {
        expireDuplicatesFirst = true;
        extended = true;
        save = 25000;
      };

      shellAliases = {
        ls = "exa --color=auto --icons";
        l = "ls -l";
        la = "ls -a";
        lla = "ls -la";
        lt = "ls --tree";
        cat = "bat";
        editor = "nvim";
        grep = "grep --color=auto";
        upgrade = "sudo nixos-rebuild switch --upgrade && home-manager switch";
      };
    };
  };

}