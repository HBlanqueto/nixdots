{ config, lib, pkgs, modulesPath, ... }:

{
  imports =
    [ (modulesPath + "/installer/scan/not-detected.nix")
    ];

  boot = {
   supportedFilesystems = [ "zfs" ]; 
   zfs.enableUnstable = true; 
   cleanTmpDir = true;
   
   kernelPackages = pkgs.linuxPackages_xanmod; # Kernel
   kernelModules = [ "kvm-amd" "wl" ];
   extraModulePackages = [ config.boot.kernelPackages.broadcom_sta ]; # Broadcom Driver
   
    initrd = {
    availableKernelModules = [ "xhci_pci" "ahci" "ehci_pci" "usbhid" "usb_storage" "sd_mod" "sr_mod" "rtsx_pci_sdmmc" ];
    kernelModules = [ "amdgpu" "wl" ];
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
    memoryPercent = 30;
    priority = 5;
  };

  hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

}