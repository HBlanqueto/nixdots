{ inputs, hostName, username, gitName, gitEmail, stateVersion, ... }:

let
    theme = import ../../share/themes;
in

{
    home-manager = {
        backupFileExtension = "backup";

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
