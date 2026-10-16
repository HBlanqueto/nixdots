{
  pkgs,
  lib,
  username,
  ...
}:

{
  services.ollama = {
    enable = true;

    user = "ollama";
    group = "ollama";

    modelsDir = "/var/lib/ollama/models";
    loadModels = [
      "qwen3:8b"
      "qwen3:4b"
    ];
    environmentVariables = {
      OLLAMA_CONTEXT_LENGTH = "16384";
      OLLAMA_MAX_LOADED_MODELS = "1";
      OLLAMA_KEEP_ALIVE = "5m";
    };
  };

  systemd.services.ollama.serviceConfig = {
    DynamicUser = lib.mkForce false;
    PrivateUsers = lib.mkForce false;
  };
}
// lib.optionalAttrs (username != null) {
  home-manager.users.${username} = {
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
        provider.ollama = {
          npm = "@ai-sdk/openai-compatible";
          name = "Ollama (local)";
          options.baseURL = "http://127.0.0.1:11434/v1";
          models."qwen3:8b" = {
            name = "Qwen3 8B";
            limit = {
              context = 16384;
              output = 4096;
            };
          };
          models."qwen3:4b" = {
            name = "Qwen3 4B";
            limit = {
              context = 16384;
              output = 4096;
            };
          };
        };

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
  };
}
