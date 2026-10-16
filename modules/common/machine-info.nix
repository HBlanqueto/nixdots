# The macOS-style visible name shown by `hostnamectl`, declared in the config so
# it survives the impermanence rollback. Older settings.nix files have no
# prettyHostName, in which case /etc/machine-info is left unwritten.
{ lib, data, ... }:

{
    environment.etc."machine-info" = lib.mkIf (data.settings ? prettyHostName) {
        text = ''
            PRETTY_HOSTNAME="${data.settings.prettyHostName}"
        '';
    };
}
