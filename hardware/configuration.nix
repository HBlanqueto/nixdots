# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{ config, lib, pkgs, ... }:

{

  imports = [
    ./hardware-configuration.nix
    ./sys/gpu/intel.nix
  ];

  hardware = {
    cpu = {
      intel.updateMicrocode = true;
      #amd.updateMicrocode = true;
    };

    pulseaudio.enable = false;
  };


  i18n.defaultLocale = "en_US.UTF-8";

  time = {
    timeZone = "America/Mexico_City";
    hardwareClockInLocalTime = true;
  };

  nix = {
    gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 3d";
    };

    optimise.automatic = true;
    settings.sandbox = false;
  };

  networking = {
    hostId = "cafebabe";
    hostName = "ASUS-C400SA";
    networkmanager.enable = true;
    useDHCP = false;
  };

  services = {
    blueman.enable = true;
    printing.enable = true;
    #fprintd.enable = true;
    upower.enable = true;
    dbus.enable = true;
    zfs.autoScrub.enable = true;
    gnome.gnome-keyring.enable = true; 
    gnome.core-utilities.enable = false;
  
    xserver = {
      layout = "es";
      enable = true;
      wacom.enable = true;
      libinput.enable = true;
      
      desktopManager.gnome.enable = true;

      displayManager = {
        gdm = {
          enable = true;
          wayland = true;
        };

      defaultSession = "gnome";
    };

    pipewire = {
      enable = true;

      wireplumber = { 
        enable = true;
        package = (import (pkgs.fetchFromGitHub { 
          owner = "K900"; 
          repo = "nixpkgs"; 
          rev = "57c7ee78b9bab82b036a232904f9161c5c4537cd";
          sha256 = "sha256-CIk45MlAa8f9UdPD5DsqjiOT89ecFsxFpM40VHOLuAE="; }) 
          { system = "x86_64-linux"; }).wireplumber;
      };

        alsa.enable = true;  
        jack.enable = true;
        pulse.enable = true;
      };
    };
  };

  zramSwap = {
    enable = true;
    algorithm = "zstd";
    memoryPercent = 45;
    priority = 10;
  };

  programs = {
    command-not-found.enable = false;
    
    zsh = {
      enable = true;
      syntaxHighlighting.enable = true;
    };
  };
  
  users = {
    mutableUsers = true;
    defaultUserShell = pkgs.zsh;

    users.hblanqueto = {
      isNormalUser = true;
      home = "/home/hblanqueto";
      description = "Mr. HBlanqueto";
      extraGroups = [
        "wheel" 
        "networkmanager"
        "audio"
        "video"
        "sudo"
      ];
    };
  };

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";

    colors = [
      "010409" "FF958E" "9DFAAA" "FBDF90" "BDfBff" "E3C9FF" "B8FFD2" "C6CDD5"
      "363B42" "EA746C" "7CE38B" "D9BE74" "BEDFE8" "BD89F5" "94E4A5" "F6FAFD"
    ];
  };

  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  environment = {
    sessionVariables = {
      MOZ_ENABLE_WAYLAND = "1";
      QT_QPA_PLATFORM="wayland";
      SDL_VIDEODRIVER= "wayland";
      CLUTTER_BACKEND = "wayland";
    }; 

    variables.EDITOR = "nvim";
    systemPackages = with pkgs; [
      firefox
      wezterm
      gnome.nautilus
      #rofi
      #pamixer
      #nm-tray
      #brightnessctl    
      neovim
      wget
      man
      zstd
    ];
  };

  fonts.fonts = with pkgs; [
    noto-fonts-emoji
    noto-fonts-cjk
    noto-fonts
    cantarell-fonts

    (nerdfonts.override {
      fonts = [
        "JetBrainsMono"
      ];
    })
  ];

  documentation = {
    enable = false;
    nixos.enable = false;
  };

  xdg.portal.enable = true;

  security.rtkit.enable = true;

  system.stateVersion = "22.05";
}