# Neptunia: la única fuente de diseño de nixdots.
#
# tokens.json y components.json son los datos (neutros, sin framework). Este
# archivo solo los traduce a cada formato. Usa únicamente builtins, así que se
# evalúa sin nixpkgs:
#
#   nix eval --raw -f share/design/neptunia slint > neptunia.slint
#   nix eval --raw -f share/design/neptunia css   > neptunia.css
#   nix eval --raw -f share/design/neptunia qml   > Neptunia.qml
#
# Desde un módulo: (import ./share/design/neptunia).slint, o `files pkgs` para
# una derivación con los tres archivos.
let
  tokens = builtins.fromJSON (builtins.readFile ./tokens.json);
  components = builtins.fromJSON (builtins.readFile ./components.json);

  themes = [
    "light"
    "dark"
    "dark-tint"
  ];
  inherit (builtins)
    attrNames
    concatStringsSep
    isAttrs
    isInt
    isFloat
    match
    substring
    toString
    ;
  colorNames = attrNames tokens.themes.light;
  lines = f: xs: concatStringsSep "\n" (map f xs);
  num = v: if isFloat v then toString v else toString v;

  # kebab-case → camelCase (QML)
  camel =
    s:
    let
      parts = builtins.filter builtins.isString (builtins.split "-" s);
      cap = p: (builtins.substring 0 1 (upper p)) + (builtins.substring 1 (-1) p);
      upper =
        p:
        builtins.replaceStrings
          [ "a" "b" "c" "d" "e" "f" "g" "h" "i" "j" "k" "l" "m" "n" "o" "p" "q" "r" "s" "t" "u" "v" "w" "x" "y" "z" ]
          [ "A" "B" "C" "D" "E" "F" "G" "H" "I" "J" "K" "L" "M" "N" "O" "P" "Q" "R" "S" "T" "U" "V" "W" "X" "Y" "Z" ]
          p;
    in
    concatStringsSep "" ([ (builtins.head parts) ] ++ map cap (builtins.tail parts));

  # #rrggbbaa → #aarrggbb (Qt)
  qtColor =
    c: if builtins.stringLength c == 9 then "#" + substring 7 2 c + substring 1 6 c else c;

  # Medidas numéricas de components.json, aplanadas: switch.small.width → switch-small-width
  flatten =
    prefix: set:
    builtins.concatLists (
      map (
        k:
        let
          v = set.${k};
          name = if prefix == "" then k else "${prefix}-${k}";
          ref = if builtins.isString v then match "(radius|space|size|duration)\\.(.+)" v else null;
        in
        if isAttrs v then
          flatten name v
        else if isInt v || isFloat v then
          [
            {
              inherit name;
              value = v;
              duration = false;
            }
          ]
        # "radius.sm", "duration.fast"… se resuelven contra tokens.json
        else if ref != null then
          [
            {
              inherit name;
              value = tokens.${builtins.head ref}.${builtins.elemAt ref 1};
              duration = builtins.head ref == "duration";
            }
          ]
        else
          [ ]
      ) (attrNames set)
    );
  metrics = flatten "" components.components;
  metricType =
    m:
    if m.duration then
      "duration"
    else if match ".*(weight|columns).*" m.name != null then
      "int"
    else if match ".*(opacity|scale).*" m.name != null then
      "float"
    else
      "length";
  metricValue =
    m:
    let
      t = metricType m;
    in
    if t == "length" then
      "${num m.value}px"
    else if t == "duration" then
      "${num m.value}ms"
    else
      num m.value;

  styleNames = attrNames tokens.font.styles;
  sh = tokens.shadow.slint;

  slint = ''
    // Generado desde share/design/neptunia (tokens.json, components.json). No editar a mano:
    // cambia los JSON y regenera con `nix eval --raw -f share/design/neptunia slint`.
    export enum NeptuniaMode { light, dark, dark-tint }

    export struct NeptuniaAccent {
        id: string,
        color: color,
        label: string,
    }

    export global Theme {
        in-out property <NeptuniaMode> mode: NeptuniaMode.light;

        // Colores (por tema)
    ${lines (
      n:
      "    out property <color> ${n}: mode == NeptuniaMode.dark ? ${tokens.themes.dark.${n}} : mode == NeptuniaMode.dark-tint ? ${
        tokens.themes."dark-tint".${n}
      } : ${tokens.themes.light.${n}};"
    ) colorNames}

        // Redondeo, espacio, tamaños y duraciones
    ${lines (n: "    out property <length> radius-${n}: ${num tokens.radius.${n}}px;") (attrNames tokens.radius)}
    ${lines (n: "    out property <length> space-${n}: ${num tokens.space.${n}}px;") (attrNames tokens.space)}
    ${lines (n: "    out property <length> ${n}: ${num tokens.size.${n}}px;") (attrNames tokens.size)}
    ${lines (n: "    out property <duration> dur-${n}: ${num tokens.duration.${n}}ms;") (attrNames tokens.duration)}

        // Tipografía
        out property <string> font-sans: "Inter";
        out property <string> font-mono: "SF Mono Ligaturized";
        out property <string> font-script: "Dancing Script";
    ${lines (s: "    out property <length> text-${s}-size: ${num tokens.font.styles.${s}.size}px;") styleNames}
    ${lines (s: "    out property <int> text-${s}-weight: ${num tokens.font.styles.${s}.weight};") styleNames}

        // Sombras (Slint admite una por elemento)
        out property <length> shadow-window-offset-y: ${num sh.window."offset-y"}px;
        out property <length> shadow-window-blur: ${num sh.window.blur}px;
        out property <color> shadow-window-color: mode == NeptuniaMode.light ? ${sh.window.color.light} : ${sh.window.color.dark};
        out property <length> shadow-float-offset-y: ${num sh.float."offset-y"}px;
        out property <length> shadow-float-blur: ${num sh.float.blur}px;
        out property <color> shadow-float-color: mode == NeptuniaMode.light ? ${sh.float.color.light} : ${sh.float.color.dark};
        out property <length> shadow-knob-offset-y: ${num sh.knob."offset-y"}px;
        out property <length> shadow-knob-blur: ${num sh.knob.blur}px;
        out property <color> shadow-knob-color: ${sh.knob.color};

        // Acentos de sistema (GNOME), seleccionables junto al de Neptunia.
        out property <[NeptuniaAccent]> accents: [
    ${lines (a: "        { id: \"${a.id}\", color: ${a.color}, label: \"${a.label}\" },") tokens.accents}
        ];
    }

    // Medidas de components.json (switch-width, segmented-segment-radius, toolbar-tabs-tab-min-width…)
    export global Metrics {
    ${lines (m: "    out property <${metricType m}> ${m.name}: ${metricValue m};") metrics}
    }
  '';

  cssVars = th: lines (n: "  --${n}: ${tokens.themes.${th}.${n}};") colorNames;
  css = ''
    /* Generado desde share/design/neptunia. data-theme="dark" o "dark-tint" cambia el subárbol. */
    :root, [data-theme="light"] {
    ${cssVars "light"}
      --shadow-window: ${tokens.shadow.window.light};
      --shadow-float: ${tokens.shadow.float.light};
    }
    ${lines (th: ''
      [data-theme="${th}"] {
      ${cssVars th}
        --shadow-window: ${tokens.shadow.window.${th}};
        --shadow-float: ${tokens.shadow.float.${th}};
      }'') [ "dark" "dark-tint" ]}
    :root {
    ${lines (n: "  --radius-${n}: ${num tokens.radius.${n}}px;") (attrNames tokens.radius)}
    ${lines (n: "  --space-${n}: ${num tokens.space.${n}}px;") (attrNames tokens.space)}
    ${lines (n: "  --${n}: ${num tokens.size.${n}}px;") (attrNames tokens.size)}
    ${lines (n: "  --dur-${n}: ${num tokens.duration.${n}}ms;") (attrNames tokens.duration)}
      --shadow-knob: ${tokens.shadow.knob};
      --font-sans: ${tokens.font.families.sans};
      --font-mono: ${tokens.font.families.mono};
      --font-script: ${tokens.font.families.script};
    }
  '';

  qml = ''
    // Generado desde share/design/neptunia. Úsalo como singleton (qmldir: singleton Neptunia 1.0 Neptunia.qml).
    pragma Singleton
    import QtQuick

    QtObject {
        property string mode: "light" // "light" | "dark" | "dark-tint"
    ${lines (
      n:
      "    readonly property color ${camel n}: mode === \"dark\" ? \"${qtColor tokens.themes.dark.${n}}\" : mode === \"dark-tint\" ? \"${
        qtColor tokens.themes."dark-tint".${n}
      }\" : \"${qtColor tokens.themes.light.${n}}\""
    ) colorNames}
    ${lines (n: "    readonly property int ${camel "radius-${n}"}: ${num tokens.radius.${n}}") (attrNames tokens.radius)}
    ${lines (n: "    readonly property int ${camel "space-${n}"}: ${num tokens.space.${n}}") (attrNames tokens.space)}
    ${lines (n: "    readonly property int ${camel n}: ${num tokens.size.${n}}") (attrNames tokens.size)}
    ${lines (n: "    readonly property int ${camel "dur-${n}"}: ${num tokens.duration.${n}}") (attrNames tokens.duration)}
        readonly property string fontSans: "Inter"
        readonly property string fontMono: "SF Mono Ligaturized"
    }
  '';
in
{
  inherit
    tokens
    components
    themes
    slint
    css
    qml
    ;

  # Derivación con los archivos generados, para home-manager o un paquete:
  #   xdg.configFile."neptunia".source = (import ./share/design/neptunia).files pkgs;
  files =
    pkgs:
    pkgs.runCommand "neptunia-design" { } ''
      mkdir -p $out
      cp ${pkgs.writeText "neptunia.slint" slint} $out/neptunia.slint
      cp ${pkgs.writeText "neptunia.css" css} $out/neptunia.css
      cp ${pkgs.writeText "Neptunia.qml" qml} $out/Neptunia.qml
      cp ${./tokens.json} $out/tokens.json
      cp ${./components.json} $out/components.json
    '';
}
