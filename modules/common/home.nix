{ inputs, lib, hostName, username, gitName, gitEmail, stateVersion, a11y, timeZoneAuto, ... }:

let
    theme = import ../../share/themes;
in
lib.mkIf (username != null) {
    home-manager = {
        backupFileExtension = "backup";

        extraSpecialArgs = {
            inherit inputs theme hostName username gitName gitEmail stateVersion a11y timeZoneAuto;
        };

        users.${username} = {
            imports = [
                ../home/common
                ../home/desktop
            ];
        };
    };
}
