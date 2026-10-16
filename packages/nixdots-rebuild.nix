{
  writeShellApplication,
  nix,
  git,
  jq,
  gawk,
  coreutils,
}:

writeShellApplication {
  name = "nixdots-rebuild";
  runtimeInputs = [
    nix
    git
    jq
    gawk
    coreutils
  ];
  text = builtins.readFile ./nixdots-rebuild.sh;

  meta = {
    description = "Safe nixos-rebuild wrapper for the nixdots configuration";
    mainProgram = "nixdots-rebuild";
  };
}
