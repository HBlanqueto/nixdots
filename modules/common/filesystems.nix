# Fuente única de las opciones de montaje del sistema en marcha. Sobrescribe lo
# que nixos-generate-config deja en hardware-configuration.nix. Las opciones de
# formateo viven en disko.nix y deben coincidir con btrfsBase.
{ lib, ... }:
let
  # compress es global al btrfs (lo fija el primer montaje); space_cache=v2 y
  # discard=async ya son el comportamiento por defecto.
  btrfsBase = [
    "noatime"
    "compress=zstd"
  ];
  sub = name: extra: {
    options = lib.mkForce ([ "subvol=${name}" ] ++ btrfsBase ++ extra);
  };
in
{
  fileSystems = {
    "/" = sub "@rootfs" [ ];
    "/nix" = sub "@nix" [ ];
    "/home" = sub "@home" [
      "nodev"
      "nosuid"
    ];
    # Sin noexec: impermanence monta con bind desde aquí y heredan sus flags.
    "/persist" =
      sub "@persist" [
        "nodev"
        "nosuid"
      ]
      // {
        neededForBoot = true;
      };
    # /var/log ya es neededForBoot por defecto en NixOS.
    "/var/log" = sub "@log" [
      "nodev"
      "nosuid"
      "noexec"
    ];
    # Solo root lee el ESP (random-seed, claves de lanzaboote).
    "/boot".options = lib.mkForce [
      "fmask=0077"
      "dmask=0077"
    ];
  };

  # Un scrub cubre todo el btrfs; un solo punto de montaje basta.
  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
    fileSystems = [ "/" ];
  };
}
