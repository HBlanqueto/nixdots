{ inputs, lib, data, theme, ... }:

lib.mkIf (data.username != null) {
    home-manager = {
        backupFileExtension = "backup";

        useGlobalPkgs = true;
        useUserPackages = true;

        # Los módulos de home-manager reciben el mismo data/theme, no campos
        # sueltos.
        extraSpecialArgs = {
            inherit inputs theme data;
        };

        users.${data.username} = {
            imports = [
                ../modules/home/common
                ../modules/home/desktop
            ];
        };
    };
}
