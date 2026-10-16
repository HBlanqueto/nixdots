_: {
    # Impermanence roots common to both hosts. ThinkPad-only persistent dirs
    # (cups, sbctl, auto-cryptenroll, fwupd…) are appended by
    # modules/hardware/thinkpad.nix, and luks-tpm.nix adds its own.
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
}
