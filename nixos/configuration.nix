# HBlanqueto's cofig
# I'm new using NixOS, my configuration isn't the best. Please do not use this as a template.

{ config, pkgs, ... }:

{
  imports =
    [
      # Modules & partitions.
      ./hardware-configuration.nix
      ./default-session.nix

      # GPU and Hardware Aceleration configuration.
      ./gpu/amd.nix
      #./gpu/nvidia.nix
    ];
  
  system.stateVersion = "22.05";

  boot = {
   supportedFilesystems = [ "btrfs" ]; 
   cleanTmpDir = true;
   
   kernelPackages = pkgs.linuxPackages_xanmod;
   kernelModules = [ "kvm-amd" "wl" ];
   extraModulePackages = [ config.boot.kernelPackages.broadcom_sta ];
   
    initrd = {
    availableKernelModules = [ "xhci_pci" "ahci" "ehci_pci" "usbhid" "usb_storage" "sd_mod" "sr_mod" "rtsx_pci_sdmmc" ];
    kernelModules = [ "amdgpu" "wl" ];
    };
    
    loader = {
      grub = {
        enable = true;
        version = 2;
        devices = [ "nodev" ];
        efiSupport = true;
        useOSProber = false; # Dualboot
      };

      efi = { canTouchEfiVariables = true; };
    };
  };

  programs.zsh.enable = true;
  
  users = {
  
  mutableUsers = true;
  defaultUserShell = pkgs.zsh;

  users.hblanqueto = {
    isNormalUser = true;
    description = "Humberto Blanqueto";
    home = "/home/hblanqueto";
    extraGroups = [ "wheel" ];
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
    useSandbox = false;
  };

  networking.hostName = "HP-23-q025la"; 
  
  networking.networkmanager.enable = true;

  time.timeZone = "America/Mexico_City";
  
  time.hardwareClockInLocalTime = true;

  i18n.defaultLocale = "en_US.UTF-8";
   
  hardware.pulseaudio.enable = false;

  services = {

  blueman.enable = true;
  printing.enable = false;

  xserver = {
    enable = true;
    layout = "es";
 
  displayManager = {
    gdm = {
     enable = true;
     autoSuspend = true;
     wayland = true;
     #nvidiaWayland = true;
      };
    };
  };

  pipewire = {
   enable = true;
   alsa.enable = true;
   alsa.support32Bit = true;
   pulse.enable = true;
   #jack.enable = true;
   #media-session.enable = true;
    };
  }; 
  
  console = {
     font = "Lat2-Terminus16";
     keyMap = "es";
  };
  
  nixpkgs.config.allowUnfree = true;
  
  documentation.enable = false;
  documentation.nixos.enable = false;
  programs.command-not-found.enable = false;
  
  environment.systemPackages = with pkgs; [ 
  firefox-wayland
  #google-chrome
  tdesktop
  gnome.gnome-terminal
  gnome.nautilus
  # wezterm
  nano
  wget
  man 
  btrfs-progs
  zstd
  ];
  
  fonts.fonts = with pkgs; [
  noto-fonts-emoji-blob-bin
  ipafont
  #cantarell-fonts
  #inter
  ];
}
