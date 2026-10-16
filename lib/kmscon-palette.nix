# La paleta neptunia como lista ordenada de (sufijo, "R,G,B"), compartida por
# la línea de comandos de kmscon (packages/kmscon-palette.nix) y su archivo de
# config (modules/common/console.nix). Única fuente: share/themes/neptunia.nix.
let
    theme = import ../share/themes/neptunia.nix;

    digits = {
        "0" = 0; "1" = 1; "2" = 2; "3" = 3; "4" = 4; "5" = 5; "6" = 6; "7" = 7;
        "8" = 8; "9" = 9; "a" = 10; "b" = 11; "c" = 12; "d" = 13; "e" = 14; "f" = 15;
    };
    byte = s: digits.${builtins.substring 0 1 s} * 16 + digits.${builtins.substring 1 1 s};
    rgb = c: "${toString (byte (builtins.substring 0 2 c))},${toString (byte (builtins.substring 2 2 c))},${toString (byte (builtins.substring 4 2 c))}";

    # sufijo "" => opción `palette`, el resto => `palette-<sufijo>`.
    entries = [
        { suffix = ""; value = "custom"; }
        { suffix = "black"; value = rgb theme.c0; }
        { suffix = "red"; value = rgb theme.c1; }
        { suffix = "green"; value = rgb theme.c2; }
        { suffix = "yellow"; value = rgb theme.c3; }
        { suffix = "blue"; value = rgb theme.c4; }
        { suffix = "magenta"; value = rgb theme.c5; }
        { suffix = "cyan"; value = rgb theme.c6; }
        { suffix = "light-grey"; value = rgb theme.c7; }
        { suffix = "dark-grey"; value = rgb theme.c8; }
        { suffix = "light-red"; value = rgb theme.c9; }
        { suffix = "light-green"; value = rgb theme.c10; }
        { suffix = "light-yellow"; value = rgb theme.c11; }
        { suffix = "light-blue"; value = rgb theme.c12; }
        { suffix = "light-magenta"; value = rgb theme.c13; }
        { suffix = "light-cyan"; value = rgb theme.c14; }
        { suffix = "white"; value = rgb theme.c15; }
        { suffix = "foreground"; value = rgb theme.fg; }
        # Fondo de la TTY en negro puro, no el bg neptunia (por defecto de kmscon).
        { suffix = "background"; value = "0,0,0"; }
    ];

    name = suffix: if suffix == "" then "palette" else "palette-${suffix}";
in
{
    inherit name entries rgb;
}
