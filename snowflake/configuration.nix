# This configuration file needs to be used on a ZFS.

{ config, lib, pkgs, ... }:

{
  imports =
    [
        ./hardware-configuration.nix

        ./sys/env/gnome.nix
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
      
      ohMyZsh = {
          enable = true;
          plugins = [ "git" ];
          theme = "robbyrussell";
      };
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
    settings.sandbox = true;
  
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
    blueman.enable = true;
    #printing.enable = true;
    #fprintd.enable = true;
    #upower.enable = true;
    #gvfs.enable = true;
    gnome.gnome-keyring.enable = true; 
    #dbus.enable = true;
    zfs.autoScrub.enable = true; 
  
  xserver = {
    enable = true;
    layout = "es";
    wacom.enable = true;
    libinput.enable = true;
    
  displayManager = {
    gdm = {
      enable = true;
      autoSuspend = false;
      wayland = true;
      #nvidiaWayland = true;   
    };

      #defaultSession = "none+awesome";
      defaultSession = "gnome";

    };
  };

  pipewire = {
    enable = true;
    wireplumber.enable = true;

    alsa = {     
      enable = true;
      support32Bit = true;    
    };

    pulse.enable = true;
    jack.enable = true;
    
    config = import ./sys/pipewire;
      media-session.config = import ./sys/pipewire/media-session.nix;
      #media-session.enable = true;
    };
  }; 
  
  hardware.pulseaudio.enable = false;

  console = {
    font = "Lat2-Terminus16";
    keyMap = "es";
    colors = [
    
    "010409"  "FF958E" "9DFAAA" "FBDF90" 
    "CEE9FF" "E3C9FF" "A5D5FF" "C6CDD5"
    "363B42" "FA7970" "7CE38B" "FAA356" 
    "CEE9FF" "E3C9FF" "A5D5FF" "F6FAFD"
    
    ];
  };
  
  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  environment = {
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
    noto-fonts
    noto-fonts-cjk
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
