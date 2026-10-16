# Remote access: the desktop clients used from this machine (Remmina for
# VNC, RustDesk as the day-to-day alternative). The other end (a Mac mini
# without Nix) is set up by hand, see docs/remote-access.md.
#
# The switch lives in modules/common/extras.nix (nixdots.extras.remoteAccess);
# this module only wires it up.
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.nixdots.extras;

  # Both clients advertise x86_64 and aarch64 in meta.platforms; the
  # availableOn filter keeps evaluation safe if either ever drops a platform.
  clients = lib.filter (lib.meta.availableOn pkgs.stdenv.hostPlatform) [
    pkgs.remmina
    pkgs.rustdesk-flutter
  ];
in
{
  config = lib.mkIf cfg.remoteAccess {
    environment.systemPackages = clients;
  };
}
