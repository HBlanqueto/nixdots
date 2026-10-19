{ pkgs, mkRustTool, tablesEnv }:

let
  setup = mkRustTool {
    pname = "nixdots-setup";
    subdir = "welcome/cli";
    runtimeInputs = with pkgs; [
      git
      nix
      nixos-rebuild
      mkpasswd
      coreutils
    ];
    description = "First-boot assistant CLI for the Keel configuration";
    # Tablas que `localekit` embebe para validar locale y zona.
    extra = tablesEnv pkgs;
  };
in
{
  inherit setup;

  # Asistente gráfico (Slint): llama al CLI nixdots-setup.
  #
  # Licencia: Slint es GPL-3.0-only (o comercial/royalty-free). El
  # flake declara gpl2Only y GPLv2-only es incompatible con GPLv3.
  # Ver welcome/gui/Cargo.toml.
  assist-setup = mkRustTool {
    pname = "nixdots-setup-gui";
    subdir = "welcome/gui";
    description = "Graphical first-boot assistant for the Keel configuration (Slint)";
    runtimeInputs = [ setup ];
    # Tablas que `localekit` embebe (locales) para la lista
    # de idiomas del asistente.
    extra = (tablesEnv pkgs) // {
      nativeBuildInputs = [
        pkgs.pkg-config
        pkgs.makeWrapper
      ];
      buildInputs = with pkgs; [
        fontconfig
        freetype
        libxkbcommon
        libglvnd
        wayland
        libx11
      ];
    };
  };
}
