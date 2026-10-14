# PLACEHOLDER - replace with the file produced by `nixos-generate-config`
# inside the VM after partitioning:
#
#   nixos-generate-config --root /mnt
#   cp /mnt/etc/nixos/hardware-configuration.nix ~/nixdots/hosts/utm/
#
# The values below are plausible for a single UEFI disk with a Btrfs root
# split into the same subvolumes as the ThinkPad, but the UUIDs are fake.
{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [ ];

  boot.initrd.availableKernelModules = [ "virtio_pci" "virtio_blk" "xhci_pci" "usbhid" "usb_storage" ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ ];
  boot.extraModulePackages = [ ];

  fileSystems."/" =
    { device = "/dev/disk/by-uuid/00000000-0000-0000-0000-000000000000";
      fsType = "btrfs";
      options = [ "subvol=@rootfs" "noatime" "compress=zstd" "space_cache=v2" ];
    };

  fileSystems."/home" =
    { device = "/dev/disk/by-uuid/00000000-0000-0000-0000-000000000000";
      fsType = "btrfs";
      options = [ "subvol=@home" "noatime" "compress=zstd" "space_cache=v2" ];
    };

  fileSystems."/nix" =
    { device = "/dev/disk/by-uuid/00000000-0000-0000-0000-000000000000";
      fsType = "btrfs";
      options = [ "subvol=@nix" "noatime" "compress=zstd" "space_cache=v2" ];
    };

  fileSystems."/persist" =
    { device = "/dev/disk/by-uuid/00000000-0000-0000-0000-000000000000";
      fsType = "btrfs";
      options = [ "subvol=@persist" "noatime" "compress=zstd" "space_cache=v2" ];
    };

  fileSystems."/var/log" =
    { device = "/dev/disk/by-uuid/00000000-0000-0000-0000-000000000000";
      fsType = "btrfs";
      options = [ "subvol=@log" "noatime" "compress=zstd" "space_cache=v2" ];
    };

  fileSystems."/boot" =
    { device = "/dev/disk/by-uuid/0000-0000";
      fsType = "vfat";
      options = [ "fmask=0022" "dmask=0022" ];
    };

  swapDevices = [ ];

  nixpkgs.hostPlatform = lib.mkDefault "aarch64-linux";
}
