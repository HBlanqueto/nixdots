rec {
  dbg = "0d1219"; # Darker BG
  lbg = "212b38"; # Lighter BG

  fg = "adbac7";
  bg = "13171d";

  c0 = "171d27";
  c1 = "d47d7a";
  c2 = "8ecb94";
  c3 = "cfb984";
  c4 = "809ad2";
  c5 = "aa96c2";
  c6 = "a5c0d2";
  c7 = "cdd9e5";

  c8 = "55657e";
  c9 = "e59592";
  c10 = "a3dcb1";
  c11 = "dfca9b";
  c12 = "97b7ff";
  c13 = "bda9d6";
  c14 = "cee8ff";
  c15 = "eef4fa";

  primary = c4;
  secondary = c6;
  primaryBright = c12;
  muted = "505050";
  transparent = "00000000";
  textColor = bg;
  activeBorderColor = primary;
  inactiveBorderColor = muted;
  highlightColor = primary;

  # Tonos de interfaz (no son colores ANSI): los usan el instalador y el
  # toolkit Neptunia para estados que la paleta de 16 no cubre.
  uiDim = "1c2430";      # superficie elevada / fondo atenuado tras un diálogo
  uiRule = "2a3442";     # subrayado de un campo de texto
  uiDisabled = "2f3a49"; # control desactivado
  logoLight = "7fb7ff";  # azul claro oficial del logo de NixOS

  # Tokens de interfaz (docs/first-boot/neptunia-tokens.css): fuente única de
  # los colores/medidas que comparten el CSS de GTK y el asistente Slint.
  # Los colores van sin «#»; los rgba tal cual. El acento de relleno con texto
  # blanco es #4a64a8 (809ad2 no contrasta con blanco).
  ui = {
    light = {
      bgWindow = "f2f4f7";
      bgSunken = "e7ebf1";
      bgSurface = "ffffff";
      bgRaised = "ffffff";
      bgControl = "e3e8ef";
      bgHover = "dde3eb";
      line = "d6dce5";
      lineStrong = "848fa1";
      ink = "1b2330";
      inkStrong = "0d1219";
      inkMuted = "55657e";
      inkFaint = "8a96a8";
      accent = "4a64a8";
      accentStrong = "3b5496";
      onAccent = "ffffff";
      accentSoft = "dce4f5";
      danger = "b04a46";
      dangerFill = "b04a46";
      onDanger = "ffffff";
      success = "357a42";
      warning = "86661a";
      info = "7058a0";
      cyan = "3a6f87";
      windowOuter = "rgba(13, 18, 25, 0.12)";
      windowInner = "rgba(255, 255, 255, 0)";
      scrim = "rgba(13, 18, 25, 0.28)";
    };
    dark = {
      bgWindow = "1e1e1e";
      bgSunken = "161616";
      bgSurface = "262626";
      bgRaised = "2b2b2b";
      bgControl = "3a3a3a";
      bgHover = "444444";
      line = "333333";
      lineStrong = "747474";
      ink = "dcdcdc";
      inkStrong = "f5f5f5";
      inkMuted = "9b9b9b";
      inkFaint = "6b6b6b";
      accent = "4a64a8";
      accentStrong = "97b7ff";
      onAccent = "ffffff";
      accentSoft = "2c3550";
      danger = "e5807c";
      dangerFill = "b04a46";
      onDanger = "ffffff";
      success = "8ecb94";
      warning = "cfb984";
      info = "aa96c2";
      cyan = "a5c0d2";
      windowOuter = "rgba(0, 0, 0, 0.9)";
      windowInner = "rgba(255, 255, 255, 0.14)";
      scrim = "rgba(0, 0, 0, 0.5)";
    };
    darkTint = {
      bgWindow = "1d1e20";
      bgSunken = "151617";
      bgSurface = "252628";
      bgRaised = "2a2b2e";
      bgControl = "38393d";
      bgHover = "424448";
      line = "323336";
      lineStrong = "73767c";
      ink = "dadcdf";
      inkStrong = "f4f5f6";
      inkMuted = "999ca3";
      inkFaint = "696b70";
      accent = "4a64a8";
      accentStrong = "97b7ff";
      onAccent = "ffffff";
      accentSoft = "2b3550";
      danger = "e5807c";
      dangerFill = "b04a46";
      onDanger = "ffffff";
      success = "8ecb94";
      warning = "cfb984";
      info = "aa96c2";
      cyan = "a5c0d2";
      windowOuter = "rgba(0, 0, 0, 0.9)";
      windowInner = "rgba(255, 255, 255, 0.14)";
      scrim = "rgba(0, 0, 0, 0.5)";
    };
    space = {
      s1 = 4;
      s2 = 8;
      s3 = 12;
      s4 = 16;
      s6 = 24;
      s8 = 32;
    };
    radius = {
      xs = 4;
      sm = 7;
      button = 8;
      md = 10;
      lg = 14;
      xl = 20;
      pill = 999;
    };
    control = {
      sm = 24;
      md = 30;
      lg = 38;
    };
    dur = {
      fast = 120;
      base = 220;
      slow = 420;
    };

    # Los nueve accent-color válidos de GNOME (además de Neptunia).
    gnomeAccents = [
      {
        id = "blue";
        color = "3584e4";
        label = "Azul";
      }
      {
        id = "teal";
        color = "2190a4";
        label = "Verde azulado";
      }
      {
        id = "green";
        color = "3a944a";
        label = "Verde";
      }
      {
        id = "yellow";
        color = "c88800";
        label = "Amarillo";
      }
      {
        id = "orange";
        color = "ed5b00";
        label = "Naranja";
      }
      {
        id = "red";
        color = "e62d42";
        label = "Rojo";
      }
      {
        id = "pink";
        color = "d56199";
        label = "Rosa";
      }
      {
        id = "purple";
        color = "9141ac";
        label = "Morado";
      }
      {
        id = "slate";
        color = "6f8396";
        label = "Pizarra";
      }
    ];
  };
}
