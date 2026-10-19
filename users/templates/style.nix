{ pkgs, theme, ... }:

let
  # Mismas claves que la tabla Lua esperaba: 4 nombres fijos + c0..c15.
  themeKeys = [
    "dbg"
    "lbg"
    "fg"
    "bg"
  ]
  ++ map (n: "c${toString n}") (pkgs.lib.range 0 15);
  themeLua =
    "return {\n"
    + pkgs.lib.concatMapStrings (k: "    ${k} = \"#${theme.colors.${k}}\",\n") themeKeys
    + "}\n";
in

{
  xdg = {
    configFile = {
      "lua-theme/theme.lua".text = themeLua;
    };
  };

  fonts = {
    fontconfig = {
      enable = true;
    };
  };
}
