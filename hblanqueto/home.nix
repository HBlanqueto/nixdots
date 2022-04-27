{ config, pkgs, ... }:

{
  home = {
    username = "Humberto Blanqueto";
    homeDirectory = "/home/hblanqueto";

    packages = with pkgs; [

      #wpsoffice
      obs-studio
      neofetch

      gnumake
      #gcc
      clang
      llvm
      python3
      lua
      luarocks

    ];

    stateVersion = "22.05";
  };

  nixpkgs.config = {
    allowUnfree = true;
    allowBroken = true;
  };

  programs = {
    home-manager.enable = true;

    vscode = {
      enable = true;
      package = pkgs.vscode;
    };

    git = {
      enable = true;
      userName = "Mr. HBlanqueto";
      userEmail = "humbertoblanqueto@outlook.com";
    };
  };

}