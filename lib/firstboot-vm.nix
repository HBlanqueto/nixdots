# VM de prueba del primer arranque, en ambas arquitecturas. Vive solo en el
# flake, no en la config real. Solo desktop + first-boot + user.
#
# Se construye con NIXDOTS_ROOT vacío (ver `dev.sh vm`), así el flake no ve
# ningún user.json, firstBoot = !(settings ? username) es true y arranca el
# asistente. El asistente escribe user.json, reconstruye y el reinicio ya crea
# la cuenta.
{ nixpkgs, inputs, theme, data }:
{ system, hostName }: nixpkgs.lib.nixosSystem {
    inherit system;

    specialArgs = {
        inherit inputs theme;
        # hostName es lo único per-config que los módulos necesitan además de
        # data; se inyecta en data para no tener más specialArgs sueltos.
        data = data // { inherit hostName; };
    };

    modules = [
        ../core/desktop
        # nix.nix aporta el overlay (pkgs.xnu-console-font), que console.nix y
        # user.nix usan directamente: la XNU es la única fuente de la TTY.
        ../core/nix.nix
        ../core/console.nix
        ../core/first-boot.nix
        ../core/user.nix
        # Cuentas desde users/*.toml; la VM solo comprueba el primer arranque
        # (el home es independiente).
        ../users/default.nix
        { system.stateVersion = data.settings.stateVersion; }
        # Solo en la VM de prueba: root con contraseña para poder depurar desde
        # tty2. Nunca en los hosts reales.
        { users.users.root.password = "test"; }

        # La config base ES la VM (qemu-vm.nix importado aquí, no como
        # virtualisation.vmVariant): así el `nixos-rebuild boot` interno
        # construye el mismo sistema que arranca, con sus fileSystems y su
        # bootloader.
        (nixpkgs.outPath + "/nixos/modules/virtualisation/qemu-vm.nix")

        ({ pkgs, ... }: {
            virtualisation = {
                memorySize = 4096;
                cores = 4;

                # Sin bootloader, QEMU arranca el toplevel original
                # (kernel/initrd directos) y un reinicio tras
                # `nixos-rebuild boot` no vería la nueva generación.
                useBootLoader = true;
                useEFIBoot = true;
            };

            # El bootloader que instala en la imagen de la VM.
            boot.loader.systemd-boot.enable = true;

            # El asistente reconstruye desde esta copia del repo. Solo se
            # siembra si no existe (persiste en el qcow2).
            systemd.services.nixdots-seed = {
                wantedBy = [ "multi-user.target" ];
                before = [ "greetd.service" ];
                serviceConfig.Type = "oneshot";
                path = with pkgs; [ git coreutils ];
                script = ''
                    if [ -e /persist/nixdots/repo ]; then exit 0; fi
                    mkdir -p /persist/nixdots
                    cp -a ${inputs.self} /persist/nixdots/repo
                    chmod -R u+w /persist/nixdots/repo
                    git -C /persist/nixdots/repo init -q
                    git -C /persist/nixdots/repo add -A
                    git -C /persist/nixdots/repo \
                        -c user.name=seed -c user.email=seed@localhost \
                        commit -qm seed
                '';
            };
        })
    ];
}
