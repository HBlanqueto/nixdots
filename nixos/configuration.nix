# Feel free to modify whatever you want. These files are totally based on anothers.
# Help is available in the configuration.nix(5) man page and in the NixOS manual 
# (accessible by running ‘nixos-help’).

{ config, lib, pkgs, ... }:

{
  imports =
    [
      # - Modules & bootloader. -
      ./hardware-configuration.nix

      # - Desktop Enviroment. -
      ./sys/gnome.nix

      # - GPU and Hardware Aceleration. -
      #./sys/gpu/amd.nix
      ./sys/gpu/intel.nix
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

  services = {
   blueman.enable = false;
   printing.enable = false;
   #fprintd.enable = true;
   #upower.enable = true;
   #gvfs.enable = true;
   zfs.autoScrub.enable = true;

  xserver = {
    enable = true;
    layout = "es";
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
    "0D1117"
    "FA7970"
    "7CE38B"
    "FAA356"
    "AFDBFF"
    "CEA5FB"
    "77BDFB"
    "C6CDD5"
    "89929B"
    "FA7970"
    "7CE38B"
    "FAA356"
    "AFDBFF"
    "CEA5FB"
    "77BDFB"
    "ECF2F8"
    ];
  };
  
  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  environment.systemPackages = with pkgs; [
    #chromium
    firefox
    tdesktop
    gnome.gnome-terminal
    #gnome-console
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