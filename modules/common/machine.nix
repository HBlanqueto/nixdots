# Per-machine facts that are not committed to the repo. lib/data.nix reads them
# from ${NIXDOTS_ROOT}/persist/nixdots/machine.json and mkHost exposes them as
# `data.machine`. In a pure evaluation (CI, `nix flake check`), or for a host
# with its own hardware file, `data.machine` is { }, so every option below falls
# back to a safe default: no encryption, no Secure Boot, no extra peripherals.
{ config, lib, data, ... }:

{
    options.nixdots.machine = {
        hostName = lib.mkOption {
            type = lib.types.str;
            default = "nixdots";
            description = "networking.hostName, and the flake output name.";
        };

        encrypt = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Root (System HD) is a LUKS2 container.";
        };

        luksUuid = lib.mkOption {
            type = lib.types.str;
            default = "";
            description = "UUID of the LUKS partition (only meaningful with encrypt).";
        };

        secureBoot = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Lanzaboote with own keys (Secure Boot).";
        };

        tpmUnlock = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "TPM2 auto-unlock of the LUKS root (needs Secure Boot).";
        };
    };

    config = {
        nixdots.machine = {
            hostName = lib.mkDefault (data.machine.hostName or "nixdots");
            encrypt = lib.mkDefault (data.machine.encrypt or false);
            luksUuid = lib.mkDefault (data.machine.luksUuid or "");
            secureBoot = lib.mkDefault (data.machine.secureBoot or false);
            tpmUnlock = lib.mkDefault (data.machine.tpmUnlock or false);
        };

        # The generic encryption module (modules/common/encryption.nix) owns the
        # cryptroot assertions; here we only feed it the machine's choice.
        nixdots.encryption = {
            enable = lib.mkDefault config.nixdots.machine.encrypt;
            tpmUnlock = lib.mkDefault config.nixdots.machine.tpmUnlock;
        };
    };
}
