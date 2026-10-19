# Un único devShell para el instalador, el asistente Slint y las herramientas
# del repo. `setup` es el CLI nixdots-setup de la misma arquitectura, que el
# asistente usa para «Aplicar» en simulación.
{
  pkgs,
  tablesEnv,
  setup,
}:
{
  default = pkgs.mkShell (
    (tablesEnv pkgs)
    // {
      packages = with pkgs; [
        cargo
        rustc
        rustfmt
        rust-analyzer
        clippy
        watchexec
        gcc
        # The released musl binaries link with clang + lld
        # (see .cargo/config.toml).
        clang
        lld
        pkg-config
        slint-viewer
        slint-lsp
        fontconfig
        freetype
        libxkbcommon
        libglvnd
        wayland
        libx11
        setup
        nixfmt
        statix
        deadnix
        nil
      ];
      # winit/femtovg cargan libwayland, libEGL… con dlopen, que no mira el
      # PATH: hacen falta en el loader path.
      LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
        with pkgs;
        [
          wayland
          libxkbcommon
          libglvnd
          fontconfig
          freetype
          libx11
        ]
      );
    }
  );
}
