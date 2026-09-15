# Warning: as of February 2026 this seems to break nixos-rebuild, as uutils mv uses an interactive prompt where coreutils don't
{ pkgs, ... }:

let
  paddedName = base: pkg:
    base + builtins.concatStringsSep ""
      (builtins.genList (_: "_") (builtins.stringLength pkg.version));

  uutilsReplacement = base: old: new: {
    oldDependency = old;
    newDependency = pkgs.symlinkJoin {
      # Make the name length match so it builds
      name = paddedName base old;
      paths = [ new ];
    };
  };
in
{
  system.replaceDependencies.replacements = [
    # coreutils
    (uutilsReplacement "coreuutils-full" pkgs.coreutils-full pkgs.uutils-coreutils-noprefix) # system
    (uutilsReplacement "coreuutils" pkgs.coreutils pkgs.uutils-coreutils-noprefix) # applications
    # findutils
    (uutilsReplacement "finduutils" pkgs.findutils pkgs.uutils-findutils) # applications
    # diffutils
    (uutilsReplacement "diffuutils" pkgs.diffutils pkgs.uutils-diffutils) # applications
  ];
}