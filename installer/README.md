# nixdots-installer

Un instalador de terminal («siguiente, siguiente, aceptar») para la
configuración NixOS de este repositorio, y una pequeña utilidad de disco al
estilo de la de macOS. Ambos son ejecutables Rust que **orquestan** herramientas
existentes — `disko`, `mkpasswd`, `nixos-generate-config`, `sbctl`, `nix`,
`nixos-install`, `nmcli`… — y nunca reimplementan el particionado ni el hashing.

## Simulación

Sin argumentos, el instalador recorre sus doce pasos más la pantalla final,
muestra los comandos que ejecutaría y no toca ningún disco. Funciona en
cualquier Linux, sin root:

```sh
nix run .#installer                 # --repo <ruta|url> para elegir el repo
```

En la cabecera aparece `simulación`. Pulsa `v` en cualquier pantalla para ver la
columna de comandos (y, durante la instalación, el log en vivo).

La utilidad de disco se abre aparte (o con `u` desde el paso **Disco**):

```sh
nix run .#disk                      # simulación con datos de ejemplo
nix run .#disk -- --real            # opera sobre los discos reales
```

## Instalación real (desde la ISO mínima de NixOS)

```sh
nix --extra-experimental-features 'nix-command flakes' \
  run github:HBlanqueto/nixdots#installer -- --install
```

`--install` exige root y comprobar que se está ejecutando desde el entorno live
(`/etc/NIXOS`). El log completo queda en `/tmp/nixdots-install.log` y se copia a
`/persist/` al terminar.

Pasos: Bienvenido · Red · Licencia · Equipo · Zona horaria · Idioma · Usuario ·
Contraseña · Git · Disco · Resumen · Instalando (→ Listo).

### Opciones

- `--install` — instalación real.
- `--repo <ruta|url>` — repositorio a instalar (por defecto
  `github:HBlanqueto/nixdots`, clonado a `/tmp/nixdots`).

## Red

La ISO mínima de NixOS activa `networking.networkmanager`, así que el instalador
usa `nmcli` (y recurre a `iwctl` → `wpa_cli` solo si faltara). No se avanza sin
conectividad comprobada contra `cache.nixos.org`.

## Lector de licencia

`l` (o el botón «leer completa») abre un visor a pantalla completa dentro de la
TUI (sin salir a `less`): ajuste de línea, barra de desplazamiento, progreso
`línea n/total`, búsqueda incremental con resaltado (`/`, `n`/`N`) y `q`/Esc para
volver.

## Notas

- La paleta proviene de `share/themes/neptunia.nix`; un test (`cargo test`) falla
  si las constantes de `src/theme.rs` se desincronizan.
- En la consola Linux (`TERM=linux`, la tty de la ISO) no hay truecolor: al
  arrancar se programan los 16 slots con OSC P y se restauran con OSC R al salir,
  también ante un pánico o Ctrl+C.
- Si se omite el paso **Git**, `settings.nix` lleva `gitName = null;` y
  `gitEmail = null;`, y el módulo de home-manager solo configura git cuando no
  son `null`.
