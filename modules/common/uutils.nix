# Warning: as of February 2026 this seems to break nixos-rebuild, as uutils mv uses an interactive prompt where coreutils don't
{ pkgs, ... }:

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
  system.replaceDependencies.replacements = [
    (uutilsReplacement "coreuutils-full" pkgs.coreutils-full pkgs.uutils-coreutils-noprefix)
    (uutilsReplacement "coreuutils" pkgs.coreutils pkgs.uutils-coreutils-noprefix)
    (uutilsReplacement "finduutils" pkgs.findutils pkgs.uutils-findutils)
    (uutilsReplacement "diffuutils" pkgs.diffutils pkgs.uutils-diffutils)
  ];
}