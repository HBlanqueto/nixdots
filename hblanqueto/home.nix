{ config, pkgs, ... }:

{
  home = {
    username = "Humberto Blanqueto";
    homeDirectory = "/home/hblanqueto";

    sessionVariables = {
      MOZ_DISABLE_RDD_SANDBOX="1";
    };

    packages = with pkgs; [
      
      google-chrome
      tdesktop
      obs-studio
      #wpsoffice
      gnome.gnome-tweaks
      polkit_gnome
      vscode
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

    bat = {
      enable = true;
      config = {
        theme = "ansi";
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
        cat = "bat --color always --plain";
        grep = "grep --color=auto";
        upgrade = "sudo nixos-rebuild switch --upgrade && home-manager switch";
      };
    };
  };

}