{ inputs, ... }:

{
  # Shared NixOS layer for the single generic configuration; per-machine facts
  # come from machine.json/facter.json (see lib/data.nix).
  imports = [
    inputs.impermanence.nixosModules.impermanence

    ./nix.nix
    ./persistence.nix
    ./machine.nix
    ./secure-boot.nix
    ./extras.nix
    ./hardware-detect.nix
    ./boot.nix
    ./user.nix
    ./first-boot.nix
    ./services.nix
    ./console.nix
    ./fonts.nix
    ./iphone-tethering.nix
  ];
}
