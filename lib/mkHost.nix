# Builds one nixosConfiguration per host.
#
# The host's `system` is passed in by the caller (from hosts/<host>/host.nix,
# never hardcoded), so the flake can emit both an x86_64-linux and an
# aarch64-linux configuration. `hostName` is used both as the flake output
# name and as networking.hostName, so `nixos-rebuild switch --flake ~/nixdots`
# resolves the host automatically.
{ inputs, settings }:
{ hostName, system, modules ? [ ] }:
inputs.nixpkgs.lib.nixosSystem {
    inherit system;

    specialArgs = {
        inherit inputs hostName;
        inherit (settings)
            stateVersion timeZone defaultLocale username userdescription
            hashedPasswordFile keyboardLayout keyboardVariant gitName gitEmail;
    };

    # Common + desktop layers are shared; the host supplies its own
    # hardware module and hardware-configuration.nix.
    modules = [
        ../modules/common
        ../modules/desktop
    ] ++ modules;
}
