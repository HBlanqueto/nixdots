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
  - `machine.json` — lo escribe el instalador (hostName, encrypt, luksUuid,
    secureBoot, tpmUnlock, extras).
  - `facter.json` — informe de hardware.
  - `user.json` — lo escribe el asistente de primer arranque (usuario, nombre,
    prettyHostName localizado, locale, región, teclado, zona, a11y, git).
  - `/persist/secrets/<usuario>.hash` — hash de la contraseña
    (`hashedPasswordFile`), en un directorio `0711` y el archivo `0600`, ambos
    de root. Nunca hashes ni secretos en el repo.
- `lib/data.nix` es el **único** punto que lee esos archivos (getEnv,
  pathExists, readFile). Los módulos consumen `data` por specialArgs.
- `settings.nix` solo tiene valores independientes de la máquina (stateVersion,
  zona, locale, teclado por defecto).
- Diferencias de hardware: `modules/common/hardware-detect.nix` decide con
  `config.hardware.facter.detected/report` (bare-metal AMD → ucodenix y amdgpu
  en initrd; portátil → acpid/upower; QEMU/UTM → spice-vdagent, qemu-guest).
  No uses condicionales por nombre de host.
- Opciones propias bajo `options.nixdots.*` (`machine`, `encryption`,
  `console`, `firstBoot`…).
- Impermanence: la raíz se restaura en cada arranque (Btrfs `@rootfs` desde
  `@rootfs-blank`). Todo estado que deba sobrevivir se declara en
  `environment.persistence."/persist"`.

## Componentes

- `modules/common/` — capa NixOS común; `modules/desktop/` — GNOME/GDM;
  `modules/home/` — home-manager (fish, wezterm, starship, yazi…).
- Escritorio: **solo GNOME**. SomeWM y Quickshell se quitaron; no los
  reintroduzcas sin que se pida.
- Consola: la TTY instalada usa **solo** la fuente XNU (kmscon con `font-name
  "XNU Console"`, tamaño 16, y `console.font` = `xnu-console.psf` en el kernel:
  initrd, consola de emergencia y Plymouth). Solo tiene 256 glifos Latin-1, así
  que en la TTY el prompt de fish (starship) y `eza` van sin iconos Nerd Font.
  El lanzador del instalador sí abre kmscon con Liga SFMono Nerd Font 14 y la
  paleta neptunia. Plymouth usa el tema integrado `spinner`.
- Workspace Rust (`Cargo.toml` raíz):
  - `core` — settings, user.json, hash, ejecución de comandos.
  - `lists` — locales, zonas y teclados embebidos al compilar.
  - `installer` — TUI ratatui (`nixdots-installer`). Solo instala el sistema;
    pantallas: Inicio · Licencia · Red · Lenguaje · Teclado · Zona horaria ·
    Equipo · Disco · Cifrado · Resumen · Instalando · Listo. No pide usuario,
    contraseña ni Git, ni escribe `user.json` ni el hash (eso lo hace el
    asistente). La frase de LUKS es siempre propia. Sin soporte de ratón.
  - `setup` — CLI del primer arranque (`nixdots-setup apply`).
  - `setup-gui` — asistente gráfico «Hola» en **Slint** (no GTK/libadwaita).
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
- devShells: `.#installer` (TUI) y `.#setup-gui` (Slint).
