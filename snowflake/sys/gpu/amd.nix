{ config, pkgs, lib, ... }:

{
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

  hardware = {  

    cpu = {
      amd.updateMicrocode = true;
    };

    opengl = {
      driSupport = true;
      #driSupport32Bit = true;  
      enable = true;
   
    extraPackages = with pkgs; [
      rocm-opencl-icd
      rocm-opencl-runtime
      vaapiVdpau
      libvdpau-va-gl
      ];
    };
  };
}