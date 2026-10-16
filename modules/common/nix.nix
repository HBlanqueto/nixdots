{ lib, inputs, stateVersion, ... }:

{
    # Impermanence roots common to both hosts. ThinkPad-only persistent dirs
    # (cups, sbctl, auto-cryptenroll) are appended by modules/hardware/thinkpad.nix.
    environment = {
        persistence."/persist" = {
            hideMounts = true;

            directories = [
                "/var/lib/bluetooth"
                "/var/lib/nixos"
                # Con la raíz efímera esto se borra en cada arranque y los
                # timers Persistent=true (gc, optimise, scrub, fstrim) pierden
                # su última ejecución: si el equipo está apagado a su hora,
                # nunca corren.
                "/var/lib/systemd/timers"
                "/var/lib/systemd/backlight"
                # secret_key de MAC aleatoria y leases DHCP.
                "/var/lib/NetworkManager"
                "/etc/NetworkManager/system-connections"
                "/var/lib/AccountsService"
                "/etc/nixos"
            ];

            files = [
                "/etc/machine-id"
            ];
        };
    };

    nix = {
        settings = {
            experimental-features = [ "nix-command" "flakes" ];
        };

        optimise.automatic = true;

        # Semanalmente recoge lo que no se use en 14 días; sin esto /nix (y las
        # generaciones que lanzaboote deja en la ESP) crecen sin límite.
        gc = {
            automatic = true;
            dates = "weekly";
            options = "--delete-older-than 14d";
        };
    };

    # Mantén vivos los sources de los inputs del flake (fuentes, temas...),
    # para que el GC semanal no obligue a re-descargarlos.
    system.extraDependencies =
        lib.mapAttrsToList (_: i: i.outPath) (lib.filterAttrs (n: _: n != "self") inputs);

    nixpkgs = {
        config = {
            allowUnfree = true;
        };
        overlays = [
            (import ../../overlays { inherit inputs; })
        ];
    };

    documentation = {
        nixos.enable = false;
        man.enable = false;
    };

    system.stateVersion = stateVersion;
}
