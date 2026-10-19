# Home de Keel

Este flake lo genera `steward home prepare` en `~/.config/home-manager` (o se copia
desde `nix flake init -t github:HBlanqueto/dotfiles#home`). Los módulos vienen de
`keel.homeModules.*`; aquí solo viven `flake.nix`, `home.nix` y `prefs.toml`.

- `steward home prepare` rellena `SYSTEM` con la arquitectura real y fija el input
  `keel` a la revisión del sistema.
- `prefs.toml` lo copia `steward home prepare` desde la semilla
  `/persist/keel/users/<usuario>.prefs.toml` y luego borra la semilla.
  Contiene apariencia y accesibilidad; git va en `home.nix`.
- Si `~/.config/home-manager` es un repositorio git, **`flake.nix`, `home.nix` y
  `prefs.toml` deben estar añadidos** (`git add`): un flake solo ve archivos
  rastreados, y `prefs.toml` es imprescindible. `steward home prepare` no crea un
  repo git; si el directorio ya lo es, hace `git add` de los tres archivos.
- Aplica con `home-manager switch -b backup --flake ~/.config/home-manager#default`.
