# Hardware detection from a nixos-facter report. The report is generated on the
# machine (`nixos-facter` needs root) and is not versioned; it lives next to
# user.json in /persist/nixdots. In a pure evaluation, or when the file does not
# exist yet, the whole facter layer stays off (report = { } => enable = false)
# and nothing below applies.
#
# This module is deliberately NOT in modules/common/default.nix: it is imported
# only by the generic `nixdots` configuration, so the existing per-host
# configurations keep evaluating exactly as before.
{ lib, ... }:

let
    report = "/persist/nixdots/facter.json";

    readable = builtins ? currentSystem && builtins.pathExists report;
in
{
    hardware.facter.report = lib.mkIf readable (builtins.fromJSON (builtins.readFile report));

    # facter would hand the network to DHCP when it sees a wired/wireless NIC,
    # but NetworkManager already manages every interface here; two managers on
    # the same link fight over the address.
    hardware.facter.detected.dhcp.enable = lib.mkIf readable false;
}
