# Btrfs subvolume rollback (impermanence): on every boot the initrd replaces
# @rootfs with the pristine @rootfs-blank snapshot created by disko. The script
# mounts the top-level subvolume by label, so this only applies to machines
# using the disko layout (the generic `nixdots` configuration imports it; a VM
# without the layout does not).
{ pkgs, ... }:

{
    boot.initrd.systemd = {
        # Tools snapshot.sh needs inside the initrd.
        initrdBin = with pkgs; [ coreutils btrfs-progs findutils util-linux gawk e2fsprogs ];

        services.rollback = {
            wantedBy = [ "initrd.target" ];
            before = [ "sysroot.mount" ];
            # El label nixos solo aparece tras abrir LUKS (si está cifrado), así que
            # esperar al dispositivo cubre ambos casos. NO pongas requires sobre
            # cryptsetup: el cifrado es opcional y sin LUKS esa unidad no existe.
            requires = [ "dev-disk-by\\x2dlabel-nixos.device" ];
            after = [
                "dev-disk-by\\x2dlabel-nixos.device"
                "systemd-cryptsetup@cryptroot.service"
            ];
            unitConfig.DefaultDependencies = "no";
            serviceConfig.Type = "oneshot";
            script = builtins.readFile ../hardware/snapshot.sh;
        };
    };
}
