{ config, lib, pkgs, ... }:

let theme = import ../../theme { };
in

{
  boot.kernelPackages = pkgs.linuxPackages_xanmod;

  console = 
    let
      normal = with theme; [ bg c1 c2 c3 c4 c5 c6 c7 ];
      bright = with theme; [ lbg c9 c10 c11 c12 c13 c14 c15 ];
    in 
  
  {
    colors = normal ++ bright;
    font = "Lat2-Terminus16";
    keyMap = "es";
  };

  zramSwap = {
    enable = true;
    memoryPercent = 75;
    priority = 10;
  };

  hardware.pulseaudio.enable = false;
  
  security.rtkit.enable = true;

  services.pipewire = {
    enable = true;

    alsa = {
      enable = true;
      support32Bit = true;
    };

    jack.enable = true;
    pulse.enable = true;
    wireplumber.enable = true;
  };

  systemd.user.services = {
    pipewire.wantedBy = [ "default.target" ];
    pipewire-pulse.wantedBy = [ "default.target" ];
  };

  i18n = {
    defaultLocale = "en_US.UTF-8";
    supportedLocales = [ "en_US.UTF-8/UTF-8" ];
  };

  time = {
    timeZone = "America/Mexico_City";
    hardwareClockInLocalTime = true;
  };

  programs = {
    command-not-found.enable = false;
    
    zsh = {
      enable = true;
      syntaxHighlighting.enable = true;
    };
  };

  documentation = {
    enable = false;
    nixos.enable = false;
  };

  environment = {
    systemPackages = with pkgs; [
      efibootmgr
      polkit_gnome
      home-manager
      wget
      man
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
}
