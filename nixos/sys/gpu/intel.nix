
{ config, pkgs, lib, ... }:

{
  # Xserver Configuration
  services.xserver = {
    videoDrivers = [ "modesetting" ];
    useGlamor = true;
  };

  # Hardware Acceleration
  nixpkgs.config.packageOverrides = pkgs: {
    vaapiIntel = pkgs.vaapiIntel.override {
      enableHybridCodec = true;
    };
  };

  hardware = { 
  
   cpu = {
     intel.updateMicrocode = true;
    };

    opengl = {
      enable = true;
      driSupport = true;
      #driSupport32Bit = true;
      #extraPackages32 = with pkgs.pkgsi686Linux; [ vaapiIntel ];

    extraPackages = with pkgs; [
      intel-media-driver
      vaapiIntel
      vaapiVdpau
      libvdpau-va-gl
     ];
    };
  };
}