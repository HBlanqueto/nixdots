{ config, pkgs, username, ... }:
let
    somewmSession = pkgs.writeShellScriptBin "somewm-session" ''
        if [ -z "''${XDG_CURRENT_DESKTOP:-}" ]; then
            export XDG_CURRENT_DESKTOP=somewm
        fi
        export XDG_SESSION_TYPE=wayland

        # Advertise the desktop to D-Bus activation and the systemd user
        # manager so xdg-desktop-portal picks up somewm-portals.conf.
        ${pkgs.dbus}/bin/dbus-update-activation-environment --systemd \
            XDG_CURRENT_DESKTOP XDG_SESSION_TYPE || true

        # graphical-session.target refuses manual start, so activate a unit
        # that binds to it. xdg-desktop-portal.service has
        # Requisite=graphical-session.target and needs the target active.
        ${pkgs.systemd}/bin/systemctl --user reset-failed || true
        ${pkgs.systemd}/bin/systemctl --user start somewm-session.service || true

        exec ${pkgs.somewm}/bin/somewm "$@"
    '';
in
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
        acpid.enable = true;
        blueman.enable = true;
        bpftune.enable = true;
        dbus.enable = true;
        fstrim.enable = true;
        gnome.gnome-keyring.enable = true;
        gvfs.enable = true;
        openssh.enable = true;
        upower.enable = true;
        trezord.enable = true;

        printing = {
            enable = true;
            drivers = [ pkgs.epson-escpr ];
        };

        avahi = {
            enable = true;
            publish = {
                enable = true;
                userServices = true;
            };
        };

        xserver = {
            enable = true;
            excludePackages = [ pkgs.xterm ];
            
            xkb = {
                layout = "es";
                variant = "";
            };
        };

        displayManager = {
            gdm.enable = true;

            sessionPackages = [
                ((pkgs.writeTextFile {
                    name = "somewm-wayland-session";
                    destination = "/share/wayland-sessions/somewm.desktop";
                    text = ''
                        [Desktop Entry]
                        Name=SomeWM
                        Comment=Dynamic window manager for Wayland
                        Exec=${somewmSession}/bin/somewm-session
                        Type=Application
                        DesktopNames=somewm;
                    '';
                }).overrideAttrs (old: {
                    passthru.providedSessions = [ "somewm" ];
                }))
            ];
        };
    

        pipewire = {
            enable = true;
            alsa = {
            enable = true;
            support32Bit = true;
        };
            jack.enable = true;
            pulse.enable = true;
        };
    };

    systemd = {
        services = {

            chime = {
                enable = true;
                description = "Boot up";
                wants = ["sound.target"];
                after = [ "sound.target" "home-manager-${username}.service" ];
                wantedBy = ["multi-user.target"];

                serviceConfig = {
                    Type = "oneshot";
                    ExecStartPre = [
                    "-${pkgs.alsa-utils}/bin/amixer -c 1 sset Master 100% unmute"
                    "-${pkgs.alsa-utils}/bin/amixer -c 1 sset Speaker 100% unmute"
                ];

                    ExecStart = "${pkgs.alsa-utils}/bin/aplay -c 2 -D plughw:1,0 /home/${username}/.config/chime/chime.wav";
                    RemainAfterExit = false;
                    SupplementaryGroups = "audio";
                };
            };
        };

        user.services.somewm-session = {
            description = "somewm graphical session";
            bindsTo = [ "graphical-session.target" ];
            before = [ "graphical-session.target" ];
            wants = [ "graphical-session-pre.target" ];
            after = [ "graphical-session-pre.target" ];

            serviceConfig = {
                Type = "oneshot";
                RemainAfterExit = true;
                ExecStart = "${pkgs.coreutils}/bin/true";
            };
        };

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

    nix = {
        settings.sandbox = false;
        optimise.automatic = true;
    };
}