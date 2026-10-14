# Remote access: the desktop clients used from this machine (Remmina for
# VNC, RustDesk as the day-to-day alternative). The other end (a Mac mini
# without Nix) is set up by hand, see docs/remote-access.md.
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.nixdots.remoteAccess;

  # Both clients advertise x86_64 and aarch64 in meta.platforms; the
  # availableOn filter keeps evaluation safe if either ever drops a platform.
  clients = lib.filter (lib.meta.availableOn pkgs.stdenv.hostPlatform) [
    pkgs.remmina
    pkgs.rustdesk-flutter
  ];
in
{
  options.nixdots.remoteAccess = {
    enable = lib.mkEnableOption "the Remmina and RustDesk remote access clients";
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = clients;
  };
}
