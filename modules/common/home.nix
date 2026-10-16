{ inputs, lib, hostName, username, gitName, gitEmail, stateVersion, ... }:

let
    theme = import ../../share/themes;
in
lib.mkIf (username != null) {
    home-manager = {
        backupFileExtension = "backup";

        useGlobalPkgs = true;
        useUserPackages = true;

        extraSpecialArgs = {
            inherit inputs theme hostName username gitName gitEmail stateVersion;
        };

        users.${username} = {
            imports = [
                ../home/common
                ../home/desktop
            ];
        };
    };
}
