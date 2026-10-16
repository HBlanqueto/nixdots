{ config, pkgs, lib, inputs, username, ... }:

{
    # Everything that depends on this being the physical ThinkPad L14 Gen 2a:
    # AMD Ryzen 5650U (x86_64-v3), Radeon graphics, laptop power/audio, the
    # Epson printer/scanner and zero-touch Secure Boot. None of it is valid on
    # the aarch64 VM, so it stays out of modules/common.
    imports = [
        inputs.lanzaboote.nixosModules.lanzaboote
        inputs.ucodenix.nixosModules.default
        ./luks-tpm.nix
    ];

    boot = {
        # Lanzaboote replaces systemd-boot for Secure Boot.
        loader.systemd-boot.enable = lib.mkForce false;

        lanzaboote = {
            enable = true;
            pkiBundle = "/var/lib/sbctl";

            # Cap what lands on the 511 MiB ESP. 4 also keeps the pcrlock PCR4
            # variants (2 bootloader stubs x N generations) under the hard limit
            # of 8.
            configurationLimit = 4;

            autoGenerateKeys = {
                enable = true;
            };

            autoEnrollKeys = {
                enable = true;
                autoReboot = false;
            };
        };

        # Btrfs subvolume rollback (impermanence). The script mounts the
        # ThinkPad's top-level subvolume, so it is ThinkPad-only.
        initrd.systemd = {
            initrdBin = with pkgs; [ coreutils btrfs-progs findutils util-linux gawk e2fsprogs ];

            services.rollback = {
                wantedBy = [ "initrd.target" ];
                before = [ "sysroot.mount" ];
                # El label nixos solo aparece tras abrir LUKS (si está cifrado), así que
                # esperar al dispositivo cubre ambos casos. NO pongas requires sobre
                # cryptsetup: el cifrado es opcional y sin LUKS esa unidad no existe.
                requires = [ "dev-disk-by\\x2dlabel-nixos.device" ];
                after = [
                    "dev-disk-by\\x2dlabel-nixos.device"
                    "systemd-cryptsetup@cryptroot.service"
                ];
                unitConfig.DefaultDependencies = "no";
                serviceConfig.Type = "oneshot";
                script = builtins.readFile ./snapshot.sh;
            };
        };

        kernelParams = [ "microcode.amd_sha_check=off" ];
    };

    # Persistent dirs that only exist on the physical machine.
    environment.persistence."/persist".directories = [
        "/var/lib/sbctl"
        "/var/lib/auto-cryptenroll"
        "/var/lib/cups"
        "/var/lib/libvirt"
    ];

    virtualisation.libvirtd = {
        enable = true;
    };

    # virt-manager drives libvirtd from the GNOME session.
    programs.virt-manager.enable = true;

    users.users = lib.mkIf (username != null) {
        ${username}.extraGroups = [ "libvirtd" ];
    };

    # The default libvirt NAT network lives on virbr0.
    networking.firewall.trustedInterfaces = [ "virbr0" ];

    services = {
        ucodenix = {
            enable = true;
        };

        acpid.enable = true;
        blueman.enable = true;
        upower.enable = true;
        trezord.enable = true;

        # brightnessctl reacts to the panel backlight hotkeys.
        udev.packages = [
            pkgs.brightnessctl
        ];

        printing = {
            enable = true;
            drivers = [ pkgs.epson-escpr ];
        };
    };

    # El chime suena en la sesión del usuario, así que solo aplica con cuenta.
    systemd.services.chime = lib.mkIf (username != null) {
        enable = true;
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

    hardware = {
        enableRedistributableFirmware = true;

        # AMD GPU: early KMS plus the open-source firmware/microcode stack.
        amdgpu.initrd.enable = true;

        bluetooth.enable = true;

        # Physical Epson L3250 scanner; epsonscan2 has no aarch64 build anyway.
        sane = {
            enable = true;
            extraBackends = [ pkgs.epsonscan2 ];
        };
    };

    # AMD VA-API (radeonsi ships in mesa).
    environment.sessionVariables = {
        LIBVA_DRIVER_NAME = "radeonsi";
    };
    hardware.graphics.extraPackages = [ pkgs.mesa ];

    environment.systemPackages = with pkgs; [
        sbctl
        efibootmgr
        brightnessctl
        radeontop
    ];
}
