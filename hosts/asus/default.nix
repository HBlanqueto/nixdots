{ config, nixpkgs, inputs, ... }:


nixpkgs.lib.nixosSystem rec {
  system = "x86_64-linux";

  modules = [

    {
      nix = import ../../nix-settings.nix { inherit inputs system nixpkgs; };
      nixpkgs = { inherit config; };
    }

    ./configuration.nix
  ];

  specialArgs = { inherit inputs; };
}