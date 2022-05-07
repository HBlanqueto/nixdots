# This configuration file needs to be used on a ZFS.

{ config, lib, pkgs, ... }:

{
  imports =
    [
        ./hardware-configuration.nix

        #./sys/env/awesome.nix

        #./sys/gpu/amd.nix
        ./sys/gpu/intel.nix
        #./sys/gpu/nvidia.nix
    ];

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
    description = "Humberto Blanqueto";
    home = "/home/hblanqueto";
    
    extraGroups = [ 
        "wheel" 
        "networkmanager"
        "audio"
        "video"
        "sudo"
      ];
    };   
  };

  nix = {
    gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 5d";
    };

    optimise.automatic = true;
    settings.sandbox = false;
  
  };

  hardware = {
    cpu = {
      intel.updateMicrocode = true;
      #amd.updateMicrocode = true;
    };
  };

  networking = {
    # generate the hostID through executing:
    # $ head -c4 /dev/urandom | od -A none -t x4
    hostId = "cafebabe";
    hostName = "ASUS-C400SA"; 
    networkmanager.enable = true;
    firewall.enable = true;
    useDHCP = false;

  };

  i18n.defaultLocale = "en_US.UTF-8";

  time = {
    timeZone = "America/Mexico_City";
    hardwareClockInLocalTime = true;
  };

  security.rtkit.enable = true;

  services = {
    #blueman.enable = true;
    #printing.enable = true;
    #fprintd.enable = true;
    upower.enable = true;
    gvfs.enable = true;
    dbus.enable = true;
    zfs.autoScrub.enable = true;
    gnome.gnome-keyring.enable = true; 
    gnome.core-utilities.enable = false;
  
  xserver = {
    enable = true;
    layout = "es";
    wacom.enable = true;
    libinput.enable = true;
    
    displayManager = {
      gdm = {
        enable = true;
        autoSuspend = true;
        wayland = true;
        #nvidiaWayland = true;   
      };

      #defaultSession = "none+awesome";
      defaultSession = "gnome";
    };

    desktopManager = {
      gnome.enable = true;
    };
  };

  pipewire = {
    enable = true;
    wireplumber = { 
      enable = true;
      package = (import (pkgs.fetchFromGitHub { owner = "K900"; repo = "nixpkgs"; rev = "57c7ee78b9bab82b036a232904f9161c5c4537cd"; sha256 = "sha256-CIk45MlAa8f9UdPD5DsqjiOT89ecFsxFpM40VHOLuAE="; }) { system = "x86_64-linux"; }).wireplumber;
    };

    alsa = {     
      enable = true;
      support32Bit = true;    
    };

    pulse.enable = true;
    jack.enable = true;
  
    };
  }; 

  hardware.pulseaudio.enable = false;

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
      #QT_WAYLAND_DISABLE_WINDOWDECORATION= "1";
      #NIXOS_OZONE_WL="1";
    }; 

    variables.EDITOR = "nvim";
    systemPackages = with pkgs; [

      #firefox
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
    noto-fonts-emoji-blob-bin
    noto-fonts-cjk
    noto-fonts
    powerline-fonts
    ipafont

    cantarell-fonts
    #inter

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
  system.stateVersion = "22.05";

}