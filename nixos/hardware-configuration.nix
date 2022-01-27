# Hey! Remember to modify this acording to your partitions and module you use. 

{ config, lib, pkgs, modulesPath, ... }:

{
  imports =
    [ (modulesPath + "/installer/scan/not-detected.nix")
    ];

  fileSystems."/" =
    { device = "/dev/sda7";
      fsType = "btrfs";
      options = [ "subvol=@rootfs" "noatime" "compress=zstd" "space_cache=v2" "discard=async" "autodefrag" ];
    };

  fileSystems."/home" =
    { device = "/dev/sda7";
      fsType = "btrfs";
      options = [ "subvol=@home" "noatime" "compress=zstd" "space_cache=v2" "discard=async" "autodefrag" ];
    };

  fileSystems."/tmp" =
    { device = "/dev/sda7";
      fsType = "btrfs";
      options = [ "subvol=@tmp" "noatime" "compress=zstd" "space_cache=v2" "discard=async" "autodefrag" ];
    };

  fileSystems."/var" =
    { device = "/dev/sda7";
      fsType = "btrfs";
      options = [ "subvol=@var" "nodatacow" "discard=async" "autodefrag" ];
    };

  fileSystems."/boot" =
    { device = "/dev/sda6";
      fsType = "vfat";
    };

  swapDevices = [ ];

  hardware.cpu.amd.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
