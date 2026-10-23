{ pkgs, mkRustTool }:

{
  # Servicio D-Bus de gestión de cuentas.
  crewd = mkRustTool {
    pname = "nixdots-userd";
    subdir = "system/services/crewd";
    description = "D-Bus user management service for Keel";
  };

  # Daemon de gestión de usuarios en runtime (F0: fundaciones).
  nixos-userd = mkRustTool {
    pname = "nixos-userd";
    subdir = "system/services/userd";
    description = "Runtime user management daemon for NixOS";
  };

  # Variante con los ganchos internos (seed-job) que usa el nixosTest.
  nixos-userd-test = mkRustTool {
    pname = "nixos-userd";
    subdir = "system/services/userd";
    description = "nixos-userd con ganchos de test (solo para la VM de pruebas)";
    extra = {
      buildFeatures = [ "test-hooks" ];
    };
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
}
