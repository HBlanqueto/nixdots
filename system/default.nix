{ inputs, theme }:

{
  mkHost = import ./lib/mkHost.nix { inherit inputs theme; };

  # Módulos exclusivos del perfil único (asumen el layout de disko y/o un
  # informe de facter). lanzaboote y luks-tpm se importan siempre:
  # secure-boot.nix decide con mkIf y luks-tpm define boot.lanzaboote.
  genericModules = [
    ./core/facter.nix
    ./core/filesystems.nix
    ./core/rollback.nix
    # Igual que lanzaboote: el módulo se importa siempre y
    # hardware-detect.nix activa ucodenix solo si el informe de facter
    # ve una CPU AMD en bare-metal.
    inputs.ucodenix.nixosModules.default
    inputs.lanzaboote.nixosModules.lanzaboote
    ./core/luks-tpm.nix
  ];

  homeModules = import ./users/templates;
}
