# Opciones de montaje btrfs compartidas por disko.nix (el layout del
# instalador) y system/core/filesystems.nix (el sistema instalado). Cada uno
# añade lo suyo: disko no necesita space_cache=v2 y el sistema instalado sí.
[
  "noatime"
  "compress=zstd"
]
