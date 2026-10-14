# Hosts: ThinkPad (x86_64) y UTM (aarch64)

El mismo flake produce dos configuraciones NixOS:

| Configuración | `system`      | Equipo                                  |
|---------------|---------------|-----------------------------------------|
| `thinkpad`    | `x86_64-linux`| ThinkPad L14 Gen 2a físico (AMD, x86_64-v3) |
| `utm`         | `aarch64-linux`| VM UTM/QEMU sobre una Mac M4           |

## Selección

`networking.hostName` es igual al nombre de la configuración, así que
`nixos-rebuild` elige el host por sí solo:

```shell
nixos-rebuild switch --flake ~/nixdots          # usa $(hostname)
nixos-rebuild switch --flake ~/nixdots#utm      # explícito
```

Los alias de fish `flake-update-rb` y `flake-update-sw` usan
`--flake ~/nixdots#$(hostname)`, por lo que funcionan igual en ambos equipos.

## Estructura

```
flake.nix                 # descubre hosts/*/host.nix y emite una config por host
settings.nix              # valores comunes (usuario, locale, hashedPasswordFile, git)
lib/mkHost.nix            # helper: mkHost { hostName, system, modules }
hosts/
  thinkpad/{host.nix, default.nix, hardware-configuration.nix}
  utm/{host.nix, default.nix, hardware-configuration.nix}   # placeholder
installer/                # instalador TUI (Rust) (ver installer/README.md)
disko.nix                 # layout de disco que aplica el instalador
test.sh                   # lint + tests Rust + comprobaciones de la TUI por pty
modules/
  common/                 # usuario, fish, locale, nix, servicios, iPhone tethering -> ambos
  desktop/                # GNOME/GDM -> ambos
  hardware/thinkpad.nix   # kernel CachyOS, amdgpu, microcode, Lanzaboote, impresora/escáner
  hardware/vm-utm.nix     # systemd-boot, virtio, spice-vdagent, qemu-guest, sin microcode
  home/common/            # HM: user, apps, style, configs
  home/desktop/           # HM: wezterm
overlays/, packages/, share/, .github/
```

Cada host declara su `profile` y `system` en `hosts/<host>/host.nix`; su
`default.nix` importa `modules/hardware/<host>.nix` y el
`hardware-configuration.nix`. Añadir un host es crear un directorio con esos
tres archivos: `flake.nix` lo detecta solo (el nombre del directorio es el
nombre de la configuración y `networking.hostName`).

## Instalador

El instalador de terminal está en `installer/`:

```shell
nix run .#installer                 # simulación (sin disco ni root)
nix run .#installer -- --install    # instalación real desde la ISO
```

Detecta el perfil del equipo y aplica `disko.nix`. Ver `installer/README.md`.

## iPhone por USB (tethering)

`modules/common/iphone-tethering.nix` habilita compartir internet del iPhone por
el cable: `usbmuxd` gestiona el emparejamiento y el módulo `ipheth` expone el
teléfono como interfaz Ethernet USB que NetworkManager levanta sola. Se instala
`libimobiledevice`/`ifuse` para emparejar y acceder al dispositivo. Con el iPhone
confiado, conéctalo y activa *Punto de acceso personal*; no hace falta `ip link`
ni DHCP manual. Aplica a ambos hosts.

## Instalar en la VM (UTM, aarch64)

> El instalador (`nix run .#installer -- --install`) automatiza los pasos 2-5
> usando `disko.nix`; esta guía es la versión manual.

1. UTM -> *Create a New Virtual Machine* -> **Virtualize** -> **Linux**.
   - ISO: `nixos-minimal-*-aarch64-linux.iso`.
   - Backend **QEMU**; disco virtio; red virtio; pantalla **virtio-gpu-gl**.
   - UEFI: por defecto (edk2 aarch64).
2. Arranca el ISO y particiona (UEFI + btrfs). El mismo layout del ThinkPad
   funciona bien: ESP `/boot` + subvols `@rootfs @home @nix @persist @log`
   (y `@rootfs-blank` si quieres el rollback de impermanence, que en la VM
   está desactivado).
3. Genera el hardware config dentro de la VM:

   ```shell
   nixos-generate-config --root /mnt
   ```

4. Reemplaza el placeholder con el archivo real (cópialo al repo):

   ```shell
   cp /mnt/etc/nixos/hardware-configuration.nix ~/nixdots/hosts/utm/
   ```

   Nota: si usas impermanence, añade `neededForBoot = true;` a `/persist` y
   `/var/log` como en el ThinkPad.

5. Instala y reinicia:

   ```shell
   nixos-install --root /mnt --flake ~/nixdots#utm --impure
   ```

### Gráficos

En UTM usa `virtio-gpu-gl` (virgl) para tener contexto GL/VA-API. El escritorio
es GNOME sobre Wayland (mutter); no requiere configuración extra.

### Rosetta

`virtualisation.rosetta.enable` sólo funciona con el backend **Apple
Virtualization**, no con **QEMU** (el que usa este host). Queda comentado.

### Carpeta compartida con la Mac

Ejemplo comentado en `vm-utm.nix` (virtiofs o 9p según el dispositivo que
añadas en UTM).

## Qué queda fuera de la VM (excluido limpiamente)

- **Brave** (`brave-previews` sólo publica `x86_64-linux`) -> se usa **Firefox**;
  `BROWSER` se elige por plataforma en `modules/home/common/default.nix`.
- `onlyoffice-desktopeditors` y `epsonscan2` (sin build aarch64).
- Kernel **CachyOS x86_64-v3**, **microcode AMD** (`ucodenix`) y `amdgpu.initrd`.
- **Lanzaboote / Secure Boot** (la VM usa `systemd-boot` normal).
- Impresión y escáner Epson, `acpid`, `upower`, `blueman`/Bluetooth, `trezord`,
  `chime` (audio `plughw:1,0`), `brightnessctl`, `radeontop`, `sbctl`,
  `efibootmgr`.
- El **rollback btrfs** (`snapshot.sh` con el UUID del ThinkPad) es sólo ThinkPad;
  impermanence (`/persist`) sí se mantiene en ambos.
- `services.pipewire.alsa.support32Bit` desactivado fuera de x86_64 (usa
  `pkgsi686Linux`, que no existe en aarch64).

## Sobre `builtins.storePath` (opencode)

`modules/common/ai.nix` fija `opencode` 1.18.30 por un bug de bun usando
`builtins.storePath` (sólo x86_64). En aarch64 usa `pkgs.opencode`. Ese
`storePath` hace que `nix flake check` en modo **puro** falle; por eso hay que
evaluar con `--impure`:

```shell
nix flake check --no-build --impure
```

## Verificación

```shell
# ThinkPad: comparar con el cierre anterior
nix build --impure .#nixosConfigurations.thinkpad.config.system.build.toplevel -o result
nix store diff-closures result-before result

# UTM: evaluar sin construir (no se puede compilar aarch64 desde x86 sin emulación)
nix eval --impure .#nixosConfigurations.utm.config.system.build.toplevel.drvPath
```

Resultado esperado del diff en ThinkPad: sólo el renombrado de hostname
(`nixos-system-nixos` -> `nixos-system-thinkpad` y el archivo interno
`initrd-hostname`) y el cambio de alias solicitado; sin cambios de paquetes.
