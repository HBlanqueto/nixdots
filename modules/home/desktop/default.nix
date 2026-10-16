_:

{
    # Desktop-user bits shared by both hosts: the WezTerm configuration.
    programs.wezterm = {
        enable = true;
        extraConfig = builtins.readFile ./wezterm.lua;
    };
}
