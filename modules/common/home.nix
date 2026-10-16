{ inputs, lib, data, theme, ... }:

lib.mkIf ((data.settings.username or null) != null) {
    home-manager = {
        backupFileExtension = "backup";

        useGlobalPkgs = true;
        useUserPackages = true;

        # Los módulos de home-manager reciben el mismo data/theme, no campos
        # sueltos.
        extraSpecialArgs = {
            inherit inputs theme data;
        };

        users.${data.settings.username} = {
            imports = [
                ../home/common
                ../home/desktop
            ];
        };
    };
}
