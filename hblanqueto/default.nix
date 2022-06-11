{ nixpkgs, home, inputs, ... }:


  home.lib.homeManagerConfiguration rec {
    system = "x86_64-linux";
    username = "hblanqueto";
    homeDirectory = "/home/${username}";


    pkgs = import nixpkgs {
      system = "x86_64-linux";
      config.allowUnfree = true;
      config.allowBroken = true;
    };


    configuration.imports = [
      ./home.nix
    ];

    # Extra arguments passed to home.nix
    extraSpecialArgs = { };

    stateVersion = "22.05";
  }