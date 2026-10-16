# Endurecimiento de los montajes del sistema en marcha. Las opciones base de
# cada sistema de archivos (device, fsType, subvol, noatime, compress, …) viven
# en modules/common/filesystems-labels.nix; aquí solo se AÑADEN flags de
# endurecimiento.
#
# NUNCA usar lib.mkForce en las options de un sistema necesario para arrancar
# (/ , /nix, /persist, /var/log): nixpkgs inyecta "x-initrd.mount" en esas
# opciones y mkForce lo borraría, dejando el initrd sin /sysroot/nix y rota
# "Find NixOS closure".
{ lib, ... }:
{
  fileSystems = {
    "/home".options = [ "nodev" "nosuid" ];
    # Sin noexec: impermanence monta con bind desde aquí y heredan sus flags.
    "/persist" = {
      neededForBoot = true;
      options = [ "nodev" "nosuid" ];
    };
    # /var/log ya es neededForBoot por defecto en NixOS.
    "/var/log".options = [ "nodev" "nosuid" "noexec" ];
    # Solo root lee el ESP (random-seed, claves de lanzaboote). El ESP no se
    # monta en el initrd, así que aquí mkForce sí es seguro.
    "/boot".options = lib.mkForce [ "fmask=0077" "dmask=0077" ];
  };

  # Un scrub cubre todo el btrfs; un solo punto de montaje basta.
  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
    fileSystems = [ "/" ];
  };
}
