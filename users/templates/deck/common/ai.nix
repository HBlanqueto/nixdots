{ pkgs, ... }:

{
  # `uv` provides `uv tool install graphifyy`; installed tools land in
  # ~/.local/bin, which is prepended to PATH below.
  home = {
    packages = [ pkgs.uv ];
    sessionPath = [ "$HOME/.local/bin" ];
  };

  programs.opencode = {
    enable = true;
    package = pkgs.opencode;

    settings = {
      # npm plugins are fetched by opencode at startup, so there is
      # nothing to package here.
      plugin = [ "@dietrichgebert/ponytail" ];

      mcp = {
        "mcp-nixos" = {
          type = "local";
          command = [ "${pkgs.mcp-nixos}/bin/mcp-nixos" ];
        };
      };
    };
  };
}
