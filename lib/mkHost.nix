# Builds the nixosConfiguration for the single generic profile.
#
# `system` comes from the caller (builtins.currentSystem with --impure, a fixed
# value in pure evaluation), never hardcoded, so the flake can emit both an
# x86_64-linux and an aarch64-linux configuration. `data` carries
# users/*.toml/machine.toml/facter.json; hostName sale de data.machine.
{ inputs, theme }:
{ system, modules ? [ ], requireImpure ? true, data }:
inputs.nixpkgs.lib.nixosSystem {
    inherit system;

    # Todo llega por `data` (más inputs/theme); no hay argumentos sueltos. El
    # hostName por defecto cubre CI y evaluación pura (sin machine.toml).
    specialArgs = {
        inherit inputs theme;
        data = data // { hostName = data.machine.hostName or "keel"; };
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
        ../core
        ../core/desktop
        ../users
        # Sin evaluación impura no se pueden leer las cuentas; abortar en vez de
        # tratar el host como primer arranque (que borraría la cuenta). Las
        # salidas puras de CI (requireImpure = false) se saltan la assertion: no
        # activan la cuenta ni el cifrado, solo sirven para `nix flake check`.
        (_: {
            assertions = [
                {
                    assertion = !requireImpure || data.impure;
                    message = "keel necesita evaluación impura para leer /persist/keel: usa steward rebuild o añade --impure";
                }
                {
                    assertion = data.usersValid;
                    message = data.usersError;
                }
            ];
        })
    ] ++ modules;
}
