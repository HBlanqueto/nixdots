_:

final: _: {
  sf-mono-liga-bin = final.callPackage ../packages/sf-mono-liga-bin.nix { };

  # brave-previews is intentionally not exposed here: the input only ships
  # x86_64-linux outputs. Its overlay is applied directly in system/core/nix.nix.
}
