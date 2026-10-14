{ pkgs, lib, ... }:

{
    # Desktop-user bits shared by both hosts: the WezTerm config and the helper
    # to launch a nested somewm. Kept apart from modules/home/common so the
    # compositor/terminal stay grouped as "desktop".
    programs.wezterm = {
        enable = true;
        extraConfig = builtins.readFile ./wezterm.lua;
    };

    programs.fish.functions = {
        # Start a nested somewm on its own Wayland display and its own IPC
        # socket. The socket is per-instance (PID based) so a nested compositor
        # can never steal or delete the real session's socket (the real session
        # runs on $XDG_RUNTIME_DIR/somewm-wayland-0). env keeps SOMEWM_SOCKET out
        # of the parent shell; only the nested compositor and the clients it
        # spawns inherit it.
        nswm = {
            description = "Run a nested somewm with its own IPC socket";
            body = ''
                if test -n "$XDG_RUNTIME_DIR"
                    env SOMEWM_SOCKET="$XDG_RUNTIME_DIR/somewm-nested-$fish_pid" \
                        WLR_BACKENDS=wayland WLR_WL_OUTPUTS=1 somewm
                else
                    echo 'nswm: XDG_RUNTIME_DIR is not set' >&2
                    return 1
                end
            '';
        };
    };
}
