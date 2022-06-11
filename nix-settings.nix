{ inputs, system, nixpkgs }:

{

  checkConfig = true;

  extraOptions = ''
      experimental-features = nix-command flakes
    '';

  gc = {
      automatic = true;
      persistent = true;
      dates = "weekly";
      options = "--delete-older-than 7d";
    };

  autoOptimiseStore = true;

  optimise.automatic = true;

  settings = {
    accept-flake-config = true;
    allowed-users = [ "hblanqueto" ];
    auto-optimise-store = true;
    max-jobs = "auto";
    sandbox = false;

    trusted-users = [ "root" "hblanqueto" ];
  };

}