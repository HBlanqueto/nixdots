{ inputs, ... }:

{
  # Shared NixOS layer for every host. Host-specific hardware lives under
  # modules/hardware/<host>.nix and is pulled in by hosts/<host>/default.nix.
  imports = [
    inputs.impermanence.nixosModules.impermanence
    inputs.home.nixosModules.home-manager

    ./nix.nix
    ./filesystems.nix
    ./kernel.nix
    ./encryption.nix
    ./boot.nix
    ./user.nix
    ./machine-info.nix
    ./services.nix
    ./fonts.nix
    ./uutils.nix
    ./ai.nix
    ./iphone-tethering.nix
    ./remote-access.nix
    ./home.nix
  ];
}
