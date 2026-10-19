# Home de nixdots

Este flake lo genera `nixdots-home-init` en `~/.config/home-manager` (o se copia
desde `nix flake init -t nixdots#home`). Los módulos vienen de
`nixdots.homeModules.*`; aquí solo viven `flake.nix`, `home.nix` y `prefs.toml`.

- `nixdots-home-init` rellena `SYSTEM` con la arquitectura real y fija el input
  `nixdots` a la revisión del sistema.
- `prefs.toml` lo copia `nixdots-home-init` desde la semilla
  `/persist/nixdots/users/<usuario>.prefs.toml` y luego borra la semilla.
  Contiene apariencia y accesibilidad; git va en `home.nix`.
- Si `~/.config/home-manager` es un repositorio git, **`flake.nix`, `home.nix` y
  `prefs.toml` deben estar añadidos** (`git add`): un flake solo ve archivos
  rastreados, y `prefs.toml` es imprescindible. `nixdots-home-init` no crea un
  repo git; si el directorio ya lo es, hace `git add` de los tres archivos.
- Aplica con `home-manager switch -b backup --flake ~/.config/home-manager#default`.
