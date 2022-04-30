{ config, pkgs, ... }:

{

  home = {
    username = "Humberto Blanqueto";
    homeDirectory = "/home/hblanqueto";

  file = {

    # Wezterm
    ".config/wezterm/wezterm.lua".text = import ./programs/wezterm { }; # Main file 
    ".config/wezterm/mytable.lua".text = import ./programs/wezterm/mytable.nix { };

    ".config/wezterm/modules/tabs.lua".text = import ./programs/wezterm/tabs.nix { };
    ".config/wezterm/modules/keys.lua".text = import ./programs/wezterm/keys.nix { }; # Keybindings
    ".config/wezterm/modules/init.lua".text = import ./programs/wezterm/init-mod.nix { };
    
    ".config/wezterm/modules/theme/init.lua".text = import ./programs/wezterm/init-col.nix { };
    ".config/wezterm/modules/theme/colors.lua".text = import ./programs/wezterm/colors.nix { }; # Colors

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
      vlc
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
        cat = "bat";
        editor = "nvim";
        grep = "grep --color=auto";
        upgrade = "sudo nixos-rebuild switch --upgrade && home-manager switch";
      };
    };
  };

}