# Standalone disko layout for nixdots.
#
# This file is deliberately *not* a NixOS module and is *not* imported by
# flake.nix. `disko` evaluates it directly, e.g.:
#
#   disko --mode destroy,format,mount disko.nix --argstr device /dev/nvme0n1
#
# Layout: a GPT with a 1 GiB vfat ESP (label BOOT) mounted at /boot, and the
# rest as btrfs (label nixos) holding the subvolumes @rootfs (/), @nix, @home,
# @persist and @log (/var/log). A read-only @rootfs-blank template is created
# right after formatting; boot/snapshot.sh restores @rootfs from it on every
# boot, which is what makes the root filesystem ephemeral.
{
    device ? "/dev/vda",
    ...
}:
let
    # Shared btrfs mount options for every subvolume.
    mountOptions = [ "noatime" "compress=zstd" "space_cache=v2" ];
in
{
    disko.devices = {
        disk.main = {
            type = "disk";
            inherit device;

            content = {
                type = "gpt";

                partitions = {
                    ESP = {
                        type = "EF00";
                        size = "1G";

                        content = {
                            type = "filesystem";
                            format = "vfat";
                            mountpoint = "/boot";
                            # -n BOOT sets the filesystem label.
                            extraArgs = [ "-n" "BOOT" ];
                            mountOptions = [ "fmask=0022" "dmask=0022" ];
                        };
                    };

                    # GPT partition name: "System HD" (the btrfs volume that
                    # holds every subvolume, including the persisted @persist).
                    "System HD" = {
                        size = "100%";

                        content = {
                            type = "btrfs";
                            # -L nixos: required by boot/snapshot.sh, which
                            # mounts /dev/disk/by-label/nixos.
                            extraArgs = [ "-L" "nixos" ];

                            subvolumes = {
                                "@rootfs" = {
                                    mountpoint = "/";
                                    inherit mountOptions;
                                };
                                "@nix" = {
                                    mountpoint = "/nix";
                                    inherit mountOptions;
                                };
                                "@home" = {
                                    mountpoint = "/home";
                                    inherit mountOptions;
                                };
                                "@persist" = {
                                    mountpoint = "/persist";
                                    inherit mountOptions;
                                };
                                "@log" = {
                                    mountpoint = "/var/log";
                                    inherit mountOptions;
                                };
                            };

                            # @rootfs is empty at this point; snapshot it
                            # read-only so the boot rollback always has a
                            # pristine template to clone from.
                            postCreateHook = ''
                                MNTPOINT=$(mktemp -d)
                                mount "$device" "$MNTPOINT" -o subvol=/
                                btrfs subvolume snapshot -r "$MNTPOINT/@rootfs" "$MNTPOINT/@rootfs-blank"
                                umount "$MNTPOINT"
                                rmdir "$MNTPOINT"
                            '';
                        };
                    };
                };
            };
        };
    };
}
