{ config, pkgs, ... }:

{
  services = {   
   xserver = {
    desktopManager = {
     gnome = {
     enable = true;
      };
     };
   };  
 };

 
 services.gnome.core-utilities.enable = false;
 
 xdg.portal.wlr.enable = true;
 
 environment.sessionVariables = {
  MOZ_ENABLE_WAYLAND = "1";
  QT_QPA_PLATFORM="wayland";
 }; 
}
