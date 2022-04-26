{ config, lib, pkgs, modulesPath, ... }:

{
  imports =
    [ 
      (modulesPath + "/installer/scan/not-detected.nix")
    ];

  boot = {
    supportedFilesystems = [ "zfs" ]; 
    zfs.enableUnstable = true; 
    cleanTmpDir = true;
   
    kernelPackages = pkgs.linuxPackages_lqx;
    kernelModules = [ "kvm-intel" ]; # "kmd-ad" "wl"
    #extraModulePackages = [ config.boot.kernelPackages.broadcom_sta ];
   
    initrd = {
    availableKernelModules = [ "xhci_pci" "ehci_pci" "ahci" "usbhid" "usb_storage" "sd_mod" ];
    #kernelModules = [ "amdgpu" "wl" ];
    };
    
    loader = {
      grub = {
        enable = true;
        version = 2;
        devices = [ "nodev" ];
        efiSupport = true;
        useOSProber = true;
      };

      efi = { 
        canTouchEfiVariables = true;
        efiSysMountPoint = "/boot";
      };
    };
  };

  fileSystems."/" =
    { 
      device = "rpool/root/nixos";
      fsType = "zfs";
    };

  fileSystems."/home" =
    { 
      device = "rpool/home";
      fsType = "zfs";
    };

  fileSystems."/boot" =
    { 
      device = "/dev/disk/by-uuid/6A77-97E0";
      fsType = "vfat";
    };

  zramSwap = {
    enable = true;
    algorithm = "zstd";
    memoryPercent = 30;
    priority = 10;
  };

  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
  # hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

}
