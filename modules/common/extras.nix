# Optional peripherals and roles, one switch per feature. Defaults come from
# data.machine.extras (machine.json); a host may still override any of them.
# Everything is off when the data is missing (CI, pure evaluation), so the
# existing hosts are unaffected.
{ config, lib, pkgs, data, ... }:

let
    cfg = config.nixdots.extras;
    inherit (data) username;
    extraData = data.machine.extras or { };
in
{
    options.nixdots.extras = {
        epson = lib.mkEnableOption "the Epson L3250 printer/scanner";
        trezor = lib.mkEnableOption "the Trezor hardware wallet (trezord)";
        chime = lib.mkEnableOption "the boot chime";
        libvirt = lib.mkEnableOption "libvirt + virt-manager";
        brightnessctl = lib.mkEnableOption "brightnessctl and its backlight hotkey rules";
        remoteAccess = lib.mkEnableOption "the Remmina and RustDesk remote access clients";
    };

    config = {
        nixdots.extras = {
            epson = lib.mkDefault (extraData.epson or false);
            trezor = lib.mkDefault (extraData.trezor or false);
            chime = lib.mkDefault (extraData.chime or false);
            libvirt = lib.mkDefault (extraData.libvirt or false);
            brightnessctl = lib.mkDefault (extraData.brightnessctl or false);
            remoteAccess = lib.mkDefault (extraData.remoteAccess or false);
        };

        # Epson L3250: printing plus the in-tree sane backend. State under
        # /var/lib/cups survives the impermanence rollback.
        services = {
            printing = lib.mkIf cfg.epson {
                enable = true;
                drivers = [ pkgs.epson-escpr ];
            };
            trezord.enable = cfg.trezor;
            # udev rule (brightnessctl) for the panel backlight hotkeys.
            udev.packages = lib.mkIf cfg.brightnessctl [ pkgs.brightnessctl ];
        };
        hardware.sane = lib.mkIf cfg.epson {
            enable = true;
            extraBackends = [ pkgs.epsonscan2 ];
        };

        # The chime plays in the user session, so it needs an account.
        systemd.services.chime = lib.mkIf (cfg.chime && username != null) {
            description = "Boot up";
            wants = [ "sound.target" ];
            after = [ "sound.target" "home-manager-${username}.service" ];
            wantedBy = [ "multi-user.target" ];

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

        virtualisation.libvirtd.enable = cfg.libvirt;
        programs.virt-manager.enable = cfg.libvirt;
        networking.firewall.trustedInterfaces = lib.mkIf cfg.libvirt [ "virbr0" ];
        users.users = lib.mkIf (cfg.libvirt && username != null) {
            ${username}.extraGroups = [ "libvirtd" ];
        };

        environment.systemPackages =
            lib.optionals cfg.brightnessctl [ pkgs.brightnessctl ]
            ++ lib.optionals cfg.remoteAccess [
                # Clientes de escritorio: Remmina (VNC) y RustDesk. El otro
                # extremo (un Mac mini sin Nix) se configura a mano, ver
                # docs/remote-access.md.
                pkgs.remmina
                pkgs.rustdesk-flutter
            ];

        environment.persistence."/persist".directories =
            lib.optionals cfg.epson [ "/var/lib/cups" ]
            ++ lib.optionals cfg.libvirt [ "/var/lib/libvirt" ];
    };
}
