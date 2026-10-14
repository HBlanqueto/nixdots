{ colors }:

{
    format = "$git_status$directory$git_branch$character";
    right_format = "$cmd_duration";
    add_newline = false;

    directory = {
        format = "[$path]($style)  ";
        style = "bold underline fg:#${colors.c11}";
        truncation_length = 3;
        truncate_to_repo = false;
        read_only = " ";
    };

    git_branch = {
        format = "[$symbol$branch]($style)  ";
        symbol = " ";
        style = "underline fg:#${colors.c10}";
    };

    git_status = {
        format = "[$conflicted$deleted$renamed$modified$staged$untracked$stashed]($style)[$ahead_behind]($style) ";
        conflicted = "[●](fg:#${colors.c1})";
        deleted = "[●](fg:#${colors.c9})";
        renamed = "[●](fg:#${colors.c13})";
        modified = "[●](fg:#${colors.c3})";
        staged = "[●](fg:#${colors.c2})";
        untracked = "[●](fg:#${colors.c5})";
        stashed = "[●](fg:#${colors.c6})";
        ahead = "[↑$count](underline fg:#${colors.c10})";
        behind = "[↓$count](underline fg:#${colors.c11})";
        diverged = "[↕$ahead_count$behind_count](underline fg:#${colors.c9})";
        style = "fg:#${colors.c8}";
    };

    character = {
        format = "$symbol ";
        success_symbol = "[❯](bold fg:#${colors.c13})";
        error_symbol = "[❯](bold fg:#${colors.c1})";
    };

    cmd_duration = {
        min_time = 2000;
        format = "[$duration]($style)";
        style = "underline fg:#${colors.c8}";
    };
}