# Per-machine facts that are not committed to the repo. system/lib/data.nix reads them
# from ${KEEL_ROOT}/persist/keel/machine.toml and mkHost exposes them as
# `data.machine`. In a pure evaluation (CI, `nix flake check`), or for a host
# with its own hardware file, `data.machine` is { }, so every option below falls
# back to a safe default: no encryption, no Secure Boot, no extra peripherals.
{
  config,
  lib,
  data,
  ...
}:

{
  options.keel.machine = {
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

    espUuid = lib.mkOption {
      type = lib.types.str;
      default = "";
      description = ''
        UUID of the reused Windows ESP. When set, /boot mounts it by
        UUID (a reused ESP keeps its own label, so by-label/BOOT would
        be wrong); when empty, the freshly created ESP is used.
      '';
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
    keel.machine = {
      encrypt = lib.mkDefault (data.machine.encrypt or false);
      luksUuid = lib.mkDefault (data.machine.luksUuid or "");
      espUuid = lib.mkDefault (data.machine.espUuid or "");
      secureBoot = lib.mkDefault (data.machine.secureBoot or false);
      tpmUnlock = lib.mkDefault (data.machine.tpmUnlock or false);
    };

    assertions = [
      {
        # El desbloqueo por TPM2 lo inscribe lanzaboote (measured boot),
        # que solo existe con Secure Boot.
        assertion = !config.keel.machine.tpmUnlock || config.keel.machine.secureBoot;
        message = "keel.machine.tpmUnlock requiere keel.machine.secureBoot";
      }
    ];
  };
}
