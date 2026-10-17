_: {
    # Impermanence roots common to every machine. Persistent dirs that depend on
    # the hardware/roles (cups, sbctl, auto-cryptenroll, fwupd, libvirt…) are
    # appended by modules/common/extras.nix, secure-boot.nix, hardware-detect.nix
    # and luks-tpm.nix.
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
                # libvirt 12 cifra su clave con systemd-creds, que usa la clave
                # del equipo; con la raíz efímera se perdería en cada arranque y
                # libvirtd no arrancaría. Cualquier servicio con credenciales
                # cifradas la necesita.
                "/var/lib/systemd/credential.secret"
            ];
        };
    };
}
