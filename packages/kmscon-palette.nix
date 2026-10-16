# Renders the neptunia palette as kmscon `--palette*` CLI arguments, one per
# line, generated from `share/themes/neptunia.nix`. This is the single source
# of truth for the colors used by both the `nix run .#installer` launcher and
# the standalone `install.sh` bootstrap (which reads the committed file
# `share/themes/kmscon-palette`).
{ pkgs }:

let
  theme = import ../share/themes/neptunia.nix;

  # "RRGGBB" -> "R,G,B", as kmscon's custom palette wants.
  digits = {
    "0" = 0;
    "1" = 1;
    "2" = 2;
    "3" = 3;
    "4" = 4;
    "5" = 5;
    "6" = 6;
    "7" = 7;
    "8" = 8;
    "9" = 9;
    "a" = 10;
    "b" = 11;
    "c" = 12;
    "d" = 13;
    "e" = 14;
    "f" = 15;
  };
  byte = s: digits.${builtins.substring 0 1 s} * 16 + digits.${builtins.substring 1 1 s};
  rgb =
    c:
    "${toString (byte (builtins.substring 0 2 c))},${toString (byte (builtins.substring 2 2 c))},${
      toString (byte (builtins.substring 4 2 c))
    }";

  args = [
    "--palette"
    "custom"
    "--palette-black"
    (rgb theme.c0)
    "--palette-red"
    (rgb theme.c1)
    "--palette-green"
    (rgb theme.c2)
    "--palette-yellow"
    (rgb theme.c3)
    "--palette-blue"
    (rgb theme.c4)
    "--palette-magenta"
    (rgb theme.c5)
    "--palette-cyan"
    (rgb theme.c6)
    "--palette-light-grey"
    (rgb theme.c7)
    "--palette-dark-grey"
    (rgb theme.c8)
    "--palette-light-red"
    (rgb theme.c9)
    "--palette-light-green"
    (rgb theme.c10)
    "--palette-light-yellow"
    (rgb theme.c11)
    "--palette-light-blue"
    (rgb theme.c12)
    "--palette-light-magenta"
    (rgb theme.c13)
    "--palette-light-cyan"
    (rgb theme.c14)
    "--palette-white"
    (rgb theme.c15)
    "--palette-foreground"
    (rgb theme.fg)
    "--palette-background"
    (rgb theme.bg)
  ];
in
pkgs.writeText "kmscon-palette" (pkgs.lib.concatStringsSep "\n" args + "\n")
