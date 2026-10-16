# Builds the nixosConfiguration for the single generic profile.
#
# `system` comes from the caller (builtins.currentSystem with --impure, a fixed
# value in pure evaluation), never hardcoded, so the flake can emit both an
# x86_64-linux and an aarch64-linux configuration. `hostName` is used as
# networking.hostName; `data` carries user.json/machine.json/facter.json and
# `machineData` is data.machine ({} in CI/pure evaluation).
{ inputs, theme }:
{ hostName, system, modules ? [ ], machineData ? { }, requireImpure ? true, data }:
inputs.nixpkgs.lib.nixosSystem {
    inherit system;

    # Todo llega por `data` (más inputs/theme); no hay argumentos sueltos.
    # hostName es per-config y machineData lo elige el perfil.
    specialArgs = {
        inherit inputs theme;
        data = data // { inherit hostName; machine = machineData; };
    };

    modules = [
        # The generic profile has no per-host hardware file, so derive the
        # platform from the system argument — unless facter already supplies it
        # from the report (both are mkDefault, and a string would clash with
        # facter's platform attrs).
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
                    assertion = !requireImpure || data.impure;
                    message = "nixdots necesita evaluación impura para leer /persist/nixdots/user.json: usa nixdots-rebuild o añade --impure";
                }
            ];
        })
    ] ++ modules;
}
