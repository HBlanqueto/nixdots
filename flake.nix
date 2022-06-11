{
  description = "HBlanqueto's NixOS configuration";

  inputs = {

    # Nixos inputs
    home.url = github:nix-community/home-manager;

    # Branches
    stable.url = "github:nixos/nixpkgs/nixos-22.05";
    nixos-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    # Default branch
    nixpkgs.follows = "nixpkgs-unstable";

  };
 
  outputs = { self, home, nixpkgs, ... }@inputs:
      with nixpkgs.lib;
      let
        config = {
          allowBroken = true;
          allowUnfree = true;
        }; 
      in

  {
    nixosConfigurations = {      
      laptop = import ./hosts/asus {
        inherit home config nixpkgs inputs;
      };
    };
    
    homeConfigurations = {
      myself = import ./hblanqueto {
        inherit home config nixpkgs inputs;
      };
    };

    asus-s400ca = self.nixosConfigurations.asus-s400ca.config.system.build.toplevel;

  };
}
