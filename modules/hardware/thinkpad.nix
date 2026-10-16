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
        # ucodenix carga microcódigo extraído de imágenes de BIOS porque
        # linux-firmware no lo trae para los Ryzen de portátil.
        inputs.ucodenix.nixosModules.default
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

    boot = {
        # amd_sha_check=off es necesario para ucodenix: esos parches no están
        # en la lista de hashes que el kernel comprueba desde EntrySign.
        # Reevaluar tras actualizar la BIOS con fwupd.
        kernelParams = [ "microcode.amd_sha_check=off" ];
    };

    hardware = {
        enableRedistributableFirmware = true;

        # AMD GPU: early KMS. The microcode comes from ucodenix (see imports).
        amdgpu.initrd.enable = true;

        bluetooth.enable = true;

        # Radeon VA-API (radeonsi ships in mesa).
        graphics.extraPackages = [ pkgs.mesa ];
    };

    services = {
        ucodenix = {
            enable = true;
        };

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
