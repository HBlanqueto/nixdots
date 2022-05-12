{ config, pkgs, lib, ... }:

{

  services.xserver = {
    videoDrivers = [ "intel" ];
    useGlamor = true;
  };


  nixpkgs.config.packageOverrides = pkgs: {
    vaapiIntel = pkgs.vaapiIntel.override {
      enableHybridCodec = true;
    };
  };

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
  };
}
