# Renders the neptunia palette as kmscon `--palette*` CLI arguments, one per
# line, generated from `share/themes/neptunia.nix`. This is the single source
# of truth for the colors used by both the `nix run .#installer` launcher and
# the standalone `install.sh` bootstrap (which reads the committed file
# `share/themes/kmscon-palette`).
{ pkgs }:

let
    palette = import ../lib/kmscon-palette.nix;
in
pkgs.writeText "kmscon-palette"
    (pkgs.lib.concatMapStrings (e: "--${palette.name e.suffix}\n${e.value}\n") palette.entries)
