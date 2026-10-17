# TPM2 auto-unlock of the LUKS2 root, via Lanzaboote's Measured Boot
# (systemd-pcrlock). It needs a TPM2 chip and Secure Boot, so it is inert unless
# `nixdots.machine.secureBoot` and `nixdots.machine.tpmUnlock` are both set (the
# latter requires the former; machine.nix asserts it).
#
# lanzaboote's autoCryptenroll runs last: it unlocks with
# `--unlock-tpm2-device=auto`, wipes the TPM2 slot and re-enrolls the pcrlock
# policy, which systemd-pcrlock keeps up to date on every boot and update. For
# that first unlock to work it needs a token already enrolled, so the installer
# enrolls a bridge token with no PCRs (`--tpm2-pcrs=`); a static `--tpm2-pcrs=7`
# would not do, because PCR7 changes as soon as lanzaboote enrolls the Secure
# Boot keys. Until autoCryptenroll replaces it (second boot), that bridge token
# is not bound to any measurement: anyone with the disk and the same TPM can
# unlock.
{ config, lib, pkgs, ... }:

let
    m = config.nixdots.machine;

    # The LUKS2 partition holding the root.
    device = config.boot.initrd.luks.devices."cryptroot".device;

    # Message shown in the journal and notified on the desktop when the bridge
    # token survived: the last auto-cryptenroll did not replace it.
    warning = "El desbloqueo con TPM no se completó; el disco está protegido solo por la entrada temporal. Ejecuta nixdots-rebuild o revisa auto-cryptenroll.";

    # Succeeds when a systemd-tpm2 token with no policy is still enrolled: the
    # installer's bridge token, which auto-cryptenroll should have replaced. Its
    # luksDump block carries neither PCRs nor a pcrlock policy. The awk reads the
    # human-readable dump: tokens start with `N: systemd-tpm2`, fields are
    # indented under them. It never writes to the disk.
    tpmBridgeLeft = pkgs.writeShellScript "nixdots-tpm-check" ''
        ${pkgs.cryptsetup}/bin/cryptsetup \
            --external-tokens-path=${config.systemd.package}/lib/cryptsetup \
            luksDump ${lib.escapeShellArg device} | ${pkgs.gawk}/bin/awk '
                function flush() { if (inblock && !has) bad=1 }
                /^[[:space:]]*[0-9]+: systemd-tpm2/ { flush(); inblock=1; has=0; next }
                /^[[:space:]]*[0-9]+:/ { flush(); inblock=0 }
                inblock {
                    if ($0 ~ /tpm2-pcrlock:[[:space:]]*yes/) has=1
                    else if ($0 ~ /tpm2-hash-pcrs:[[:space:]]*[0-9]/) has=1
                    else if ($0 ~ /tpm2-pubkey-pcrs:[[:space:]]*[0-9]/) has=1
                }
                END { flush(); exit bad ? 0 : 1 }
            '
    '';

    # Warns the user at graphical login if the check left a warning file.
    tpmWarn = pkgs.writeShellScript "nixdots-tpm-warning" ''
        if [ -e /run/nixdots-tpm/warning ]; then
            ${pkgs.libnotify}/bin/notify-send --urgency=critical --icon=dialog-warning \
                "Desbloqueo con TPM incompleto" ${lib.escapeShellArg warning}
        fi
    '';
in
{
    config = lib.mkIf (m.encrypt && m.tpmUnlock) {
        boot.lanzaboote = {
            measuredBoot = {
                enable = true;
                pcrs = [ 0 4 7 ];

                autoCryptenroll = {
                    enable = true;
                    inherit device;
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

        # Safety net: auto-cryptenroll should have wiped the bridge token and
        # enrolled the pcrlock policy. If a policy-less systemd-tpm2 token is
        # still there, log it and leave a flag for the login notification. It
        # never deletes anything by itself.
        systemd.services.nixdots-tpm-check = {
            description = "Comprueba que auto-cryptenroll sustituyó el token TPM puente";
            after = [ "auto-cryptenroll.service" ];
            wants = [ "auto-cryptenroll.service" ];
            wantedBy = [ "multi-user.target" ];
            serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
                RuntimeDirectory = "nixdots-tpm";
                RuntimeDirectoryMode = "0755";
            };
            script = ''
                if ${tpmBridgeLeft}; then
                    printf '%s\n' ${lib.escapeShellArg warning} \
                        | ${config.systemd.package}/bin/systemd-cat -t nixdots-tpm-check -p err
                    printf '%s\n' ${lib.escapeShellArg warning} > "$RUNTIME_DIRECTORY/warning"
                fi
            '';
        };

        # GNOME notification, at graphical login, when the check flagged.
        systemd.user.services.nixdots-tpm-warning = {
            description = "Avisa si el token TPM puente sigue activo";
            after = [ "graphical-session.target" ];
            wantedBy = [ "graphical-session.target" ];
            serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
                ExecStart = tpmWarn;
            };
        };
    };
}
