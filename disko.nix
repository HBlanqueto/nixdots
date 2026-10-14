# Standalone disko layout for nixdots.
#
# This file is deliberately *not* a NixOS module and is *not* imported by
# flake.nix. `disko` evaluates it directly, e.g.:
#
#   disko --mode destroy,format,mount disko.nix --argstr device /dev/nvme0n1
#
# Layout: a GPT with a 1 GiB vfat ESP (label BOOT) mounted at /boot, and the
# rest under a LUKS2 container (name `core`) holding a btrfs filesystem (label
# nixos) with the subvolumes @rootfs (/), @nix, @home, @persist and @log
# (/var/log). A read-only @rootfs-blank template is created right after
# formatting; boot/snapshot.sh restores @rootfs from it on every boot, which is
# what makes the root filesystem ephemeral.
#
# The LUKS passphrase is supplied out-of-band: the installer passes a root-only
# file via `--argstr passwordFile <path>`; a manual run without it (null) makes
# disko prompt interactively. The unencrypted ESP keeps lanzaboote / Secure Boot
# (which read the kernel and initrd before unlocking) working.
{
    device ? "/dev/vda",
    # Path to a file containing the LUKS passphrase (absolute). `null` lets
    # disko ask for it interactively.
    passwordFile ? null,
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

                    "Core HD" = {
                        content = {
                            type = "luks";
                            name = "core";
                            inherit passwordFile;
                            settings.allowDiscards = true;
                            extraFormatArgs = [ "--type" "luks2" ];

                            content = {
                                type = "btrfs";
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
                                # pristine template to clone from. `$device` is
                                # the mapped LUKS device (/dev/mapper/core).
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
    };
}
