# /home entry point. Imports the per-user configuration subdirectory.
# The directory name must match the global username from ./settings.nix.

{ username, lib, ... }:

let
    userDir = ./humbe;
in
{
    imports = [ userDir ];

    assertions = [{
        assertion = builtins.baseNameOf (builtins.toString userDir) == username;
        message = "Rename the user configuration directory to match the username from settings.nix.";
    }];
}