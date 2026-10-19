{
  lib,
  inputs,
  data,
  ...
}:

{
  nix = {
    settings = {
      experimental-features = [
        "nix-command"
        "flakes"
      ];

      # Todos los usuarios necesitan el daemon para home-manager (el home
      # ya no lo aplica el sistema). trusted-users se queda en root.
      allowed-users = [ "@users" ];
    };

    # Solo flakes: sin nix-channel. NIX_PATH y el registro apuntan al nixpkgs
    # del flake (nixpkgs.flake.*, activo por defecto), así `nix-shell -p` y
    # `,` usan la misma revisión que el sistema.
    channel.enable = false;

    optimise.automatic = true;

    # Semanalmente recoge lo que no se use en 14 días; sin esto /nix (y las
    # generaciones que lanzaboote deja en la ESP) crecen sin límite.
    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 14d";
    };
  };

  # Mantén vivos los sources de los inputs del flake (fuentes, temas...),
  # para que el GC semanal no obligue a re-descargarlos.
  system.extraDependencies = lib.mapAttrsToList (_: i: i.outPath) (
    lib.filterAttrs (n: _: n != "self") inputs
  );

  nixpkgs = {
    config = {
      allowUnfree = true;
    };
    overlays = [
      (import ../overlays { inherit inputs; })
      # brave-previews publica brave-origin-beta usando `xorg.libxcb`, que
      # nixpkgs renombró a `libxcb`. Aplicamos su overlay a nuestro pkgs
      # (así el paquete se evalúa con nuestros overlays) y reescribimos el
      # alias deprecado para no emitir el aviso.
      inputs.brave-previews.overlays.default
      (_final: prev: {
        xorg = prev.xorg // {
          inherit (prev) libxcb;
        };
      })
    ];
  };

  documentation = {
    nixos.enable = false;
    man.enable = false;
  };

  system.stateVersion = data.settings.stateVersion;
}
