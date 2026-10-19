# Plantilla «minimal»: fish, starship, git y el tema neptunia; sin apps ni IA.
{ pkgs, lib, userData, osConfig ? { }, ... }:

let
    git = userData.git or null;

    # starship sin iconos (preset oficial `plain-text-symbols`) para la TTY, cuya
    # fuente XNU solo tiene los 256 glifos Latin-1.
    starshipPlain = pkgs.runCommand "starship-plain-text-symbols.toml" { } ''
        ${pkgs.starship}/bin/starship preset plain-text-symbols -o $out
    '';
in

{
    imports = [ ../style.nix ];

    home.stateVersion = osConfig.system.stateVersion or "26.05";

    programs = {
        fish = {
            enable = true;
            interactiveShellInit = ''
                set -g fish_greeting

                if test "$XDG_SESSION_TYPE" = tty
                    set -gx STARSHIP_CONFIG ${starshipPlain}
                end
            '';
        };

        starship.enable = true;

        git = lib.mkIf (git != null) {
            enable = true;
            settings.user = {
                name = git.name;
                email = git.email;
            };
        };
    };
}
