{
  description = "HBlanqueto's NixOS configuration";

  inputs = {

    # Nixos inputs
    home.url = github:nix-community/home-manager;

    # Branches
    nixos-unstable.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixpkgs-unstable";

    # Default branch
    nixpkgs.follows = "nixpkgs-unstable";

  };
 
  outputs = { self, home, nixpkgs, ... }:

  {
    nixosConfigurations = {      
      laptop = import ./hosts/asus-s400ca {
        inherit home nixpkgs;
      };
    };
    
    homeConfigurations = {
      myself = import ./hblanqueto {
        inherit home nixpkgs;
      };
    };

    asus-s400ca = self.nixosConfigurations.asus-s400ca.config.system.build.toplevel;
  };

}
