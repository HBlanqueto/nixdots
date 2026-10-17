# Neptunia: diseños guardados como datos

Esta carpeta es la única fuente del diseño de nixdots. Los lienzos de Claude
son donde se diseña y se aprueba; aquí se guarda lo aprobado en un formato que
cualquier proyecto puede leer, presente o futuro, sin depender de un framework.

```
share/design/neptunia/
├── tokens.json        colores (claro, oscuro, oscuro tintado), radios, espacios, tamaños,
│                      duraciones, tipografía y sombras
├── components.json    medidas y estados de cada componente aprobado (interruptor 32×18,
│                      segmentado rectangular, pestañas de barra de herramientas…)
├── default.nix        traduce los JSON a cada formato; solo usa builtins, sin nixpkgs
├── generated/         salidas ya generadas, para quien no use Nix (SlintPad, otro repo)
│   ├── neptunia.slint   globals Theme y Metrics para Slint
│   ├── neptunia.css     variables CSS (GTK4, web, Quickshell con CSS)
│   └── Neptunia.qml     singleton para QML / Quickshell
└── demo.slint         galería para ajustar a ojo con slint-viewer o SlintPad
```

## Regla de oro

Nadie escribe un color ni una medida a mano. Si cambia el diseño, cambia
`tokens.json` o `components.json` y se regenera:

```sh
cd share/design/neptunia
nix eval --raw -f . slint > generated/neptunia.slint
nix eval --raw -f . css   > generated/neptunia.css
nix eval --raw -f . qml   > generated/Neptunia.qml
```

## Cómo lo lee cada proyecto

| Proyecto | Cómo se sincroniza |
| --- | --- |
| Psamathe y apps Triton (Slint) | `import { Theme, Metrics } from "…/neptunia.slint";`. Desde Nix: `(import ./share/design/neptunia).slint`; o copia `generated/neptunia.slint`. Cambiar de tema es `Theme.mode = NeptuniaMode.dark;`. |
| Instalador (ratatui) | Sigue con la paleta de terminal de `share/themes/neptunia.nix`. Si algún día quiere los grises de interfaz: `serde_json` sobre `tokens.json` con `include_str!`. |
| Quickshell (QML) | `Neptunia.qml` como singleton (`qmldir`: `singleton Neptunia 1.0 Neptunia.qml`) y `Neptunia.mode = "dark"`. |
| GTK4 / web | `neptunia.css` con `data-theme="dark"` o `"dark-tint"` en la raíz. |
| Home Manager | `xdg.configFile."neptunia".source = (import ./share/design/neptunia).files pkgs;` deja los tres archivos y los JSON en `~/.config/neptunia/`, y cualquier app los lee de ahí. |
| Otro repo futuro | Como input del flake (`inputs.nixdots`) y `import "${inputs.nixdots}/share/design/neptunia"`, o leyendo `tokens.json` directamente. Cada proyecto genera lo suyo a su manera; los datos son los mismos. |

## Ajustar una pantalla sin recompilar Rust

`slint-viewer --auto-reload` abre un `.slint` suelto y lo recarga cada vez que
guardas. Es lo más rápido para ajustar una sola pantalla, por ejemplo «Tu
cuenta», sin tocar el resto:

```sh
slint-viewer --auto-reload share/design/neptunia/demo.slint
slint-viewer --auto-reload setup-gui/ui/cuenta.slint   # una pantalla concreta
```

`slint-lsp`, con la extensión de VS Code o con Neovim o Helix, da
autocompletado y una vista previa en vivo en la que se pueden tocar
propiedades. Los dos se instalan desde nixpkgs (`slint-lsp`, `slint-viewer`)
en el `devShell`.

## SlintPad

SlintPad (slintpad.com) es un editor en el navegador para probar un componente
sin instalar nada. Como `demo.slint` importa `generated/neptunia.slint`, hay dos
formas de abrirlo:

- Pega primero el contenido de `generated/neptunia.slint` y debajo el
  componente que quieras probar, quitando la línea `import`.
- O ábrelo desde GitHub con
  `https://slintpad.com/?load_url=<url raw de demo.slint>`. SlintPad suele
  resolver el import relativo; si no lo resuelve, usa la primera forma.

## Traducciones con `@tr()`

Marca cada texto visible con `@tr("Continuar")`, como ya hace `demo.slint`, y
usa el español como idioma fuente. Slint extrae esas cadenas a archivos `.po`,
uno por idioma, que se compilan dentro del binario. Así el asistente puede
cambiar de idioma en vivo cuando se elige en el paso 1:

```sh
cargo install slint-tr-extractor
find setup-gui/ui -name '*.slint' | xargs slint-tr-extractor -o psamathe.pot
msginit -i psamathe.pot -l en -o lang/en/LC_MESSAGES/psamathe.po
```

```rust
// build.rs (versiones recientes de Slint; comprueba la API de la versión que uses)
slint_build::compile_with_config(
    "ui/app.slint",
    slint_build::CompilerConfiguration::new().with_bundled_translations("lang"),
).unwrap();
// al elegir el idioma:
slint::select_bundled_translation("en").ok();
```

El saludo animado en 13 idiomas no es una traducción: es una lista fija de
palabras y se queda como datos.

## De los lienzos de Claude a esta carpeta

1. Se diseña y se aprueba en los lienzos (sistema de diseño Neptunia).
2. Lo aprobado se vuelca a `tokens.json` y `components.json`. Pide a Claude
   «actualiza share/design/neptunia» y lo hace desde el sistema de diseño.
3. Se regeneran las salidas y cada app las toma en su siguiente build.
