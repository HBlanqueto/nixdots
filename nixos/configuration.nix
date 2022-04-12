# This configuration file needs to be used on a ZFS.

{ config, lib, pkgs, ... }:

{
  imports =
    [
      # - Modules & bootloader. -
      ./hardware-configuration.nix

      # - Desktop Enviroment. -
      #./sys/gnome.nix
      ./sys/awesome.nix

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
      "sudo"
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

    networkmanager.enable = true;
    # generate the hostID through executing:
    # $ head -c4 /dev/urandom | od -A none -t x4
    hostId = "cafebabe";
    hostName = "ASUS-C400SA";
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
   #upower.enable = true;
   #gvfs.enable = true;
   gnome.gnome-keyring.enable = true; 
   zfs.autoScrub.enable = true;
   dbus.enable = true;    
  
  xserver = {
    enable = true;
    layout = "es";
    libinput.enable = true;
    
   displayManager = {

   gdm = {
      enable = true;
      autoSuspend = false;
      wayland = true;
      #nvidiaWayland = true;   
     };
 
      defaultSession = "none+awesome";

    };
  };

  pipewire = {
    enable = true;
    wireplumber.enable = false;

    alsa = {     
      enable = true;
      #support32Bit = true;    
     };

    pulse.enable = true;
    jack.enable = true;
    
    config = import ./sys/pipewire;
     media-session.config = import ./sys/pipewire/media-session.nix;
     media-session.enable = true;
    };
  }; 
  
  hardware.pulseaudio.enable = false;

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
    colors = [
    
    "010409"  "FF958E" "9DFAAA" "FCBE87" 
    "CEE9FF" "E3C9FF" "A5D5FF" "C6CDD5"
    "363B42" "FA7970" "7CE38B" "FAA356" 
    "CEE9FF" "E3C9FF" "A5D5FF" "F6FAFD"
    
    ];
  };
  
  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  environment.systemPackages = with pkgs; [
    #chromium
    google-chrome-dev
    #firefox
    tdesktop
    
    #gnome.gnome-terminal
    wezterm
    #gnome-console
    
    #gnome.nautilus
    xfce.thunar

    rofi
    pamixer
    #nm-tray
    polkit_gnome
    nano
    wget
    man 
    zstd
  ];
  
  fonts.fonts = with pkgs; [
    noto-fonts-emoji-blob-bin
    noto-fonts
    #noto-fonts-cjk
    powerline-fonts
    #ipafont
    cantarell-fonts
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
