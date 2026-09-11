{ inputs }:

final: prev: {
    sf-mono-liga-bin = final.callPackage ../packages/sf-mono-liga-bin.nix { inherit inputs; };

    mac-style-plymouth = inputs.mac-style.packages.${prev.stdenv.hostPlatform.system}.default;

    somewm = inputs.somewm.packages.${prev.stdenv.hostPlatform.system}.default;
}