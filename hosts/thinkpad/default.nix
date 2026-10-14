{ ... }:

{
  imports = [
    ./hardware-configuration.nix
    ../../modules/hardware/thinkpad.nix
  ];

  # The ThinkPad is the client side of remote access (see
  # modules/common/remote-access.nix and docs/remote-access.md); the UTM
  # VM keeps the option off.
  nixdots.remoteAccess.enable = true;
}
