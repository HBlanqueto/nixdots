# Plantilla «raft»: fish, starship, git y el tema neptunia; sin apps ni IA.
{ config, pkgs, ... }:

let
  # starship sin iconos (preset oficial `plain-text-symbols`) para la TTY, que
  # no tiene glifos Nerd Font.
  starshipPlain = pkgs.runCommand "starship-plain-text-symbols.toml" { } ''
    ${pkgs.starship}/bin/starship preset plain-text-symbols -o $out
  '';
in

{
  imports = [ ../style.nix ];

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

    git = {
      enable = true;
      settings.user = {
        inherit (config.keel.git) name email;
      };
    };
  };
}
