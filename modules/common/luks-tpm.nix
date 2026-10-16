# TPM2 auto-unlock of the LUKS2 root, via Lanzaboote's Measured Boot
# (systemd-pcrlock). It needs a TPM2 chip and Secure Boot, so it is inert unless
# `nixdots.machine.secureBoot` and `nixdots.encryption.tpmUnlock` are both set;
# on the UTM VM (no lanzaboote) the encryption assertion rejects it.
#
# The generic encrypted-root bits live in modules/common/encryption.nix; this
# module only adds the TPM-specific pieces, so it contributes nothing unless
# `nixdots.encryption.tpmUnlock` is set.
{ config, lib, ... }:

let
    cfg = config.nixdots.encryption;
in
{
    config = lib.mkIf (cfg.enable && cfg.tpmUnlock) {
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
        # already persisted by secure-boot.nix.
        environment.persistence."/persist" = {
            directories = [ "/var/lib/pcrlock.d" ];
            files = [ "/var/lib/systemd/pcrlock.json" ];
        };
    };
}
