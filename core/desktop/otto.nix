# Sesión Otto (HBlanqueto/otto): el paquete y el módulo NixOS salen del input
# `otto` del flake (fork privado, rama neptunia). El input solo se importa si
# existe el token; así una máquina sin él (la VM UTM, una instalación nueva)
# sigue evaluando sin intentar bajar el input privado.
#
# Se activa sola en hardware real x86_64 (el backend udev de Otto usa
# DRM/Vulkan; la VM aarch64 UTM con virtio-gpu no tiene Vulkan) y machine.json
# puede forzarla con `"otto": true`/`"otto": false`, que manda sobre la
# detección.
{ config, lib, pkgs, inputs, data, ... }:

let
    cfg = config.nixdots.desktop.otto;

    tokensFile = "/persist/secrets/nix-access-tokens.conf";
    hasToken = data.exists tokensFile;

    autoEnable =
        pkgs.stdenv.hostPlatform.isx86_64
        && config.hardware.facter.enable
        && config.hardware.facter.detected.virtualisation.none.enable;
in
{
    # El módulo de Otto solo se importa con token. `lib.optional` es perezoso:
    # sin él no se toca `inputs.otto` y no se descarga el input privado.
    imports = lib.optional hasToken inputs.otto.nixosModules.default;

    options.nixdots.desktop.otto.enable =
        lib.mkEnableOption "sesión Otto (experimental)";

    config = {
        # machine.json manda; si no trae `otto`, se usa autoEnable (que ya exige
        # el token).
        nixdots.desktop.otto.enable =
            lib.mkDefault (data.machine.otto or (autoEnable && hasToken));

        assertions = [
            {
                assertion = !(data.machine.otto or false) || hasToken;
                message = ''
                    `"otto": true` en machine.json exige el token de GitHub, que
                    baja el input privado del que sale la sesión. Créalo como
                    root con permisos root:wheel 0640:
                        ${tokensFile}
                            access-tokens = github.com=<token>
                '';
            }
        ];
    }
    // lib.optionalAttrs hasToken {
        # `programs.otto` solo existe si se importó su módulo.
        programs.otto.enable = cfg.enable;

        # Skia solo es dependencia de compilación de Otto: sin esta raíz, el GC
        # semanal la borra y el siguiente build de Otto la recompila entera.
        system.extraDependencies = lib.mkIf cfg.enable [
            config.programs.otto.package.skiaBinaries
        ];
    };
}
