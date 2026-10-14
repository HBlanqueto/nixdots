# Warning: as of February 2026 this seems to break nixos-rebuild, as uutils mv uses an interactive prompt where coreutils don't.
{ pkgs, lib, ... }:

let
  paddedName = base: pkg:
    base + builtins.concatStringsSep ""
      (builtins.genList (_: "_") (builtins.stringLength pkg.version));

  uutilsReplacement = base: old: new: {
    oldDependency = old;
    newDependency = pkgs.symlinkJoin {
      name = paddedName base old;
      paths = [ new ];
    };
  };
in
{
  # system.replaceDependencies is implemented with import-from-derivation: it
  # builds a `references.nix` by realising (building) the old dependencies.
  # That is fine on the ThinkPad, but the aarch64 VM cannot build aarch64
  # packages from an x86 host, so cross-arch evaluation (nix eval / flake
  # check) would fail. Only apply the replacement on x86_64.
  system.replaceDependencies.replacements =
    lib.optionals pkgs.stdenv.hostPlatform.isx86_64 [
      (uutilsReplacement "coreuutils-full" pkgs.coreutils-full pkgs.uutils-coreutils-noprefix)
      (uutilsReplacement "coreuutils" pkgs.coreutils pkgs.uutils-coreutils-noprefix)
      (uutilsReplacement "finduutils" pkgs.findutils pkgs.uutils-findutils)
      (uutilsReplacement "diffuutils" pkgs.diffutils pkgs.uutils-diffutils)
    ];
}
