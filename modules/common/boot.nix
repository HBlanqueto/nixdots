{ config, pkgs, ... }:

{
    # Boot settings shared by both hosts. The bootloader lives in
    # modules/common/secure-boot.nix and the impermanence rollback in
    # modules/common/rollback.nix.
    boot = {
        loader.efi.canTouchEfiVariables = true;

        consoleLogLevel = 3;

        initrd.verbose = false;

        # Con el disco cifrado, quien llega a esa consola ya pasó la frase
        # LUKS. Sin cifrar, es un acceso de root físico: se activa a mano solo
        # para diagnosticar puntualmente.
        initrd.systemd.emergencyAccess = config.nixdots.machine.encrypt;

        plymouth = {
            enable = true;
            theme = "spinner";
            # Plymouth usa Consoleet Darwin (la fuente de consola de macOS);
            # consoleet_xnuv.otf es la variante Smooth, que no queremos.
            font = "${pkgs.consoleet-darwin}/share/fonts/opentype/consoleet_xnu.otf";
        };

        # Generic kernel parameters shared by both hosts; the AMD hardware
        # module (modules/common/hardware-detect.nix) adds
        # microcode.amd_sha_check=off on top of these, for ucodenix.
        kernelParams = [ "quiet" "rd.udev.log_level=3" "rd.systemd.show_status=auto" ];

        kernel.sysctl = {
            # Silencia el watchdog de la NMI (no aporta en este hardware).
            "kernel.nmi_watchdog" = 0;
            # Para zram: swap agresivo y sin page-cluster.
            "vm.swappiness" = 100;
            "vm.page-cluster" = 0;
            # Cola de red y control de congestión; bpftune ajusta el resto en caliente.
            "net.core.default_qdisc" = "cake";
            "net.ipv4.tcp_congestion_control" = "bbr";
        };
    };
}
