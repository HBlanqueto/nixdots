{ pkgs, inputs, ... }:

{
    # Everything that depends on this being the physical ThinkPad L14 Gen 2a:
    # AMD Ryzen 5650U (x86_64-v3), Radeon graphics, laptop power, and the
    # machine's optional peripherals. None of it is valid on the aarch64 VM.
    #
    # The bootloader (Secure Boot/lanzaboote) is now driven by
    # modules/common/secure-boot.nix from nixdots.machine, the rollback service
    # lives in modules/common/rollback.nix, and the peripherals in
    # modules/common/extras.nix.
    imports = [
        inputs.lanzaboote.nixosModules.lanzaboote
        ../common/luks-tpm.nix
        ../common/rollback.nix
    ];

    # Own keys + Lanzaboote.
    nixdots.machine.secureBoot = true;

    # Peripherals/roles of this machine. remoteAccess is enabled in
    # hosts/ThinkPad-L14-Gen2/default.nix.
    nixdots.extras = {
        epson = true;
        trezor = true;
        chime = true;
        libvirt = true;
        brightnessctl = true;
    };

    hardware = {
        enableRedistributableFirmware = true;

        # AMD GPU: early KMS. Microcode comes from linux-firmware
        # (hardware.cpu.amd.updateMicrocode); ucodenix was dropped.
        amdgpu.initrd.enable = true;
        cpu.amd.updateMicrocode = true;

        bluetooth.enable = true;
    };

    services = {
        acpid.enable = true;
        blueman.enable = true;
        fwupd.enable = true;
        upower.enable = true;
    };

    environment.persistence."/persist".directories = [
        "/var/lib/fwupd"
    ];

    environment.systemPackages = with pkgs; [
        radeontop
    ];
}
