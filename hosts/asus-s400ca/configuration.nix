# Edit this configuration file to define what should be installed on
# your system.  Help is available in the configuration.nix(5) man page
# and in the NixOS manual (accessible by running ‘nixos-help’).

{ config, lib, pkgs, ... }:


let 
   theme = import ../../theme/theme.nix { };
in

{

  imports = [
    ./hardware-configuration.nix
    ../standard
  ];

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

  xdg.portal.enable = true;
  
  security.rtkit.enable = true;

  hardware = {
    opengl = {
      enable = true;
      driSupport = true;

      extraPackages = with pkgs; [
          intel-media-driver
          vaapiIntel
          vaapiVdpau
          libvdpau-va-gl
        ];
      };

    cpu = {
      intel.updateMicrocode = true;
    };

    pulseaudio.enable = false;
  };

  networking = {
    hostId = "c65e07d9";
    hostName = "asus-s400ca";
    networkmanager.enable = true;
    useDHCP = false;
  };

  services = {
    #blueman.enable = true;
    #printing.enable = true;
    #fprintd.enable = true;
    #upower.enable = true;
    dbus.enable = true;
    zfs.autoScrub.enable = true;
    gnome.gnome-keyring.enable = true; 
    gnome.core-utilities.enable = false;
    gnome.chrome-gnome-shell.enable = true;
    logrotate.checkConfig = false;
   
    xserver = {
      videoDrivers = [ "intel" ];
      useGlamor = true;
      layout = "es";
      enable = true;
      wacom.enable = false;
      libinput.enable = true;
      
      desktopManager.gnome.enable = true;

      displayManager = {
        gdm = {
          enable = true;
          wayland = true;
        };

      defaultSession = "gnome";
      };
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
    ];
  };

  system.stateVersion = "22.05";
}