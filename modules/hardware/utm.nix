{ lib, ... }:

{
    # UTM (QEMU backend) on Apple Silicon, native aarch64. Stock kernel and
    # plain systemd-boot: no Lanzaboote/Secure Boot, no AMD microcode, TLP,
    # fprintd or battery handling.
    boot = {
        loader.systemd-boot.enable = true;
        loader.systemd-boot.configurationLimit = 5;

        # virtio drivers needed to find the disk, network and console inside
        # the VM. virtio_gpu is used by the virtio-gpu(-gl) display device.
        initrd.availableKernelModules = [
            "virtio_pci"
            "virtio_blk"
            "virtio_scsi"
            "virtio_net"
            "virtio_gpu"
            "xhci_pci"
            "usbhid"
        ];

        initrd.kernelModules = [ "virtio_gpu" ];
    };

    # virtio-gpu no necesita (ni gana con) la aceleración 3D de kmscon.
    nixdots.console.hwRender = false;

    # Clipboard sharing and dynamic resolution with the UTM/SPICE console.
    services.spice-vdagentd.enable = true;
    services.qemuGuest.enable = true;

    environment.sessionVariables = {
        # Software cursor avoids a missing/hardware cursor artefact under
        # virtio-gpu.
        WLR_NO_HARDWARE_CURSORS = "1";
    };

    # Optional shared folder with the Mac. Pick either virtiofs or 9p in the
    # UTM device settings, then uncomment one of the mounts below.
    #
    # fileSystems."/mnt/mac-share" = {
    #     device = "share";                 # virtiofs tag, or "9p" source
    #     fsType = "virtiofs";              # or "9p"
    #     options = [ "nofail" "x-systemd.automount" ];
    # };

    # virtualisation.rosetta.enable = true;   # x86_64 emulation via Rosetta.
    # Rosetta only works with UTM's "Apple Virtualization" backend, NOT with
    # the QEMU backend this host targets, so it is left disabled.
}
