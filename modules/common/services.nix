{ config, pkgs, lib, ... }:

{
    options.nixdots.ssh.enable = lib.mkEnableOption "the SSH server (keys only, persistent host keys)";

    config = {
    networking = {
        networkmanager.enable = true;
    };

    security = {
        sudo-rs = {
            enable = true;
        };

        rtkit = {
            enable = true;
        };
    };

    services = {
        bpftune.enable = true;

        openssh = lib.mkIf config.nixdots.ssh.enable {
            enable = true;
            settings = {
                PasswordAuthentication = false;
                KbdInteractiveAuthentication = false;
                PermitRootLogin = "no";
            };
            # Solo ed25519, en /persist para que no se regenere en cada arranque.
            hostKeys = [
                {
                    path = "/persist/etc/ssh/ssh_host_ed25519_key";
                    type = "ed25519";
                }
            ];
        };

        avahi = {
            # Solo para descubrir la impresora Epson; no anunciar servicios.
            enable = true;
            nssmdns4 = true;
            publish.enable = false;
        };

        pipewire = {
            enable = true;
            wireplumber.enable = true;

            alsa = {
                enable = true;

                # 32-bit ALSA (pkgsi686Linux) only exists on x86; enabling it
                # breaks evaluation on aarch64, so gate it on the host platform.
                support32Bit = pkgs.stdenv.hostPlatform.isx86_64;
            };
            jack.enable = true;
            pulse.enable = true;
        };
    };

    systemd = {
        oomd = {
            enable = true;
            enableRootSlice = true;
            enableUserSlices = true;
            enableSystemSlice = true;
            settings.OOM = {
                "DefaultMemoryPressureDurationSec" = "20s";
            };
        };
    };

    zramSwap = {
        enable = true;
        algorithm = "zstd";
        memoryPercent = 50;
    };
    };
}
