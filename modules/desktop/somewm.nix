{ config, pkgs, username, ... }:
let
    dconfProfile = pkgs.writeText "somewm-dconf-profile" ''
        user-db:somewm
        user-db:user
    '';

    somewmSession = pkgs.writeShellScriptBin "somewm-session" ''
        if [ -z "''${XDG_CURRENT_DESKTOP:-}" ]; then
            export XDG_CURRENT_DESKTOP=somewm
        fi
        export XDG_SESSION_TYPE=wayland
        export DCONF_PROFILE=${dconfProfile}

        # Give this session its own IPC socket. The compositor and every client
        # it spawns (Quickshell, terminals, somewm-client) inherit the variable,
        # so they all agree on the path. The name mirrors the Wayland display
        # this compositor creates (wayland-0); a nested instance is launched
        # with the `nswm` fish function, which sets a per-instance SOMEWM_SOCKET
        # and therefore can never steal or delete this session's socket.
        if [ -n "''${XDG_RUNTIME_DIR:-}" ]; then
            export SOMEWM_SOCKET="''${XDG_RUNTIME_DIR}/somewm-''${WAYLAND_DISPLAY:-wayland-0}"
        fi

        ${pkgs.dbus}/bin/dbus-update-activation-environment --systemd \
            XDG_CURRENT_DESKTOP XDG_SESSION_TYPE DCONF_PROFILE SOMEWM_SOCKET || true

        rm -f "''${XDG_CONFIG_HOME:-$HOME/.config}/dconf/somewm"
        ${pkgs.dconf}/bin/dconf write /org/gnome/desktop/wm/preferences/button-layout "':'" || true

        ${pkgs.systemd}/bin/systemctl --user reset-failed || true
        ${pkgs.systemd}/bin/systemctl --user start somewm-session.service || true

        exec ${pkgs.somewm}/bin/somewm "$@"
    '';
in
{
    services.displayManager.sessionPackages = [
        ((pkgs.writeTextFile {
            name = "somewm-wayland-session";
            destination = "/share/wayland-sessions/somewm.desktop";
            text = ''
                [Desktop Entry]
                Name=SomeWM
                Comment=Dynamic window manager for Wayland
                Exec=${somewmSession}/bin/somewm-session
                Type=Application
                DesktopNames=somewm;
            '';
        }).overrideAttrs (old: {
            passthru.providedSessions = [ "somewm" ];
        }))
    ];

    systemd.user.services.somewm-session = {
        description = "somewm graphical session";
        bindsTo = [ "graphical-session.target" ];
        partOf = [ "graphical-session.target" ];
        before = [ "graphical-session.target" ];
        wants = [ "graphical-session-pre.target" ];
        after = [ "graphical-session-pre.target" ];

        serviceConfig = {
            Type = "oneshot";
            RemainAfterExit = true;
            ExecStart = "${pkgs.coreutils}/bin/true";
            # Tear the session target down when this marker is stopped
            # (i.e. at logout), so a later login never sees a stale
            # active graphical-session.target and gnome-session aborts
            # with "A graphical session is already running!".
            ExecStop = "${pkgs.systemd}/bin/systemctl --user stop graphical-session.target";
        };
    };
}
