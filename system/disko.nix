# Standalone disko layout for Keel.
#
# This file is deliberately *not* a NixOS module and is *not* imported by
# flake.nix. `disko` evaluates it directly, e.g.:
#
#   disko --mode destroy,format,mount disko.nix --argstr device /dev/nvme0n1
#
# Layout: a GPT with a 1 GiB vfat ESP (label BOOT) mounted at /boot, and the
# rest as the `System HD` partition. With `encrypt = "true"` that partition is
# wrapped in a LUKS2 container (mapper `cryptroot`) holding a btrfs filesystem
# (label nixos); otherwise btrfs sits directly on the partition. Either way the
# filesystem carries the subvolumes @rootfs (/), @nix, @home, @persist and @log
# (/var/log). A read-only @rootfs-blank template is created right after
# formatting; system/core/snapshot.sh restores @rootfs from it on every
# boot, which is what makes the root filesystem ephemeral.
#
# The LUKS passphrase comes from a single source: the installer writes the exact
# bytes typed on the password screen (no trailing newline) to a short-lived
# /run/keel/luks.pass and passes it here as `--argstr luksPasswordFile`; the
# same file is reused for the slot check, the recovery key and the TPM enroll.
# A manual run without it (null) makes disko prompt for the passphrase on stdin.
# The unencrypted ESP keeps lanzaboote / Secure Boot (which read the kernel and
# initrd before unlocking) working.
{
  device ? "/dev/vda",
  # Absolute path to the file holding the LUKS passphrase, passed by the
  # installer as `--argstr luksPasswordFile /run/keel/luks.pass`. `null`
  # (the default) makes disko ask for it on stdin.
  luksPasswordFile ? null,
  # Passed by the installer as `--argstr encrypt true` when the user checked
  # "Cifrar System HD" on the Disco screen.
  encrypt ? "false",
  ...
}:
let
  doEncrypt = encrypt == "true";

  # Base compartida con system/core/filesystems.nix.
  mountOptions = import ./lib/btrfs-mount-options.nix;

  # The btrfs filesystem itself, shared by the encrypted and plain layouts.
  # `$device` is the device btrfs is created on, i.e. /dev/mapper/cryptroot
  # when encrypted.
  btrfsContent = {
    type = "btrfs";
    extraArgs = [
      "-L"
      "nixos"
    ];

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

    # @rootfs is empty at this point; snapshot it read-only so the boot
    # rollback always has a pristine template to clone from.
    postCreateHook = ''
      MNTPOINT=$(mktemp -d)
      mount "$device" "$MNTPOINT" -o subvol=/
      btrfs subvolume snapshot -r "$MNTPOINT/@rootfs" "$MNTPOINT/@rootfs-blank"
      umount "$MNTPOINT"
      rmdir "$MNTPOINT"
    '';
  };
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
              extraArgs = [
                "-n"
                "BOOT"
              ];
              mountOptions = [
                "fmask=0077"
                "dmask=0077"
              ];
            };
          };

          "System HD" = {
            size = "100%";
            # Visible GPT partition name only. It is never used as a
            # device path (udev escapes the space as `System\x20HD`);
            # the installer finds the partition by its filesystem
            # type (crypto_LUKS) and its UUID instead.
            label = "System HD";

            content =
              if doEncrypt then
                {
                  type = "luks";
                  name = "cryptroot";
                  passwordFile = luksPasswordFile;
                  settings.allowDiscards = true;
                  extraFormatArgs = [
                    "--type"
                    "luks2"
                  ];

                  content = btrfsContent;
                }
              else
                btrfsContent;
          };
        };
      };
    };
  };
}
