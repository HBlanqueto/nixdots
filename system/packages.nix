{ pkgs, mkRustTool }:

{
  # Servicio D-Bus de gestión de cuentas.
  crewd = mkRustTool {
    pname = "nixdots-userd";
    subdir = "system/services/crewd";
    description = "D-Bus user management service for Keel";
  };

  home-init = mkRustTool {
    pname = "nixdots-home-init";
    subdir = "system/services/home-init";
    description = "First-login home setup for Keel";
    runtimeInputs = [
      pkgs.home-manager
      pkgs.systemd
      pkgs.git
      pkgs.coreutils
    ];
  };

  # Safe wrapper around nixos-rebuild (system/packages/nixdots-rebuild.*).
  nixdots-rebuild = pkgs.callPackage ./packages/nixdots-rebuild.nix { };

  # The macOS XNU console font, as PSF2 (kernel and kmscon).
  xnu-console-font = pkgs.callPackage ./packages/xnu-console-font.nix { };
}
