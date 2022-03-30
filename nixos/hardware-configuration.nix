{ config, lib, pkgs, modulesPath, ... }:

{
  imports =
    [ (modulesPath + "/installer/scan/not-detected.nix")
    ];

  boot = {
   supportedFilesystems = [ "zfs" ]; 
   zfs.enableUnstable = true; 
   cleanTmpDir = true;
   
   kernelPackages = pkgs.linuxPackages_lqx; # Kernel
   kernelModules = [ "kvm-intel" ]; # "kmd-ad" "wl"
   #extraModulePackages = [ config.boot.kernelPackages.broadcom_sta ]; # Broadcom Drive
   
    initrd = {
    availableKernelModules = [ "xhci_pci" "ehci_pci" "ahci" "usbhid" "usb_storage" "sd_mod" ];
    #kernelModules = [ "amdgpu" "wl" ];
    };
    
    loader = {

      systemd-boot = {
        enable = false;
        consoleMode = "max";
        configurationLimit = 3;
      };

      grub = {
        enable = true;
        version = 2;
        devices = [ "nodev" ];
        efiSupport = true;
        useOSProber = false;
        theme = null;
      };

      efi = { 
        canTouchEfiVariables = true;
        efiSysMountPoint = "/boot";
      };
    };
  };

  fileSystems."/" =
    { device = "rpool/root/nixos";
      fsType = "zfs";
    };

  fileSystems."/home" =
    { device = "rpool/home";
      fsType = "zfs";
    };

  fileSystems."/boot" =
    { device = "/dev/sda6";
      fsType = "vfat";
    };

  zramSwap = {
    enable = true;
    algorithm = "zstd";
    memoryPercent = 60;
    priority = 10;
  };

  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
  # hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;


}