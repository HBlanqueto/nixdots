# TPM2 auto-unlock of the LUKS2 root, via Lanzaboote's Measured Boot
# (systemd-pcrlock). ThinkPad-only: it needs a TPM2 chip and Secure Boot, so it
# is imported by modules/hardware/thinkpad.nix and never by the UTM VM.
#
# The root is ephemeral (see snapshot.sh) and the LUKS passphrase is enrolled
# into the TPM against the PCRs that systemd-pcrlock manages. It is enabled
# automatically once the generated hardware-configuration.nix lists the LUKS
# device from disko.nix, so a host that has not been reinstalled with encryption
# (and the UTM VM) keeps booting with the passphrase and stays unaffected.
{ config, lib, ... }:

let
    cfg = config.nixdots.tpmUnlock;
in
{
    options.nixdots.tpmUnlock = {
        enable = lib.mkOption {
            type = lib.types.bool;
            # Detected from the generated hardware-configuration.nix (a LUKS root
            # is mounted at /dev/mapper/<name>) rather than from
            # boot.initrd.luks.devices, which this module itself defines below
            # and would therefore cause infinite recursion.
            default = (config.fileSystems."/".device or "") == "/dev/mapper/${cfg.luksName}";
            defaultText = lib.literalExpression ''
                config.fileSystems."/".device == "/dev/mapper/${cfg.luksName}"
            '';
            description = ''
                Unlock the LUKS2 root with the TPM2 chip using Lanzaboote's
                Measured Boot. Enabled automatically once the encrypted root
                (`${cfg.luksName}`) is present in the boot initrd.
            '';
        };

        luksName = lib.mkOption {
            type = lib.types.str;
            default = "cryptroot";
            description = "Name of the LUKS device created by disko.nix.";
        };
    };

    config = lib.mkIf cfg.enable {
        # systemd in stage 1 is required for pcrlock, the TPM2 crypttab options
        # and the rollback ordering in thinkpad.nix.
        boot.initrd.systemd = {
            enable = true;
            tpm2.enable = true;
        };

        # Try the TPM2 token when opening the root; the passphrase stays as a
        # fallback (systemd prompts for it if the token fails).
        boot.initrd.luks.devices."${cfg.luksName}".crypttabExtraOpts = [
            "tpm2-device=auto"
        ];

        boot.lanzaboote = {
            # systemd-pcrlock refuses to create a policy for more than 8 PCR4
            # variants (2 bootloader x N generations), hence the hard cap of 4.
            configurationLimit = 4;

            measuredBoot = {
                enable = true;
                pcrs = [ 0 4 7 ];

                autoCryptenroll = {
                    enable = true;
                    device = config.boot.initrd.luks.devices."${cfg.luksName}".device;
                    autoReboot = false;
                };
            };
        };

        # Ephemeral root: these must outlive the rollback or the unlock breaks
        # on the next boot. /var/lib/auto-cryptenroll (the enrollment marker)
        # is already persisted by thinkpad.nix.
        environment.persistence."/persist" = {
            directories = [ "/var/lib/pcrlock.d" ];
            files = [ "/var/lib/systemd/pcrlock.json" ];
        };
    };
}
