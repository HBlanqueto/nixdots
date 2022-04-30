{
  character = {
    error_symbol = "[❯](bold red)";
    success_symbol = "[❯](bold green)";
    vicmd_symbol = "[❯](bold yellow)";
    format = "[|](bold bright-black) $symbol ";
  };

  hostname = {
    ssh_only = true;
    format = "[$hostname](bold blue) ";
    disabled = false;
  };

  directory = {
    format = "[ ](bold blue) [$path](bold blue) $read_only";
    truncation_length = 8;
    read_only	= "🔒";
  };

  git_branch = {
    format = "on [$symbol$branch(:$remote_branch)](bold bright-green) ";
    symbol = "🌿 ";
    truncation_length = 8;
    truncation_symbol = "";
    ignore_branches = [];
  };

  format = "$all";
  add_newline = true;
  line_break.disabled = true;
  nodejs.disabled = true;

  nix_shell.symbol = "[](blue) ";
  python.symbol = "[](blue) ";
  rust.symbol = "[](red) ";
  lua.symbol = "[](blue) ";
  package.symbol = "📦  ";

}