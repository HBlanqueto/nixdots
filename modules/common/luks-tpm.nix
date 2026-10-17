# TPM2 auto-unlock of the LUKS2 root, via Lanzaboote's Measured Boot
# (systemd-pcrlock). It needs a TPM2 chip and Secure Boot, so it is inert unless
# `nixdots.machine.secureBoot` and `nixdots.machine.tpmUnlock` are both set (the
# latter requires the former; machine.nix asserts it).
#
# lanzaboote's autoCryptenroll is the only TPM enrol when this is on: the
# installer no longer runs `systemd-cryptenroll --tpm2-pcrs=7`, because a static
# PCR7 binding stops working as soon as lanzaboote enrolls the Secure Boot keys
# (PCR7 changes). autoCryptenroll unlocks with `--unlock-tpm2-device=auto`,
# wipes the (possible) old TPM2 slot and re-enrolls the pcrlock policy, which
# systemd-pcrlock keeps up to date on every boot and update.
{ config, lib, ... }:

let
    m = config.nixdots.machine;
in
{
    config = lib.mkIf (m.encrypt && m.tpmUnlock) {
        boot.lanzaboote = {
            measuredBoot = {
                enable = true;
                pcrs = [ 0 4 7 ];

                autoCryptenroll = {
                    enable = true;
                    device = config.boot.initrd.luks.devices."cryptroot".device;
                    autoReboot = false;
                };
            };
        };

        # Try the TPM2 token when opening the root; the passphrase stays as a
        # fallback (systemd prompts for it if the token fails).
        boot.initrd.luks.devices."cryptroot".crypttabExtraOpts = [
            "tpm2-device=auto"
        ];

        # Ephemeral root: these must outlive the rollback or the unlock breaks on
        # the next boot. /var/lib/auto-cryptenroll (the enrollment marker) is
        # persisted by secure-boot.nix whenever encrypt and tpmUnlock are set.
        environment.persistence."/persist" = {
            directories = [ "/var/lib/pcrlock.d" ];
            files = [ "/var/lib/systemd/pcrlock.json" ];
        };
    };
}
