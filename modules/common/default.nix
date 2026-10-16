{ inputs, ... }:

{
  # Shared NixOS layer for the single generic configuration; per-machine facts
  # come from machine.json/facter.json (see lib/data.nix).
  imports = [
    inputs.impermanence.nixosModules.impermanence
    inputs.home.nixosModules.home-manager

    ./nix.nix
    ./persistence.nix
    ./filesystems.nix
    ./kernel.nix
    ./encryption.nix
    ./machine.nix
    ./secure-boot.nix
    ./extras.nix
    ./hardware-detect.nix
    ./boot.nix
    ./user.nix
    ./machine-info.nix
    ./first-boot.nix
    ./services.nix
    ./console.nix
    ./fonts.nix
    ./ai.nix
    ./iphone-tethering.nix
    ./remote-access.nix
    ./home.nix
  ];
}
