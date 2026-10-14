# nixdots-installer

Un instalador de terminal («siguiente, siguiente, aceptar») para la
configuración NixOS de este repositorio. Es un ejecutable en Rust que
**orquesta** herramientas existentes — `disko`, `mkpasswd`,
`nixos-generate-config`, `sbctl`, `nix` y `nixos-install` — y nunca
reimplementa el particionado ni el hashing.

## Simulación

Sin argumentos se ejecuta en modo simulación: recorre las ocho pantallas,
muestra los comandos que ejecutaría y no toca ningún disco. Funciona en
cualquier Linux, sin root:

```sh
nix run .#installer
```

o directamente con cargo, desde `installer/`:

```sh
cargo run                      # --repo <ruta|url> para elegir el repo
```

En la cabecera aparece `simulación`. Pulsa `v` en cualquier pantalla para ver
la columna de comandos (y, durante la instalación, el log en vivo).

## Instalación real (desde la ISO mínima de NixOS)

```sh
nix --extra-experimental-features 'nix-command flakes' \
  run github:HBlanqueto/nixdots#installer -- --install
```

`--install` exige root y comprobar que se está ejecutando desde el entorno
live (`/etc/NIXOS`). El log completo queda en `/tmp/nixdots-install.log` y se
copia a `/persist/` al terminar.

### Opciones

- `--install` — instalación real.
- `--repo <ruta|url>` — repositorio a instalar (por defecto
  `github:HBlanqueto/nixdots`, clonado a `/tmp/nixdots`).

## Notas

- La paleta proviene de `share/themes/neptunia.nix`; un test (`cargo test`)
  falla si las constantes de `src/theme.rs` se desincronizan.
- En la consola Linux (`TERM=linux`, la tty de la ISO) no hay truecolor: al
  arrancar se programan los 16 slots con OSC P y se restauran con OSC R al
  salir, también ante un pánico o Ctrl+C.
