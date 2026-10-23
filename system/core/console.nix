{
  config,
  theme,
  ...
}:

let
  palette = import ../../kits/neptunia/palette.nix { inherit theme; };
in
{
  config.console = {
    earlySetup = true;
    # Los 16 colores ANSI (con el fondo negro puro) salen de kits/neptunia/palette.nix.
    colors = palette.consoleColors;
    # Lat2-Terminus16 cubre Latin-1/2 y trae los glifos que la TTY (initrd y
    # consola de emergencia) necesita.
    font = "Lat2-Terminus16";
    # The console uses the same xkb layout as the desktop.
    useXkbConfig = true;
  };
}
