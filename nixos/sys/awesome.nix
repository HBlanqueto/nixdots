{ config, pkgs, ... }:

{

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
      shadowOffsets = [ (-18) (-18) ];
      shadowOpacity = 0.8;

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
        "85:class_g = 'splash'"
      ];

      wintypes = {
        popup_menu = { full-shadow = true; };
        dropdown_menu = { full-shadow = true; };
        notification = { full-shadow = true; };
        normal = { full-shadow = true; };
      };

      settings = {
        # animations = true;

        corner-radius = 7;
        rounded-corners-exclude = [
          
        ];

        blur-method = "dual_kawase";
        blur-strength = 8.0;
        kernel = "11x11gaussian";
        blur-background = false;
        blur-background-frame = false;
        blur-background-fixed = false;

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

 nixpkgs.config.google-chrome-dev.commandLineArgs = "--enable-features=VaapiVideoDecoder --disable-features=UseOzonePlataform";

 xdg = {
  portal = {
    enable = true;
    extraPortals = with pkgs; [
      xdg-desktop-portal-wlr
    ];
    gtkUsePortal = true;
   };
 };

}
