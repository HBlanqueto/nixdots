# Explicit opt-in for a LUKS2-encrypted root. Nothing is autodetected: the
# installer writes these options (and the LUKS device) into machine.json, so a
# machine installed without encryption simply keeps the defaults
# (`enable = false`) and is unaffected.
{ config, options, lib, ... }:

let
    cfg = config.nixdots.encryption;
in
{
    options.nixdots.encryption = {
        enable = lib.mkEnableOption "raíz cifrada con LUKS (cryptroot)";
        tpmUnlock = lib.mkEnableOption "desbloqueo automático con TPM2";
    };

    config = lib.mkIf cfg.enable {
        assertions = [
            {
                assertion = config.boot.initrd.luks.devices ? "cryptroot";
                message = "nixdots.encryption.enable está activo pero no hay boot.initrd.luks.devices.cryptroot";
            }
            {
                # `tpmUnlock` needs lanzaboote (Secure Boot + measured boot on the
                # physical machine). The VM has no lanzaboote, so asking for TPM
                # there is a configuration error. `options ? ...` avoids
                # referencing an option that may not be declared.
                assertion =
                    !cfg.tpmUnlock
                    || ((options ? boot.lanzaboote) && config.boot.lanzaboote.enable);
                message = "tpmUnlock requiere lanzaboote/TPM; este host no lo tiene";
            }
        ];

        # systemd in stage 1 is required for the encrypted root (and, with
        # tpmUnlock, for the TPM2 crypttab options and pcrlock).
        boot.initrd.systemd.enable = true;
    };
}
