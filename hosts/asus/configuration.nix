# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{ config, lib, pkgs, ... }:

{
  boot = {
    supportedFilesystems = [ "zfs" ];
    zfs.enableUnstable = true; 
    cleanTmpDir = true;
   
    initrd.kernelModules = [ "i915" ];

    kernelParams = [
      "acpi_backlight=native"
      "i915.enable_psr=0"
      "i915.enable_guc=2"
    ];
    
    loader = {
      grub = {
        enable = true;
        version = 2;
        devices = [ "nodev" ];
        efiSupport = true;
        useOSProber = true;
      };

      efi = { 
        canTouchEfiVariables = true;
        efiSysMountPoint = "/boot";
      };

    };
  };

  hardware = {
    enableRedistributableFirmware = true;

    opengl = {
      enable = true;
      driSupport = true;
      driSupport32Bit = true;

      extraPackages = with pkgs; [
          intel-media-driver
          vaapiIntel
          vaapiVdpau
          libvdpau-va-gl
        ];
      };

    cpu.intel.updateMicrocode = true;
  };

  imports = [
    ./hardware-configuration.nix
    
    # Shared configuration across all machines
    ../shared
  ];

  networking = {
    hostId = "c65e07d9";
    hostName = "asus-s400ca";
    networkmanager.enable = true;

    useDHCP = false;
  };

  services = {
    #blueman.enable = true;
    #printing.enable = true;

    #upower.enable = true;
    dbus.enable = true;
    gnome.gnome-keyring.enable = true; 

    gnome.core-utilities.enable = false;
    logrotate.checkConfig = false;
    zfs.autoScrub.enable = true;
   
    xserver = {
      enable = true;
      layout = "es";
      wacom.enable = false;
      videoDrivers = [ "intel" ];
      useGlamor = true;
      
      desktopManager.gnome.enable = true;
      displayManager.gdm.enable = true;
    };
  };

  environment = {
    sessionVariables = {
      MOZ_ENABLE_WAYLAND = "1";
      QT_QPA_PLATFORM="wayland";
      SDL_VIDEODRIVER= "wayland";
      CLUTTER_BACKEND = "wayland";
    }; 

    systemPackages = with pkgs; [
      firefox
      wezterm
      gnome.nautilus
    ];
  };

  xdg.portal.enable = true;

  system.stateVersion = "22.05";
}