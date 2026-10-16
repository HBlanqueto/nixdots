# Bootloader, driven by nixdots.machine.secureBoot.
#
#   secureBoot = true  -> lanzaboote signs the kernel/initrd with the machine's
#                         own keys (auto-generated under /var/lib/sbctl and
#                         auto-enrolled on first boot).
#   secureBoot = false -> plain systemd-boot.
#
# This module does NOT import lanzaboote itself (imports cannot depend on
# `config`). Whoever sets secureBoot = true must import
# inputs.lanzaboote.nixosModules.lanzaboote: flake.nix does it for the generic
# `nixdots` output whenever machine.json asks for Secure Boot. The boot.lanzaboote
# definitions are only emitted when that option actually exists, so hosts without
# lanzaboote evaluate fine.
{ config, options, lib, pkgs, ... }:

let
    m = config.nixdots.machine;
    hasLanzaboote = options ? boot.lanzaboote;
in
{
    config = lib.mkMerge [
        {
            assertions = [
                {
                    assertion = !m.secureBoot || hasLanzaboote;
                    message = "nixdots.machine.secureBoot = true requiere importar inputs.lanzaboote.nixosModules.lanzaboote";
                }
            ];
        }

        # Secure Boot: lanzaboote owns the ESP, systemd-boot steps aside. The
        # boot.lanzaboote key is only emitted when the module is present
        # (optionalAttrs, not mkIf: a definition for a nonexistent option errors
        # even when its condition is false).
        (lib.mkIf m.secureBoot (
            {
                boot.loader.systemd-boot.enable = lib.mkForce false;

                environment.persistence."/persist".directories =
                    [ "/var/lib/sbctl" ]
                    ++ lib.optional (m.encrypt && m.tpmUnlock) "/var/lib/auto-cryptenroll";

                environment.systemPackages = with pkgs; [ sbctl efibootmgr ];
            }
            // lib.optionalAttrs hasLanzaboote {
                boot.lanzaboote = {
                    enable = true;
                    pkiBundle = "/var/lib/sbctl";

                    # Cap what lands on the (511 MiB) ESP. 4 also keeps the pcrlock PCR4
                    # variants (2 bootloader stubs x N generations) under the hard limit
                    # of 8.
                    configurationLimit = 4;

                    autoGenerateKeys.enable = true;

                    autoEnrollKeys = {
                        enable = true;
                        autoReboot = false;
                    };
                };
            }
        ))

        # Sin Secure Boot basta con systemd-boot.
        (lib.mkIf (!m.secureBoot) {
            boot.loader.systemd-boot.enable = lib.mkDefault true;
            boot.loader.systemd-boot.configurationLimit = lib.mkDefault 5;
        })
    ];
}
