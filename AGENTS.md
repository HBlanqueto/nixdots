# AGENTS.md — nixdots

Guía para agentes (opencode, Claude…) que trabajan en este repo. Describe el
estado **actual**; si algo de aquí contradice el código, manda el código y
actualiza este archivo en el mismo cambio.

## Forma de trabajar

- Aplica los cambios directamente. No escribas tests nuevos ni pidas
  verificaciones salvo que se pida. Si eliminas o renombras una función, elimina
  o adapta sus tests existentes para que el workspace siga compilando.
- Commits pequeños y lógicos con conventional commits
  (`feat(installer): …`, `fix(console): …`, `refactor(common): …`).
- No cambies `stateVersion` (settings.nix, "26.05").
- No añadas dependencias pesadas (inputs, crates, servicios) sin justificarlo en
  el commit.
- Si algo es ambiguo, elige la opción más simple y anótalo en el resumen final.
- Al terminar, resume qué cambiaste, por qué y qué queda pendiente.
- Idioma: comentarios, mensajes de la interfaz y documentación en español.

## Arquitectura (no volver atrás)

- **Una sola configuración**: `nixosConfigurations.nixdots`. No hay `hosts/`.
  La arquitectura sale de `builtins.currentSystem` y el hardware del informe de
  **nixos-facter**. Por eso la evaluación real exige `--impure`; `nixdots-rebuild`
  lo añade. La salida también se expone con el hostname de la máquina.
- Variantes puras solo para CI: `nixdots-x86_64-linux` y
  `nixdots-aarch64-linux` (no leen /persist). VMs de prueba del primer
  arranque: `firstboot-vm` y `firstboot-vm-aarch64` (`run-firstboot-vm.sh`).
- **Datos fuera del repo**, en `/persist/nixdots/`:
  - `machine.json` — el archivo del sistema. Lo escriben el instalador
    (hostName, encrypt, luksUuid, espUuid, secureBoot, tpmUnlock, extras) y el
    asistente (prettyHostName, defaultLocale, region, timeZone, timeZoneAuto,
    keyboardLayout/Variant, autoLoginUser); gana el último en escribir. La
    escritura fusiona sobre el JSON existente. `espUuid` es el UUID de la ESP de
    Windows reutilizada (vacío si se crea nueva); `/boot` se monta por ese UUID
    en ese caso, y por label `BOOT` en el resto.
  - `facter.json` — informe de hardware.
  - `users/<usuario>.json` — una cuenta por archivo, lo escribe el asistente
    (username, fullName, admin, template, personalConfig, git, colorScheme,
    accentColor y a11y). El nombre del archivo debe coincidir con `username`.
  - `users/<usuario>/home.nix` — config personal opcional, se importa encima de
    la plantilla si `personalConfig` y el usuario es admin. Debe ser de **root**:
    el rebuild evalúa como root y el archivo podría leer `/persist/secrets`.
  - `user.json` — **respaldo transitorio**: si no hay `users/*.json`, `data.nix`
    lo lee como un único usuario `admin = true` y `template = "nixdots"`.
    `nixdots-setup migrate` lo convierte a `users/<usuario>.json` + machine.json
    y deja `user.json.bak`; el respaldo se retira en un commit aparte y al final.
  - `/persist/secrets/<usuario>.hash` — hash de la contraseña
    (`hashedPasswordFile`), en un directorio `0711` y el archivo `0600`, ambos
    de root. Nunca hashes ni secretos en el repo.
  - `/persist/secrets/nix-access-tokens.conf` — root:wheel `0640`, opcional:
    `access-tokens = github.com=<token>`. Hoy no hay inputs privados; el archivo
    se conserva por si vuelven.
- `lib/data.nix` es el **único** punto que lee esos archivos (getEnv,
  pathExists, readFile). Los módulos consumen `data` por specialArgs.
- `settings.nix` solo tiene valores independientes de la máquina (stateVersion,
  zona, locale, teclado por defecto).
- Diferencias de hardware: `core/hardware-detect.nix` decide con
  `config.hardware.facter.detected/report` (bare-metal AMD → ucodenix y amdgpu
  en initrd; portátil → acpid/upower; QEMU/UTM → spice-vdagent, qemu-guest).
  No uses condicionales por nombre de host.
- Opciones propias bajo `options.nixdots.*` (`machine`, `console`,
  `firstBoot`…). `nixdots.machine` (machine.json) es la única capa de cifrado y
  Secure Boot. El instalador inscribe un token TPM2 puente sin PCRs
  (`--tpm2-pcrs=`) para que lanzaboote (`measuredBoot.autoCryptenroll`, con
  `systemd-pcrlock`) pueda desbloquear y reinscribir en el primer arranque la
  política atada a las PCR 0 4 7, que se actualiza sola; hasta ese arranque el
  token puente no está atado a ninguna medida.
- Kernel: es el de nixpkgs por defecto (LTS). Para cambiarlo, `boot.kernelPackages`
  en un módulo (no hay `nixdots.kernel`).
- Impermanence: la raíz se restaura en cada arranque (Btrfs `@rootfs` desde
  `@rootfs-blank`). Todo estado que deba sobrevivir se declara en
  `environment.persistence."/persist"`.

