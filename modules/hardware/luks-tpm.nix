# TPM2 auto-unlock of the LUKS2 root, via Lanzaboote's Measured Boot
# (systemd-pcrlock). ThinkPad-only: it needs a TPM2 chip and Secure Boot, so it
# is imported by modules/hardware/thinkpad.nix and never by the UTM VM.
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
      # systemd-pcrlock refuses to create a policy for more than 8 PCR4
      # variants (2 bootloader x N generations), hence the hard cap of 4.
      configurationLimit = 4;

      measuredBoot = {
        enable = true;
        pcrs = [
          0
          4
          7
        ];

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
    # already persisted by thinkpad.nix.
    environment.persistence."/persist" = {
      directories = [ "/var/lib/pcrlock.d" ];
      files = [ "/var/lib/systemd/pcrlock.json" ];
    };
  };
}
