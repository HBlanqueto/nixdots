# Nueve conceptos de prompt para starship, paleta neptunia en hex literal.
#
# Uso:
#   programs.starship.settings =
#     (import ./config/prompts.nix).capsula.normal;
#
# Conceptos: capsula | plano | trazo
# Variantes: minimal | normal | extravagante
#
# Paleta usada (por si cambias el tema y quieres buscar/reemplazar):
#   2a3646  relleno de cápsula (surface)
#   55657e  tenue (c8)
#   d47d7a  coral (c1)        e59592  coral brillante (c9)
#   8ecb94  verde (c2)        a3dcb1  verde brillante (c10)
#   cfb984  oro (c3)          dfca9b  oro brillante (c11)
#   aa96c2  morado (c5)       bda9d6  morado brillante (c13)
#   a5c0d2  celeste (c6)
#
# Glifos usados (todos Nerd Font):   media luna,  rama git.
# Con warn_about_missing_glyphs = true en wezterm te avisa si falta alguno.

{

  # ─────────────────────────────────────────────────────────────
  # CÁPSULA — la geometría lleva la estructura
  # ─────────────────────────────────────────────────────────────

  capsula.minimal = {
    format = "$directory$character";
    add_newline = false;

    directory = {
      format = "[](fg:#2a3646)[ $path ]($style)[](fg:#2a3646) ";
      style = "fg:#bda9d6 bg:#2a3646";
      truncation_length = 2;
      truncation_symbol = "…/";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](fg:#55657e)";
      error_symbol = "[❯](fg:#d47d7a)";
    };
  };

  capsula.normal = {
    format = "$directory$git_branch$git_status$character";
    right_format = "$cmd_duration";
    add_newline = false;

    directory = {
      format = "[](fg:#2a3646)[ $path ]($style)[](fg:#2a3646) ";
      style = "bold fg:#dfca9b bg:#2a3646";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[](fg:#2a3646)[ $symbol$branch ]($style)[](fg:#2a3646) ";
      symbol = " ";
      style = "fg:#a3dcb1 bg:#2a3646";
    };

    git_status = {
      format = "[$all_status$ahead_behind]($style) ";
      style = "fg:#e59592";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](bold fg:#a3dcb1)";
      error_symbol = "[❯](bold fg:#e59592)";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[$duration]($style)";
      style = "fg:#55657e";
    };
  };

  capsula.extravagante = {
    format = "$directory$git_branch$git_status$cmd_duration$line_break$character";
    add_newline = true;

    directory = {
      format = "[](fg:#2a3646)[ $path ]($style)[](fg:#2a3646) ";
      style = "bold fg:#e59592 bg:#2a3646";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[](fg:#2a3646)[ $symbol$branch ]($style)[](fg:#2a3646) ";
      symbol = " ";
      style = "bold fg:#dfca9b bg:#2a3646";
    };

    git_status = {
      format = "[](fg:#2a3646)[ $all_status$ahead_behind ]($style)[](fg:#2a3646) ";
      style = "fg:#bda9d6 bg:#2a3646";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[](fg:#2a3646)[ $duration ]($style)[](fg:#2a3646)";
      style = "fg:#a3dcb1 bg:#2a3646";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](bold fg:#e59592)[❯](bold fg:#dfca9b)[❯](bold fg:#a3dcb1)";
      error_symbol = "[❯❯❯](bold fg:#d47d7a)";
    };
  };

  # ─────────────────────────────────────────────────────────────
  # PLANO — el espacio y el color llevan la estructura
  # ─────────────────────────────────────────────────────────────

  plano.minimal = {
    format = "$directory$character";
    add_newline = false;

    directory = {
      format = "[$path]($style) ";
      style = "fg:#cfb984";
      truncation_length = 1;
      truncation_symbol = "";
      truncate_to_repo = false;
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](fg:#55657e)";
      error_symbol = "[❯](fg:#d47d7a)";
    };
  };

  plano.normal = {
    format = "$directory$git_branch$git_status$character";
    right_format = "$cmd_duration";
    add_newline = false;

    directory = {
      format = "[$path]($style) ";
      style = "bold fg:#dfca9b";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[$symbol$branch]($style) ";
      symbol = " ";
      style = "fg:#a3dcb1";
    };

    git_status = {
      format = "[$all_status$ahead_behind]($style) ";
      style = "fg:#e59592";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](bold fg:#bda9d6)";
      error_symbol = "[❯](bold fg:#d47d7a)";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[$duration]($style)";
      style = "fg:#55657e";
    };
  };

  plano.extravagante = {
    format = "[╭─](fg:#55657e)$directory$git_branch$git_status$line_break[╰─](fg:#55657e)$character";
    right_format = "$cmd_duration$time";
    add_newline = true;

    directory = {
      format = "[ $path]($style)";
      style = "bold fg:#e59592";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[ · ](fg:#55657e)[$symbol$branch]($style)";
      symbol = " ";
      style = "fg:#dfca9b";
    };

    git_status = {
      format = "[ · ](fg:#55657e)[$all_status$ahead_behind]($style)";
      style = "fg:#bda9d6";
    };

    character = {
      format = " $symbol ";
      success_symbol = "[❯](bold fg:#e59592)[❯](bold fg:#dfca9b)[❯](bold fg:#a3dcb1)";
      error_symbol = "[❯❯❯](bold fg:#d47d7a)";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[$duration ]($style)";
      style = "fg:#55657e";
    };

    time = {
      disabled = false;
      format = "[$time]($style)";
      time_format = "%H:%M";
      style = "fg:#55657e";
    };
  };

  # ─────────────────────────────────────────────────────────────
  # TRAZO — el color lleva la estructura, la geometría es plana
  # ─────────────────────────────────────────────────────────────

  trazo.minimal = {
    format = "$directory$character";
    add_newline = false;

    directory = {
      format = "[$path]($style)  ";
      style = "underline fg:#e59592";
      truncation_length = 2;
      truncation_symbol = "…/";
      truncate_to_repo = false;
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](fg:#55657e)";
      error_symbol = "[❯](fg:#d47d7a)";
    };
  };

  trazo.normal = {
    format = "$directory$git_branch$git_status$character";
    right_format = "$cmd_duration";
    add_newline = false;

    directory = {
      format = "[$path]($style)  ";
      style = "bold underline fg:#dfca9b";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[$symbol$branch]($style)  ";
      symbol = " ";
      style = "underline fg:#a3dcb1";
    };

    git_status = {
      format = "[$all_status$ahead_behind]($style)  ";
      style = "underline fg:#e59592";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](bold fg:#bda9d6)";
      error_symbol = "[❯](bold fg:#d47d7a)";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[$duration]($style)";
      style = "underline fg:#55657e";
    };
  };

  trazo.extravagante = {
    format = "$git_status$directory$git_branch$character";
    right_format = "$cmd_duration";
    add_newline = false;

    directory = {
      format = "[$path]($style)  ";
      style = "bold underline fg:#dfca9b";
      truncation_length = 3;
      truncate_to_repo = false;
      read_only = " ";
    };

    git_branch = {
      format = "[$symbol$branch]($style)  ";
      symbol = " ";
      style = "underline fg:#a3dcb1";
    };

    # Cada tipo de cambio enciende un punto de su propio color.
    # Repo limpio = sin puntos (git no expone un estado "limpio").
    git_status = {
      format = "[$conflicted$deleted$renamed$modified$staged$untracked$stashed]($style)[$ahead_behind]($style) ";
      conflicted = "[●](fg:#d47d7a)";
      deleted = "[●](fg:#e59592)";
      renamed = "[●](fg:#bda9d6)";
      modified = "[●](fg:#cfb984)";
      staged = "[●](fg:#8ecb94)";
      untracked = "[●](fg:#aa96c2)";
      stashed = "[●](fg:#a5c0d2)";
      ahead = "[↑$count](underline fg:#a3dcb1)";
      behind = "[↓$count](underline fg:#dfca9b)";
      diverged = "[↕$ahead_count$behind_count](underline fg:#e59592)";
      style = "fg:#55657e";
    };

    character = {
      format = "$symbol ";
      success_symbol = "[❯](bold fg:#bda9d6)";
      error_symbol = "[❯](bold fg:#d47d7a)";
    };

    cmd_duration = {
      min_time = 2000;
      format = "[$duration]($style)";
      style = "underline fg:#55657e";
    };
  };

}