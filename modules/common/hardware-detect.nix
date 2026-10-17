# Extra hardware wiring on top of what the upstream facter modules already do
# (microcode, GPU kernel modules, hardware.graphics, bluetooth stack, fprintd,
# DHCP, initrd storage/input modules). Only active when facter is enabled, so it
# is a no-op on hosts without a report.
{ config, lib, pkgs, ... }:

let
    detected = config.hardware.facter.detected;
    report = config.hardware.facter.report;

    isBaremetal = detected.virtualisation.none.enable;
    isQemu = detected.virtualisation.qemu.enable;
    hasAmdGpu = detected.graphics.amd.enable;
    hasAmdCpu = lib.any (c: (c.vendor_name or "") == "AuthenticAMD") (report.hardware.cpu or [ ]);
in
{
    config = lib.mkIf config.hardware.facter.enable {
        services = {
            # Firmware updates on physical hardware.
            fwupd.enable = lib.mkIf isBaremetal true;

            # QEMU guests (UTM included): clipboard / dynamic resolution agent.
            spice-vdagentd.enable = lib.mkIf isQemu true;
            qemuGuest.enable = lib.mkIf isQemu true;

            # AMD bare-metal: ucodenix loads the microcode extracted from the
            # BIOS (linux-firmware does not ship it for mobile Ryzen).
            ucodenix.enable = lib.mkIf (isBaremetal && hasAmdCpu) true;
        };

        # AMD bare-metal: the unsigned microcode patches fail the hash check.
        boot.kernelParams =
            lib.mkIf (isBaremetal && hasAmdCpu) [ "microcode.amd_sha_check=off" ];

        environment = {
            # fwupd state must survive the impermanence rollback.
            persistence."/persist".directories = lib.optional isBaremetal "/var/lib/fwupd";

            # facter ya carga amdgpu en el initrd; aquí solo el monitor.
            systemPackages = lib.optional hasAmdGpu pkgs.radeontop;
        };

        # virtio-gpu no necesita (ni gana con) la aceleración 3D de kmscon.
        nixdots.console.hwRender = lib.mkIf isQemu false;
    };
}
