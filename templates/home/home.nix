# Rellena estos valores; el resto del home sale de nixdots.homeModules.
{ ... }:

{
  home.username = "USUARIO";
  home.homeDirectory = "/home/USUARIO";
  home.stateVersion = "26.05";

  # Identidad de git (nombre y correo).
  _module.args.userData = {
    username = "USUARIO";
    git = {
      name = "TU NOMBRE";
      email = "tu@correo";
    };
    a11y = { };
  };
}
