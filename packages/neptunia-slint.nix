# Genera los tokens de interfaz de Neptunia para Slint
# (setup-gui/ui/neptunia-tokens.slint) desde share/themes/neptunia.nix, la
# única fuente de verdad de la paleta. El archivo está versionado; regenéralo
# con:
#   cp "$(nix build .#neptunia-slint --print-out-paths)" setup-gui/ui/neptunia-tokens.slint
{ pkgs }:

let
  lib = pkgs.lib;
  ui = (import ../share/themes/neptunia.nix).ui;

  hex = s: if lib.hasPrefix "rgba" s then s else "#${s}";
  pick = theme: name: hex ui.${theme}.${name};

  color =
    name:
    "    out property <color> ${name}: "
    + "theme == \"light\" ? ${pick "light" name} "
    + ": (theme == \"dark\" ? ${pick "dark" name} : ${pick "darkTint" name});\n";

  colorNames = [
    "bgWindow"
    "bgSunken"
    "bgSurface"
    "bgRaised"
    "bgControl"
    "bgHover"
    "line"
    "lineStrong"
    "ink"
    "inkStrong"
    "inkMuted"
    "inkFaint"
    "accent"
    "accentStrong"
    "onAccent"
    "accentSoft"
    "danger"
    "dangerFill"
    "onDanger"
    "success"
    "warning"
    "info"
    "cyan"
    "windowOuter"
    "windowInner"
    "scrim"
  ];

  spaces =
    lib.concatMapStrings
      (n: "    out property <length> space${n}: ${toString (builtins.getAttr "s${n}" ui.space)}px;\n")
      [
        "1"
        "2"
        "3"
        "4"
        "6"
        "8"
      ];

  radii =
    lib.concatMapStrings
      (
        n:
        "    out property <length> ${
              if n == "button" then "radiusButton" else "radius${n}"
            }: ${toString ui.radius.${n}}px;\n"
      )
      [
        "xs"
        "sm"
        "button"
        "md"
        "lg"
        "xl"
        "pill"
      ];

  controls =
    lib.concatMapStrings (n: "    out property <length> control${n}: ${toString ui.control.${n}}px;\n")
      [
        "sm"
        "md"
        "lg"
      ];

  durs =
    "    out property <duration> durFast: ${toString ui.dur.fast}ms;\n"
    + "    out property <duration> durBase: ${toString ui.dur.base}ms;\n"
    + "    out property <duration> durSlow: ${toString ui.dur.slow}ms;\n";

  accents = lib.concatMapStrings (
    a: "        { id: \"${a.id}\", color: #${a.color}, label: \"${a.label}\" },\n"
  ) ui.gnomeAccents;
in
pkgs.writeText "neptunia-tokens.slint" ''
  // GENERADO por packages/neptunia-slint.nix desde share/themes/neptunia.nix.
  // No lo edites a mano: regenéralo con `nix build .#neptunia-slint`.
  //
  // theme: "light" | "dark" | "dark-tint"; el resto son los tokens del
  // sistema Neptunia (docs/first-boot/neptunia-tokens.css).
  export struct Accent {
      id: string,
      color: color,
      label: string,
  }

  export global Tokens {
      in-out property <string> theme: "light";
      out property <bool> dark: theme != "light";

  ${lib.concatMapStrings color colorNames}${spaces}${radii}${controls}${durs}
      out property <[Accent]> gnomeAccents: [
  ${accents}    ];
  }
''
