{ config, pkgs, ... }:

{

  networking = {
    networkmanager.enable = true;
    firewall.enable = true;
    useDHCP = false;
  };

  # This theme GTK programs
  environment.systemPackages = with pkgs; [
    gtk-engine-murrine
    gtk_engines
    gsettings-desktop-schemas
    lxappearance
  ];

  services = {   

    xserver = {     
      windowManager = {
        awesome = {
          enable = true;
          luaModules = with pkgs.luajitPackages; [
              lgi
              ldbus
              luadbi-mysql
              luaposix
           ];
         };
       };
    };
    
    picom = {
      enable = true;
      experimentalBackends = true;
      backend = "glx";
      vSync = true;
      shadow = true;
      shadowOffsets = [ (-25) (-25) ];
      shadowOpacity = 0.6;

      shadowExclude = [
        #"class_g = 'slop'"
        "window_type = 'menu'"
        "window_type = 'dock'"
        #"window_type = 'desktop'"
        "class_g = 'Firefox' && window_type *= 'utility'"
        "_GTK_FRAME_EXTENTS@:c"
        "class_g = 'TelegramDesktop' && argb"
      ];

      opacityRules = [
        #"85:class_g = 'splash'"
      ];

      wintypes = {
        popup_menu = { full-shadow = true; };
        dropdown_menu = { full-shadow = true; };
        notification = { full-shadow = true; };
        normal = { full-shadow = true; };
      };

      settings = {
        #animations = true;

        corner-radius = 8;
        rounded-corners-exclude = [
         "window_type = 'dock'"
         "class_g = 'Conky'"
         "class_g = 'Rofi'"  
        ];

        blur-method = "dual_kawase";
        blur-strength = 8.0;
        kernel = "11x11gaussian";
        blur-background = false;
        blur-background-frame = true;
        blur-background-fixed = true;

        blur-background-exclude = [
          "!window_type = 'splash'"
        ];

        daemon = false;
        dbus = false;
        mark-wmwin-focused = true;
        mark-ovredir-focused = true;
        detect-rounded-corners = true;
        detect-client-opacity = true;
        glx-no-stencil = true;
        use-damage = true;
        resize-damage = 1;
        transparent-clipping = false;
      };
    };
  };

  nixpkgs.config.google-chrome.commandLineArgs = "--enable-features=VaapiVideoDecoder --disable-features=UseOzonePlataform";

}
