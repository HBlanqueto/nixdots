# Per-machine facts that are not committed to the repo. flake.nix reads them
# from ${NIXDOTS_ROOT}/persist/nixdots/machine.json and passes the parsed JSON
# as the `machineData` specialArg. In a pure evaluation (CI, `nix flake check`)
# machineData is { }, so every option below falls back to a safe default:
# no encryption, no Secure Boot, no extra peripherals.
{ config, lib, machineData ? { }, ... }:

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
            hostName = lib.mkDefault (machineData.hostName or "nixdots");
            encrypt = lib.mkDefault (machineData.encrypt or false);
            luksUuid = lib.mkDefault (machineData.luksUuid or "");
            secureBoot = lib.mkDefault (machineData.secureBoot or false);
            tpmUnlock = lib.mkDefault (machineData.tpmUnlock or false);
        };

        # The generic encryption module (modules/common/encryption.nix) owns the
        # cryptroot assertions; here we only feed it the machine's choice.
        nixdots.encryption = {
            enable = lib.mkDefault config.nixdots.machine.encrypt;
            tpmUnlock = lib.mkDefault config.nixdots.machine.tpmUnlock;
        };
    };
}
