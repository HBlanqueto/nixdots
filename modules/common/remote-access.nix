# Remote access: Tailscale as the private overlay (no opened ports) plus the
# desktop clients used from this machine (Remmina for VNC, RustDesk as the
# day-to-day alternative). The other end (a Mac mini without Nix) is set up
# by hand, see docs/remote-access.md.
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
    enable = lib.mkEnableOption "remote access over Tailscale, with the Remmina and RustDesk clients";

    trustTailscale = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Add tailscale0 to networking.firewall.trustedInterfaces.
        Off by default: this machine only dials out as a Tailscale
        client, so no inbound traffic has to be trusted.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    services.tailscale.enable = true;

    environment.systemPackages = clients;

    networking.firewall.trustedInterfaces = lib.mkIf cfg.trustTailscale [ "tailscale0" ];
  };
}
