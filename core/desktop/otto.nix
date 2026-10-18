# Sesión Otto (HBlanqueto/otto): el paquete y el módulo NixOS salen del input
# `otto` del flake (fork privado, rama neptunia), que se baja con el token de
# /persist/secrets/nix-access-tokens.conf (ver core/nix.nix). Aquí solo se
# importa su módulo y se decide si la sesión se activa.
#
# Se activa sola en hardware real x86_64 (el backend udev de Otto usa
# DRM/Vulkan; la VM aarch64 UTM con virtio-gpu no tiene Vulkan) y machine.json
# puede forzarla con `"otto": true`/`false`, que manda sobre la detección.
{ config, lib, pkgs, inputs, data, ... }:

let
    cfg = config.nixdots.desktop.otto;

    autoEnable =
        pkgs.stdenv.hostPlatform.isx86_64
        && config.hardware.facter.enable
        && config.hardware.facter.detected.virtualisation.none.enable;
in
{
    imports = [ inputs.otto.nixosModules.default ];

    options.nixdots.desktop.otto.enable =
        lib.mkEnableOption "sesión Otto (experimental)";

    config = {
        nixdots.desktop.otto.enable =
            lib.mkDefault (data.machine.otto or autoEnable);

        programs.otto.enable = cfg.enable;
    };
}
