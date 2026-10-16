# Builds one nixosConfiguration per host.
#
# The host's `system` is passed in by the caller (from hosts/<host>/host.nix,
# never hardcoded), so the flake can emit both an x86_64-linux and an
# aarch64-linux configuration. `hostName` is used both as the flake output
# name and as networking.hostName, so `nixos-rebuild switch --flake ~/nixdots`
# resolves the host automatically.
{ inputs, settings, firstBoot, impure }:
{ hostName, system, modules ? [ ] }:
inputs.nixpkgs.lib.nixosSystem {
    inherit system;

    specialArgs = {
        inherit inputs hostName firstBoot;
    } // import ./specialArgs.nix { inherit settings; };

    # Common + desktop layers are shared; the host supplies its own
    # hardware module and hardware-configuration.nix.
    modules = [
        ../modules/common
        ../modules/desktop
        # Sin evaluación impura no se puede leer user.json; abortar en vez de
        # tratar el host como primer arranque (que borraría la cuenta).
        ({ lib, ... }: {
            assertions = [
                {
                    assertion = impure;
                    message = "nixdots necesita evaluación impura para leer /persist/nixdots/user.json: usa nixdots-rebuild o añade --impure";
                }
            ];
        })
    ] ++ modules;
}
