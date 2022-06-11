{ config, pkgs, home, ... }:

let
  theme = import ../theme { };
in

{
  home = {

    file = {
      # Wezterm
      ".config/wezterm/wezterm.lua".text = import ./programs/wezterm.nix { inherit theme; }; 

      # Gnome terminal padding
      ".config/gtk-3.0/gtk.css".text = import ./programs/gtk.nix { };
    };

      sessionVariables = {
        MOZ_DISABLE_RDD_SANDBOX="1";
        MOZ_USE_XINPUT2 = "1";
        NO_AT_BRIDGE = "1";
    };

    packages = with pkgs; [
       
        # Programs
        vscode
        neofetch
        tdesktop
        obs-studio
        amberol
        clapper
        gnome-text-editor
        gnome.gnome-tweaks
        xdg-user-dirs

        # Software Development
        # Compilers and tools
        #gnumake
        #clang
        #llvm
        rustc
        cargo
        #python39
        lua5_4
        luarocks

        # Awesome Window Manager
        #rofi
        #pamixer
        #nm-tray
        #brightnessctl
      ];
    };

  programs = {

    firefox = {
      enable = true;

      profiles = {
          myuser = {
            id = 0;
            settings = {
              
              "media.ffmpeg.vaapi.enabled" = true;
              "media.ffvpx.enabled" = false;
              "media.navigator.mediadatadecoder_vpx_enabled" = true;
              #"gfx.webrender.all" = true;
              #"media.av1.enabled" = false;

              "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
              "svg.context-properties.content.enabled" = true;
              "browser.uidensity" = "0";
              
             };
           };
         };
       }; 

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
        update = "sudo nix-channel --update &&  nix-channel --update";
        upgrade = "sudo nixos-rebuild switch --upgrade && home-manager switch";
      };
    };
  };

}
