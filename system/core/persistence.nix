{ config, ... }:
{
  # Impermanence roots common to every machine. Persistent dirs that depend on
  # the hardware/roles (cups, sbctl, auto-cryptenroll, fwupd, libvirt…) are
  # appended by system/core/extras.nix, secure-boot.nix, hardware-detect.nix
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
        # Perfiles de energía elegidos por el usuario.
        "/var/lib/power-profiles-daemon"
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
        #
        # systemd NO sigue enlaces simbólicos para esta clave, así que
        # debe ser un archivo real (bind mount). Impermanence
        # (rev 7b1d382) bind-montea los `files` con `method = "auto"`
        # solo si el destino ya existe; el script de abajo lo crea antes.
        "/var/lib/systemd/credential.secret"
      ];
    };
  };

  system.activationScripts.keelCredentialSecret = {
    deps = [ "specialfs" ];
    text = ''
      # Clave de systemd-creds: archivo real, nunca symlink. systemd
      # (261.3) exige un archivo regular de 4112 bytes (16 de machine-id
      # hasheado + 4096 de clave), modo exacto 0400, y lo valida al leer
      # (incluido que el machine-id coincida); si no, lo rechaza o borra.
      # Por eso la genera `systemd-creds setup`, no nosotros.
      if [ -L /var/lib/systemd/credential.secret ]; then
          rm -f /var/lib/systemd/credential.secret
      fi
      if [ ! -e /persist/var/lib/systemd/credential.secret ]; then
          mkdir -p /persist/var/lib/systemd
          SYSTEMD_CREDENTIAL_SECRET=/persist/var/lib/systemd/credential.secret \
              ${config.systemd.package}/bin/systemd-creds setup
      fi
    '';
  };

  # `persist-files` (impermanence) corre después: al existir ya el destino,
  # `method = "auto"` hace bind mount en vez de symlink.
  system.activationScripts.persist-files.deps = [ "keelCredentialSecret" ];
}
