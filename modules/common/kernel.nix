{
  config,
  pkgs,
  lib,
  ...
}:

{
  options.nixdots.kernel = lib.mkOption {
    type = lib.types.enum [
      "lts"
      "latest"
      "zen"
    ];
    default = "lts";
    description = "Kernel: lts (precompilado en cache.nixos.org), latest o zen.";
  };

  # The chosen kernel; with lts nothing forces a local build. `zen` is
  # x86_64-only, but the attrset is lazy so it is only evaluated when picked.
  config.boot.kernelPackages =
    {
      lts = pkgs.linuxPackages;
      latest = pkgs.linuxPackages_latest;
      zen = pkgs.linuxPackages_zen;
    }
    .${config.nixdots.kernel};
}
