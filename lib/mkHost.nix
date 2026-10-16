# Builds one nixosConfiguration per host.
#
# The host's `system` is passed in by the caller (from hosts/<host>/host.nix,
# never hardcoded), so the flake can emit both an x86_64-linux and an
# aarch64-linux configuration. `hostName` is used both as the flake output
# name and as networking.hostName, so `nixos-rebuild switch --flake ~/nixdots`
# resolves the host automatically.
{ inputs, settings, firstBoot, impure }:
{ hostName, system, modules ? [ ], machineData ? { }, requireImpure ? true }:
inputs.nixpkgs.lib.nixosSystem {
    inherit system;

    specialArgs = {
        inherit inputs hostName firstBoot machineData;
    } // import ./specialArgs.nix { inherit settings; };

    # Common + desktop layers are shared; the host supplies its own
    # hardware module and hardware-configuration.nix.
    modules = [
        # Per-host hardware-configuration.nix sets this too. The generic
        # profile has no such file, so derive it from the system argument —
        # unless facter already supplies it from the report (both are
        # mkDefault, and a string would clash with facter's platform attrs).
        ({ config, lib, ... }: {
            nixpkgs.hostPlatform =
                lib.mkIf (!config.hardware.facter.enable) (lib.mkDefault system);
        })
        ../modules/common
        ../modules/desktop
        # Sin evaluación impura no se puede leer user.json; abortar en vez de
        # tratar el host como primer arranque (que borraría la cuenta). Las
        # salidas puras de CI (requireImpure = false) se saltan la assertion: no
        # activan la cuenta ni el cifrado, solo sirven para `nix flake check`.
        (_: {
            assertions = [
                {
                    assertion = !requireImpure || impure;
                    message = "nixdots necesita evaluación impura para leer /persist/nixdots/user.json: usa nixdots-rebuild o añade --impure";
                }
            ];
        })
    ] ++ modules;
}
