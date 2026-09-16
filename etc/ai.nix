{ pkgs, lib, username, ... }:


let
    opencode-good = builtins.storePath
        "/nix/store/kcwkilp2z25gbhiwmrqwjxs3w593k105-opencode-1.18.30";
in

{
    services.ollama = {
        enable = true;

        user = "ollama";
        group = "ollama";

        modelsDir = "/var/lib/ollama/models";
        loadModels = [ "qwen3:8b" "qwen3:4b" ];
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

    home-manager.users.${username} = {
        home.packages = [ opencode-good ];

        xdg.configFile."opencode/opencode.jsonc".text = builtins.toJSON {
            "$schema" = "https://opencode.ai/config.json";
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
            model = "ollama/qwen3:4b";

            plugin = [ "opencode-dynamic-context-pruning" ];

            mcp = {
                "mcp-nixos" = {
                    type = "local";
                    command = [ "${pkgs.mcp-nixos}/bin/mcp-nixos" ];
                };
            };
        };
    };
}