# Extra hardware wiring on top of what the upstream facter modules already do
# (microcode, GPU kernel modules, hardware.graphics, bluetooth stack, fprintd,
# DHCP, initrd storage/input modules). Only active when facter is enabled, so it
# is a no-op on hosts without a report.
{ config, options, lib, pkgs, ... }:

let
    detected = config.hardware.facter.detected;
    report = config.hardware.facter.report;

    isBaremetal = detected.virtualisation.none.enable;
    isQemu = detected.virtualisation.qemu.enable;
    hasAmdGpu = detected.graphics.amd.enable;
    hasAmdCpu = lib.any (c: (c.vendor_name or "") == "AuthenticAMD") (report.hardware.cpu or [ ]);

    # SMBIOS chassis type codes for portable machines (hwinfo / DMI):
    # 8 Portable, 9 Laptop, 10 Notebook, 11 Hand Held, 14 Sub Notebook,
    # 30 Tablet, 31 Convertible, 32 Detachable.
    portableChassis = [ 8 9 10 11 14 30 31 32 ];
    isPortable = lib.any
        (c: lib.elem (c.chassis_type.value or 0) portableChassis)
        ((report.smbios or { }).chassis or [ ]);
in
{
    config = lib.mkIf config.hardware.facter.enable (lib.mkMerge [
        {
            # Physical laptop: power/lid events and firmware updates.
            services = {
                acpid.enable = lib.mkIf isPortable true;
                upower.enable = lib.mkIf isPortable true;
                fwupd.enable = lib.mkIf isBaremetal true;

                # facter enables the bluetooth stack; blueman is the GNOME applet.
                blueman.enable = lib.mkIf detected.bluetooth.enable true;

                # QEMU guests (UTM included): clipboard / dynamic resolution agent.
                spice-vdagentd.enable = lib.mkIf isQemu true;
                qemuGuest.enable = lib.mkIf isQemu true;
            };

            # fwupd state must survive the impermanence rollback.
            environment.persistence."/persist".directories = lib.optional isBaremetal "/var/lib/fwupd";

            # AMD: early KMS (facter already loads amdgpu in the initrd) plus the
            # monitoring tool.
            hardware.amdgpu.initrd.enable = lib.mkIf hasAmdGpu true;
            environment.systemPackages = lib.optional hasAmdGpu pkgs.radeontop;

            environment.sessionVariables = lib.mkIf isQemu {
                # Software cursor avoids a missing/hardware cursor artefact under
                # virtio-gpu.
                WLR_NO_HARDWARE_CURSORS = "1";
            };

            # virtio-gpu no necesita (ni gana con) la aceleración 3D de kmscon.
            nixdots.console.hwRender = lib.mkIf isQemu false;
        }

        # ucodenix only exists where its module is imported (the generic profile
        # and the physical ThinkPad), so guard these definitions on the option.
        (lib.optionalAttrs (options ? services.ucodenix) {
            # AMD bare-metal: ucodenix loads the microcode extracted from the
            # BIOS (linux-firmware does not ship it for mobile Ryzen). The
            # kernelParam disables the hash check its unsigned patches fail.
            services.ucodenix.enable = lib.mkIf (isBaremetal && hasAmdCpu) true;
            boot.kernelParams =
                lib.mkIf (isBaremetal && hasAmdCpu) [ "microcode.amd_sha_check=off" ];
        })
    ]);
}
