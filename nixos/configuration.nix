# This configuration file needs to be used on a ZFS.

{ config, lib, pkgs, ... }:

{
  imports =
    [
      # - Modules & partitions. -
      ./hardware-configuration.nix

      # - Desktop Enviroment. -
      ./sys/gnome.nix

      # - GPU and Hardware Aceleration. -
      ./sys/gpu/amd.nix
      #./sys/gpu/intel.nix
      #./sys/gpu/nvidia.nix
    ];
  
  system.stateVersion = "22.05";

  programs.zsh = {
    enable = true;
    autosuggestions.enable = true;
    syntaxHighlighting.enable = true;
    ohMyZsh.enable = true;
    ohMyZsh.plugins = [ "git" ];
    ohMyZsh.theme = "gentoo";
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
      ];
    };   
  };

  nix = {
    gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 7d";
    };

    optimise.automatic = true;
    settings.sandbox = false;
  };

  networking = {
    hostId = "c39e1dae";
    hostName = "HP-23-q025la"; 
    networkmanager.enable = true;
    firewall.enable = true;
    useDHCP = false;

  };

  i18n.defaultLocale = "en_US.UTF-8";

  time = {
    timeZone = "America/Mexico_City";
    hardwareClockInLocalTime = true;
  };

  services = {
   blueman.enable = true;
   printing.enable = false;  
   zfs.autoScrub.enable = true;

  xserver = {
    enable = true;
    layout = "es";
    desktopManager.xterm.enable = false;
    libinput.enable = true;

  # Use this If you only use Gnome
  displayManager = {  
   gdm = {
      enable = true;
      autoSuspend = true;
      wayland = true;
      #nvidiaWayland = true;   
     };
   defaultSession = "gnome";
    };
  };

  pipewire = {
    enable = true;
    wireplumber.enable = true;

    alsa = {     
      enable = true;
      #support32Bit = true;    
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
    "000000"
    "dc322f"
    "859900"
    "b58900"
    "268bd2"
    "d33682"
    "2aa198"
    "F6F2ED"
    "9AA3A6"
    "dc322f"
    "859900"
    "b58900"
    "268bd2"
    "d33682"
    "2aa198"
    "7F7762"

    ];
  };
  
  nixpkgs.config.allowUnfree = true;
  
  environment.systemPackages = with pkgs; [ 
    firefox-wayland
    tdesktop
    gnome.gnome-terminal
    gnome.nautilus
    nano
    wget
    man 
    zstd
  ];
  
  fonts.fonts = with pkgs; [
    noto-fonts-emoji-blob-bin
    noto-fonts
    noto-fonts-cjk
    powerline-fonts
    ipafont
    #cantarell-fonts
    #inter

  (nerdfonts.override {
      fonts = [
        "JetBrainsMono"
      ];
    })
  ];

  documentation.enable = false;
  documentation.nixos.enable = false;
  programs.command-not-found.enable = false;

}