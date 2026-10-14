{ inputs }:

final: prev: {
    sf-mono-liga-bin = final.callPackage ../packages/sf-mono-liga-bin.nix { inherit inputs; };

    mac-style-plymouth = inputs.mac-style.packages.${prev.stdenv.hostPlatform.system}.default;
    # brave-previews is intentionally not exposed here: the input only ships
    # x86_64-linux outputs, so referencing it would break aarch64 evaluation.
    # It is used directly (guarded by isx86_64) in modules/home/common/apps.nix.
}