# The macOS-style visible name shown by `hostnamectl`, declared in the config so
# it survives the impermanence rollback. Older settings.nix files have no
# prettyHostName, in which case /etc/machine-info is left unwritten.
{ lib, data, ... }:

let
    # El nombre visible puede traer `\` o `"`; hay que escaparlos antes de
    # escribirlo entre comillas en /etc/machine-info.
    pretty = data.settings.prettyHostName;
    escaped = lib.replaceStrings [ "\\" "\"" ] [ "\\\\" "\\\"" ] pretty;
in

{
    environment.etc."machine-info" = lib.mkIf (data.settings ? prettyHostName) {
        text = ''
            PRETTY_HOSTNAME="${escaped}"
        '';
    };
}
