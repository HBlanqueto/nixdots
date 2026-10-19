{ pkgs, ... }:
let
  # Subconjunto de Noto para los scripts que Inter no cubre en los nombres de
  # idioma del asistente (árabe, índicas, hebreo y otros). No se instala
  # `noto-fonts` completo porque pesa mucho. Además de instalarlos, se añaden
  # a la cadena de `sans-serif` para que fontconfig (y fontique, que usa
  # Slint) los encuentre al faltar un glifo.
  notoScripts = [
    "Noto Sans Arabic"
    "Noto Sans Hebrew"
    "Noto Sans Devanagari"
    "Noto Sans Bengali"
    "Noto Sans Gurmukhi"
    "Noto Sans Gujarati"
    "Noto Sans Oriya"
    "Noto Sans Tamil"
    "Noto Sans Telugu"
    "Noto Sans Kannada"
    "Noto Sans Malayalam"
    "Noto Sans Sinhala"
    "Noto Sans Thai"
    "Noto Sans Lao"
    "Noto Sans Khmer"
    "Noto Sans Myanmar"
    "Noto Sans Ethiopic"
    "Noto Sans Armenian"
    "Noto Sans Georgian"
    "Noto Sans Syriac"
    "Noto Sans Thaana"
    "Noto Serif Tibetan"
  ];
in
{

  fonts = {
    fontDir = {
      enable = true;
    };

    enableGhostscriptFonts = true;

    packages = with pkgs; [
      inter
      cantarell-fonts
      noto-fonts-cjk-sans
      noto-fonts-cjk-serif
      noto-fonts-color-emoji
      (noto-fonts.override {
        suffix = "-locales";
        variants = notoScripts;
      })

      nerd-fonts.ubuntu-mono
      sf-mono-liga-bin

      victor-mono

      corefonts
      vista-fonts
      twitter-color-emoji
    ];

    fontconfig = {
      defaultFonts = {
        monospace = [
          "UbuntuMono Nerd Font"
          "Victor Mono"
        ];
        # Inter y CJK primero; después los scripts de Noto para que
        # fontconfig resuelva los glifos que Inter no cubre.
        sansSerif = [
          "Inter"
          "Noto Sans CJK SC"
        ]
        ++ notoScripts;
        serif = [
          "Noto Serif CJK SC"
          "Noto Serif Tibetan"
        ];
        emoji = [
          "Twitter Color Emoji"
          "Noto Color Emoji"
        ];
      };
    };
  };
}
