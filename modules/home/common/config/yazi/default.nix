let
  theme = import ../../../../../share/themes;
  icons = import ./icons.nix;
in

with theme.colors;

{
  settings = {
    mgr = {
      ratio = [
        0
        1
        1
      ];
    };
  };

  theme = {
    mgr = {
      border_style = {
        fg = "#${lbg}";
      };
      border_symbol = "│";
    };

    indicator = {
      padding = {
        open = "█";
        close = "█";
      };

      current = {
        reversed = false;
        fg = "#${fg}";
        bg = "#${lbg}";
      };

      parent = {
        reversed = false;
        fg = "#${fg}";
        bg = "#${lbg}";
      };
    };

    filetype = {
      rules = icons.filetypeRules;
    };

    icon = {
      globs = icons.globs;
      dirs = icons.dirs;
      files = icons.files;
      exts = icons.exts;
      conds = icons.conds;
    };
  };
}
