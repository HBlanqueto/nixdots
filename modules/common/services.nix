{ config, pkgs, username, ... }:

{
    networking = {
        networkmanager.enable = true;
    };

    security = {
        sudo-rs = {
            enable = true;
        };

        polkit = {
            enable = true;
        };

        rtkit = {
            enable = true;
        };

        pam = {
            services = {
                login = {
                    enableGnomeKeyring = true;
                };
            };
        };
    };

    services = {
        dbus.enable = true;
        fstrim.enable = true;
        gnome.gnome-keyring.enable = true;
        gvfs.enable = true;
        openssh.enable = true;
        seatd.enable = true;

        avahi = {
            enable = true;
            publish = {
                enable = true;
                userServices = true;
            };
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
}
