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
flake.nix                 # emite nixosConfigurations.{thinkpad,utm}
settings.nix              # valores comunes (usuario, locale, git); sin system/hostName
lib/mkHost.nix            # helper: mkHost { hostName, system, modules }
hosts/
  thinkpad/{default.nix, hardware-configuration.nix}
  utm/{default.nix, hardware-configuration.nix}   # placeholder
modules/
  common/                 # usuario, fish, locale, nix, servicios y paquetes CLI -> ambos
  desktop/                # somewm, quickshell, gnome/gdm, wezterm -> ambos
  hardware/thinkpad.nix   # kernel CachyOS, amdgpu, microcode, Lanzaboote, impresora/escáner
  hardware/vm-utm.nix     # systemd-boot, virtio, spice-vdagent, qemu-guest, sin microcode
  home/common/            # HM: user, apps, style, configs
  home/desktop/           # HM: wezterm, función nswm
overlays/, packages/, share/ , .github/
```

Cada host importa `modules/hardware/<host>.nix` y su
`hardware-configuration.nix` desde `hosts/<host>/default.nix`.

## Instalar en la VM (UTM, aarch64)

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

### Gráficos y SceneFX

`somewm` enlaza SceneFX 0.5, que **requiere GLES2 en runtime**. En UTM usa
`virtio-gpu-gl` (virgl) para tener contexto GL; así los efectos (esquinas
redondeadas, sombras, blur) funcionan. Como último recurso hay un fallback
comentado en `modules/hardware/vm-utm.nix` (`WLR_RENDERER=pixman`), pero con
pixman no habrá efectos.

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
