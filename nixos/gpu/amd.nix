
{ config, pkgs, lib, ... }:

{

  # Xserver configuration
  services.xserver = {
    videoDrivers = [ "amdgpu" ];
    extraConfig = ''
      Section "Device"
        Identifier "Radeon"
        Driver "radeon"
        Option "TearFree" "on"
      EndSection
      Section "Device"
        Identifier "AMD"
        Driver "amdgpu"
        Option "TearFree" "true"
      EndSection
    '';
  };


  # Hardware Aceleration
  hardware.opengl = {
  enable = true;
  extraPackages = with pkgs; [
      rocm-opencl-icd
      rocm-opencl-runtime
      vaapiVdpau
      libvdpau-va-gl
    ];
  };

  hardware.opengl.driSupport = true;
  # For 32 bit applications
  #hardware.opengl.driSupport32Bit = true;

}
