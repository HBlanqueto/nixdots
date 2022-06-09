# This file contains services and packages I usually use in all
# my machines, modify configuration.nix from the folder asus-s400ca

{ config, lib, pkgs, ... }:

let 
   theme = import ../../theme/theme.nix { };
in

{
  boot.kernelPackages = pkgs.linuxPackages_lqx;

  nix = {
    extraOptions = ''
      experimental-features = nix-command flakes
    '';

    autoOptimiseStore = true;
    checkConfig = true;

    gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 7d";
    };

    optimise.automatic = true;
    settings.sandbox = false;
  };

  console = 
    let
      normal = with theme.colors; [ bg c1 c2 c3 c4 c5 c6 c7 ];
      bright = with theme.colors; [ lbg c9 c10 c11 c12 c13 c14 c15 ];
    in {

    colors = normal ++ bright;
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  i18n.defaultLocale = "en_US.UTF-8";

  time = {
    timeZone = "America/Mexico_City";
    hardwareClockInLocalTime = true;
  };

  zramSwap = {
    enable = true;
    algorithm = "zstd";
    memoryPercent = 65;
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
      description = "Humberto Blanqueto";
      extraGroups = [
        "wheel" 
        "networkmanager"
        "audio"
        "video"
        "sudo"
      ];
    };
  };

  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  environment = {
    systemPackages = with pkgs; [
      efibootmgr
      polkit_gnome
      home-manager
      wget
      man
      zstd
    ];
  };

  fonts.fonts = with pkgs; [
    twemoji-color-font
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

}