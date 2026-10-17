# Bootloader, driven by nixdots.machine.secureBoot.
#
#   secureBoot = true  -> lanzaboote signs the kernel/initrd with the machine's
#                         own keys (auto-generated under /var/lib/sbctl and
#                         auto-enrolled on first boot).
#   secureBoot = false -> plain systemd-boot.
#
# flake.nix importa inputs.lanzaboote.nixosModules.lanzaboote y luks-tpm.nix
# siempre en genericModules, así que boot.lanzaboote existe en todos los hosts y
# aquí basta mkIf (sin optionalAttrs ni assertions).
{ config, lib, pkgs, ... }:

let
    m = config.nixdots.machine;
in
{
    config = lib.mkMerge [
        # Secure Boot: lanzaboote owns the ESP, systemd-boot steps aside.
        (lib.mkIf m.secureBoot {
            boot.loader.systemd-boot.enable = lib.mkForce false;

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

            environment.persistence."/persist".directories =
                [ "/var/lib/sbctl" ]
                ++ lib.optional (m.encrypt && m.tpmUnlock) "/var/lib/auto-cryptenroll";

            environment.systemPackages = with pkgs; [ sbctl efibootmgr ];
        })

        # Sin Secure Boot basta con systemd-boot.
        (lib.mkIf (!m.secureBoot) {
            boot.loader.systemd-boot.enable = lib.mkDefault true;
            boot.loader.systemd-boot.configurationLimit = lib.mkDefault 5;
        })
    ];
}