## Componentes

- Tres capas:
  - `core/` — sistema (hardware, boot, consola, nix, seguridad,
    filesystems/rollback/persistence, servicios, GNOME, fuentes, extras y
    first-boot). No referencia a ningún usuario concreto ni rutas de `/home`.
  - `users/` — cuentas: `users/default.nix` lee `data.users` y genera
    `users.users.<nombre>` (`hashedPasswordFile` derivado, `wheel` solo si
    `admin`) y cablea home-manager desde `users/templates/<template>`.
    `nixdots.users.homeManager.enable` (default true) lo desactiva en la VM de
    primer arranque.
  - `users/templates/` — `nixdots` (home completo: apps, IA, escritorio) y
    `minimal` (fish, starship, git y el tema). Reciben `userData` (su propio
    usuario) por `_module.args`; `stateVersion` y `timeZoneAuto` salen de
    `osConfig`. El personal opcional va en `/persist/nixdots/users/<nombre>/home.nix`.
- `modules/hardware/snapshot.sh` (ayudante de rollback) sigue en `modules/`.
- Escritorio: **solo GNOME**. SomeWM y Quickshell se quitaron; no los
  reintroduzcas sin que se pida.
- Otto (fork `github:HBlanqueto/otto`, con `layers` y `smithay-otto`) salió del
  sistema y solo se conserva como referencia externa, fuera de este repo: no lo
  reintroduzcas sin que se pida.
- Consola: la TTY instalada usa **solo** la fuente XNU (kmscon con el motor
  `psf`, `font-name` = `xnu-console.psf`, tamaño 32 —al doble—, y `console.font`
  = `xnu-console.psf` en el kernel: initrd y consola de emergencia). Solo tiene
  256 glifos Latin-1, así que en la TTY el prompt de fish (starship) y `eza` van
  sin iconos Nerd Font, y los caracteres fuera de Latin-1 salen como «?». El
  lanzador del instalador sí abre kmscon con JetBrainsMono Nerd Font Mono 14 y la
  paleta neptunia. Plymouth usa el tema integrado `spinner` con la fuente
  Consoleet Darwin (OTF, `pkgs.consoleet-darwin`).
- Workspace Rust (`Cargo.toml` raíz); los crates viven en `tools/`:
  - `tools/nixdots-common` — settings, users/<usuario>.json, machine.json, hash,
    ejecución de comandos.
  - `tools/lists` — locales, zonas y teclados embebidos al compilar.
  - `tools/installer` — TUI ratatui (`nixdots-installer`). Solo instala el sistema;
    pantallas: Inicio · Licencia · Red · Lenguaje · Teclado · Zona horaria ·
    Equipo · Disco · Cifrado · Resumen · Instalando · Listo. No pide usuario,
    contraseña ni Git, ni escribe `users/*.json` ni el hash (eso lo hace el
    asistente). La frase de LUKS es siempre propia. Sin soporte de ratón.
  - `tools/setup` — CLI del primer arranque (`nixdots-setup apply`) y de
    migración (`nixdots-setup migrate`).
  - `tools/setup-gui` — asistente gráfico «Hola» en **Slint** (no GTK/libadwaita).
- El instalador orquesta herramientas existentes (disko, nixos-facter, sbctl,
  nixos-install…); nunca reimplementa particionado. Los binarios se publican en
  la release `latest` desde `.github/workflows/installer-release.yml`;
  `install.sh` los descarga.

## Tema

- Dos fuentes, cada una para lo suyo; nada de colores hardcodeados en Nix, Lua,
  Rust ni Slint:
  - `share/themes/neptunia.nix` (vía `share/themes/default.nix`) — paleta de
    **terminal** (kmscon, wezterm, instalador y yazi).
  - `share/design/neptunia` (`tokens.json` + `components.json` → `generated/`) —
    **interfaz gráfica** (Slint, CSS, QML). El asistente importa `Theme` y
    `Metrics` de `generated/neptunia.slint`.
- Copias generadas y versionadas (checks del flake que fallan si se
  desincronizan):
  - `share/themes/kmscon-palette` ← `nix build .#kmscon-palette`
  - `share/design/neptunia/generated/*` ←
    `nix eval --raw -f share/design/neptunia {slint,css,qml}`

## Estilo

- Nix: respeta el estilo del archivo que tocas; no reformatees archivos enteros
  en un cambio funcional. Herramientas en `nix develop`: nixfmt, statix,
  deadnix, nil.
- Rust: rustfmt y `cargo clippy -D warnings` limpios. Errores con
  `color_eyre`/`Result`, nunca `unwrap()` en rutas que dependan del sistema.
  Secretos con `Zeroizing` y por stdin o archivo temporal, nunca por argv.
  `Cargo.toml` tiene dos perfiles: `release` (solo `panic = "abort"`, el que usa
  Nix) y `dist` (hereda el anterior y añade `opt-level = "z"`, `lto = true`,
  `codegen-units = 1` y `strip = true`; es el de las releases de GitHub,
  `cargo build --profile dist`).
- devShells: `.#installer` (TUI) y `.#setup-gui` (Slint).
