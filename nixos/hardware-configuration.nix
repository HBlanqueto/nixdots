# Mount system
# mount /dev/sdaX /mnt

# Create subvolumen
# btrfs su cr /mnt/@root
# btrfs su cr /mnt/@home
# btrfs su cr /mnt/@var
# btrfs su cr /mnt/@tmp
# umount -R /mnt

# Mount system with subvolumens
# mount -o noatime,compress=zstd,space_cache=v2,subvol=@root /dev/sdaX /mnt
# mkdir -p /mnt/{home,boot,var,tmp,}
# mount -o noatime,compress=zstd,space_cache=v2,subvol=@home /dev/sdaX /home
# mount -o noatime,compress=zstd,space_cache=v2,subvol=@tmp /dev/sdaX /tmp
# mount -o nodatacow,subvol=@var /dev/sdaX /mnt/var

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
