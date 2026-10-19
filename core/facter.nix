# Hardware detection from a nixos-facter report. The report is generated on the
# machine (`nixos-facter` needs root) and is not versioned; it lives next to
# users data in /persist/nixdots. In a pure evaluation, or when it does not
# exist yet, the whole facter layer stays off (facterReport = null) and nothing
# below applies.
#
# This module is deliberately NOT in core/default.nix: it is imported
# only by the generic `nixdots` configuration, so the existing per-host
# configurations keep evaluating exactly as before.
{ lib, data, ... }:

let
    # lib/data.nix already checks impure + pathExists (same NIXDOTS_ROOT dance
    # as the users data): the installer evaluates with the variable pointing at the
    # target (e.g. "/mnt"); on a running system it is unset ("").
    readable = data.facterReport != null;
in
{
    hardware.facter = {
        # nixpkgs reads and parses reportPath itself (report defaults to it).
        reportPath = lib.mkIf readable data.facterReport;

        # facter would hand the network to DHCP when it sees a wired/wireless
        # NIC, but NetworkManager already manages every interface here; two
        # managers on the same link fight over the address.
        detected.dhcp.enable = lib.mkIf readable false;

        # El lector de huellas de esta máquina no funciona bien en Linux, así
        # que no se usa: fprintd/libfprint fuera aunque el informe lo detecte.
        detected.fingerprint.enable = false;
    };
}
