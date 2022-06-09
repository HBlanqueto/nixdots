{ nixpkgs, ... }:

nixpkgs.lib.nixosSystem rec {

  system = "x86_64-linux";

  modules = [
    ./configuration.nix
  ];

}
