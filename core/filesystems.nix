# Device-agnostic filesystems for the generic `keel` configuration. The
# installer's disko layout labels the btrfs filesystem `nixos` and the ESP
# `BOOT` (see disko.nix), so the mounts are addressed by label instead of by
# UUID and the same config is valid on every disk. Módulo exclusivo del perfil
# único (flake.nix, genericModules): asume el layout de disko.
#
# Las opciones de fileSystems son listas y se concatenan, así que la base
# (subvol, noatime, compress…) y el endurecimiento (nodev, nosuid, noexec) van
# juntas sin mkForce. NUNCA usar lib.mkForce en las options de un sistema
# necesario para arrancar (/ , /nix, /persist, /var/log): nixpkgs inyecta
# "x-initrd.mount" en esas opciones y mkForce lo borraría, dejando el initrd sin
# /sysroot/nix y rota "Find NixOS closure".
{ config, lib, ... }:

let
    # Base compartida con disko.nix; space_cache=v2 es el extra del sistema
    # instalado (en disko es inocuo no ponerlo).
    btrfsOptions = import ../lib/btrfs-mount-options.nix ++ [ "space_cache=v2" ];

    btrfs = {
        device = "/dev/disk/by-label/nixos";
        fsType = "btrfs";
    };

    # Opciones de un subvolumen: la base más los flags de endurecimiento.
    subvol = name: extra: [ "subvol=${name}" ] ++ btrfsOptions ++ extra;

    m = config.keel.machine;
in
{
    fileSystems = {
        "/" = btrfs // { options = subvol "@rootfs" [ ]; };
        "/nix" = btrfs // { options = subvol "@nix" [ ]; };
        "/home" = btrfs // { options = subvol "@home" [ "nodev" "nosuid" ]; };
        # Sin noexec: impermanence monta con bind desde aquí y heredan sus flags.
        "/persist" = btrfs // {
            neededForBoot = true;
            options = subvol "@persist" [ "nodev" "nosuid" ];
        };
        # /var/log ya es neededForBoot por defecto en NixOS.
        "/var/log" = btrfs // { options = subvol "@log" [ "nodev" "nosuid" "noexec" ]; };

        "/boot" = {
            # Una ESP reutilizada de Windows conserva su etiqueta, así que se
            # monta por UUID; una creada por disko lleva la etiqueta BOOT.
            device =
                if m.espUuid != ""
                then "/dev/disk/by-uuid/${m.espUuid}"
                else "/dev/disk/by-label/BOOT";
            fsType = "vfat";
            # Solo root lee el ESP (random-seed, claves de lanzaboote).
            options = [ "fmask=0077" "dmask=0077" ];
        };
    };

    # Un scrub cubre todo el btrfs; un solo punto de montaje basta.
    services.btrfs.autoScrub = {
        enable = true;
        interval = "monthly";
        fileSystems = [ "/" ];
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
            message = "keel.machine.encrypt está activo pero no hay boot.initrd.luks.devices.cryptroot";
        }
    ];
}
