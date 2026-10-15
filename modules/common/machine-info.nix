# The macOS-style visible name shown by `hostnamectl`, declared in the config so
# it survives the impermanence rollback. Older settings.nix files have no
# prettyHostName, in which case /etc/machine-info is left unwritten.
{ lib, settings, ... }:

{
    environment.etc."machine-info" = lib.mkIf (settings ? prettyHostName) {
        text = ''
            PRETTY_HOSTNAME="${settings.prettyHostName}"
        '';
    };
}
