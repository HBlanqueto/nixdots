# Device-agnostic filesystems for the generic `nixdots` configuration. The
# installer's disko layout labels the btrfs filesystem `nixos` and the ESP
# `BOOT` (see disko.nix), so the mounts are addressed by label instead of by
# UUID and the same config is valid on every disk.
#
# modules/common/filesystems.nix adds the hardening flags to these mounts. Do
# NOT set the base options there (nor here with mkForce): nixpkgs injects
# "x-initrd.mount" into the boot-critical mounts and mkForce would drop it.
{ config, lib, ... }:

let
    # Must match mountOptions in disko.nix (space_cache=v2 is the hardware
    # config's extra and harmless to keep).
    btrfsOptions = [ "noatime" "compress=zstd" "space_cache=v2" ];

    btrfs = {
        device = "/dev/disk/by-label/nixos";
        fsType = "btrfs";
    };

    subvol = name: { options = [ "subvol=${name}" ] ++ btrfsOptions; };

    m = config.nixdots.machine;
in
{
    fileSystems = {
        "/" = btrfs // subvol "@rootfs";
        "/nix" = btrfs // subvol "@nix";
        "/home" = btrfs // subvol "@home";
        "/persist" = btrfs // subvol "@persist";
        "/var/log" = btrfs // subvol "@log";

        "/boot" = {
            # Una ESP reutilizada de Windows conserva su etiqueta, así que se
            # monta por UUID; una creada por disko lleva la etiqueta BOOT.
            device =
                if m.espUuid != ""
                then "/dev/disk/by-uuid/${m.espUuid}"
                else "/dev/disk/by-label/BOOT";
            fsType = "vfat";
            options = [ "fmask=0077" "dmask=0077" ];
        };
    };

    # Only a host that was actually encrypted gets a cryptroot; the passphrase
    # prompt is skipped entirely otherwise. boot.initrd.systemd.enable lo activa
    # siempre rollback.nix, así que no se duplica aquí.
    boot.initrd.luks.devices.cryptroot = lib.mkIf m.encrypt {
        device = "/dev/disk/by-uuid/${m.luksUuid}";
        allowDiscards = true;
    };

    assertions = [
        {
            assertion = !m.encrypt || config.boot.initrd.luks.devices ? "cryptroot";
            message = "nixdots.machine.encrypt está activo pero no hay boot.initrd.luks.devices.cryptroot";
        }
    ];
}
