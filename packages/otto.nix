# Compositor Wayland Otto (nongio/otto) y sus aplicaciones de escritorio.
#
# Se compila todo el workspace menos otto-rdp (necesita GStreamer de desarrollo
# y es un extra opcional) y los sample-clients/tools, y se instalan los mismos
# binarios y recursos que su paquete de Arch (.deb/.rpm declaran). La sesión se
# ofrece por share/wayland-sessions/otto.desktop.
#
# Skia: skia-safe 0.93 escribe binarios descargados durante el build; en el
# sandbox no hay red, así que se construye desde fuentes igual que el neovide de
# nixpkgs: el árbol de rust-skia/skia (tag m145-0.92.0, el que fija
# skia-bindings) más sus externals de third_party, y SKIA_SOURCE_DIR/FORCE_SKIA_BUILD
# para forzar el build local.
{
    lib,
    rustPlatform,
    clangStdenv,
    fetchFromGitHub,
    fetchgit,
    linkFarm,
    runCommand,
    gn,
    ninja,
    python3,
    pkg-config,
    makeWrapper,
    removeReferencesTo,
    wayland,
    libxkbcommon,
    libglvnd,
    mesa,
    libgbm,
    libdrm,
    vulkan-loader,
    vulkan-headers,
    fontconfig,
    freetype,
    libinput,
    systemd,
    dbus,
    seatd,
    pipewire,
    xwayland,
    libdisplay-info_0_3,
    pixman,
    linux-pam,
    polkit,
    zlib,
    gst_all_1,
}:

let
    version = "1.7.1";

    src = fetchFromGitHub {
        owner = "nongio";
        repo = "otto";
        rev = "0ffa2ea50bca75ddd779341be5d81e9cedfbd0b5";
        hash = "sha256-HRTc0CqQVR1leDWKQvaTOlymI24ZHvvYW3o2HdyaVlA=";
    };

    # El árbol de Skia que espera skia-bindings 0.93 (tag y externals: ver
    # skia-bindings/Cargo.toml#package.metadata y skia/DEPS).
    skia-source =
        let
            repo = fetchFromGitHub {
                owner = "rust-skia";
                repo = "skia";
                tag = "m145-0.92.0";
                hash = "sha256-9N780AwheKBJRcZC4l/uWFNq+oOyoNp4M6dJAVVAFeo=";
            };
            externals = linkFarm "skia-externals" (
                lib.mapAttrsToList
                    (name: value: { inherit name; path = fetchgit value; })
                    (lib.importJSON ./skia-externals.json)
            );
        in
        runCommand "skia-source" { } ''
            cp -R ${repo} $out
            chmod -R +w $out
            ln -s ${externals} $out/third_party/externals
        '';

    # Librerías que se cargan con dlopen (EGL, Vulkan, Wayland): el RPATH de
    # Nix no las ve, así que van en el LD_LIBRARY_PATH del wrapper.
    runtimeLibs = [
        mesa
        libgbm
        libglvnd
        vulkan-loader
        wayland
        libxkbcommon
        libdrm
    ];

    # Todo lo que las crates enlazan o buscan con pkg-config al compilar.
    buildDeps = runtimeLibs ++ [
        libinput
        systemd
        dbus
        seatd
        pipewire
        pixman
        libdisplay-info_0_3
        linux-pam
        vulkan-headers
        fontconfig
        freetype
        xwayland
        polkit
        zlib
        gst_all_1.gstreamer
        gst_all_1.gst-plugins-base
    ];
in
rustPlatform.buildRustPackage.override { stdenv = clangStdenv; } {
    pname = "otto";
    inherit version src;

    cargoLock = {
        lockFile = "${src}/Cargo.lock";
        # Dependencias git: nongio/layers (laye-rs) y el fork de smithay.
        outputHashes = {
            "laye-rs-1.18.2" = "sha256-H/LMPE1CWpgLitMfRjFGNXHNvKw3puL8A8aIGU4ErLs=";
            "layers-debug-server-0.3.0" = "sha256-H/LMPE1CWpgLitMfRjFGNXHNvKw3puL8A8aIGU4ErLs=";
            "smithay-0.7.0" = "sha256-8ZpQvdjgsXtlH1KDnDxYnt9mG9S83pcpcojwBmvEZ3E=";
            "smithay-drm-extras-0.1.0" = "sha256-8ZpQvdjgsXtlH1KDnDxYnt9mG9S83pcpcojwBmvEZ3E=";
        };
    };

    # Todo el escritorio (barra, islas, launcher, Files, ajustes…), sin los
    # extras que no hacen falta para una sesión.
    cargoBuildFlags = [
        "--workspace"
        "--exclude"
        "otto-rdp"
        "--exclude"
        "submenu-gtk"
        "--exclude"
        "submenu"
        "--exclude"
        "client-rainbow"
        "--exclude"
        "deadlock-test"
        "--exclude"
        "otto-arch"
    ];

    env = {
        SKIA_SOURCE_DIR = skia-source;
        SKIA_GN_COMMAND = "${gn}/bin/gn";
        SKIA_NINJA_COMMAND = "${ninja}/bin/ninja";
        FORCE_SKIA_BUILD = "1";
    };

    nativeBuildInputs = [
        makeWrapper
        pkg-config
        python3
        removeReferencesTo
    ];

    buildInputs = [
        rustPlatform.bindgenHook
    ]
    ++ buildDeps;

    # El hook de instalación copia todos los ejecutables del workspace; aquí se
    # añaden los recursos, y en postFixup se reescriben las rutas /usr/… y se
    # envuelven los binarios.
    postInstall = ''
        # Config: la de ejemplo es la que ya trae valores por defecto (tema
        # claro, Inter, scroll natural) y arranca barra e islas.
        install -Dm644 otto_config.example.toml $out/etc/otto/config.toml
        install -Dm644 otto_config.example.toml $out/etc/otto/config.example.toml

        # Archivo de sesión (GDM lo lista por share/wayland-sessions).
        install -Dm644 resources/otto.desktop $out/share/wayland-sessions/otto.desktop

        # Entradas de aplicaciones e icono de Files.
        install -Dm644 resources/otto-files.desktop $out/share/applications/otto-files.desktop
        install -Dm644 resources/otto-trash.desktop $out/share/applications/otto-trash.desktop
        install -Dm644 resources/otto-settings.desktop $out/share/applications/otto-settings.desktop
        install -Dm644 resources/otto-preview.desktop $out/share/applications/otto-preview.desktop
        for _px in 16 24 32 48 64 128 256 512; do
            install -Dm644 "components/otto-files/resources/icons/hicolor/''${_px}x''${_px}/apps/otto-files.png" \
                "$out/share/icons/hicolor/''${_px}x''${_px}/apps/otto-files.png"
        done
        install -Dm644 components/otto-files/resources/icons/hicolor/scalable/apps/otto-files.svg \
            $out/share/icons/hicolor/scalable/apps/otto-files.svg

        # Portal: la descripción del backend, su activación D-Bus y su unidad
        # de usuario. UseIn=otto lo limita a la sesión Otto.
        install -Dm644 components/xdg-desktop-portal-otto/otto.portal \
            $out/share/xdg-desktop-portal/portals/otto.portal
        install -Dm644 components/xdg-desktop-portal-otto/org.freedesktop.impl.portal.desktop.otto.service \
            $out/share/dbus-1/services/org.freedesktop.impl.portal.desktop.otto.service
        install -Dm644 components/xdg-desktop-portal-otto/xdg-desktop-portal-otto.service \
            $out/lib/systemd/user/xdg-desktop-portal-otto.service

        # Agente polkit para los ajustes protegidos.
        install -Dm644 resources/polkit/org.otto.settings.policy \
            $out/share/polkit-1/actions/org.otto.settings.policy

        # Fondo de pantalla, glifos Solar y recursos del escritorio.
        install -Dm644 resources/wallpaper.jpg $out/share/otto/wallpaper.jpg
        install -Dm644 -t $out/share/otto/icons/solar/linear resources/icons/solar/linear/*.svg
        install -Dm644 -t $out/share/otto/icons/solar/bold resources/icons/solar/bold/*.svg
        install -Dm644 resources/icons/solar/LICENSE $out/share/otto/icons/solar/LICENSE
        cp -r resources/plugins $out/share/otto/plugins
        cp -r resources/widgets $out/share/otto/widgets

        # El lanzador se abre en modo «ask» bajo este nombre.
        ln -s otto-launcher $out/bin/otto-ask
        install -Dm755 resources/bin/otto-look $out/bin/otto-look

        # El servicio D-Bus y su unidad apuntan a /usr/libexec; la entrada de
        # sesión, a /usr/bin. En Nix las rutas son del store.
        substituteInPlace $out/share/dbus-1/services/org.freedesktop.impl.portal.desktop.otto.service \
            --replace-fail /usr/libexec/xdg-desktop-portal-otto $out/bin/xdg-desktop-portal-otto
        substituteInPlace $out/lib/systemd/user/xdg-desktop-portal-otto.service \
            --replace-fail /usr/libexec/xdg-desktop-portal-otto $out/bin/xdg-desktop-portal-otto
        substituteInPlace $out/share/wayland-sessions/otto.desktop \
            --replace-fail /usr/bin/otto $out/bin/otto
    '';

    postFixup = ''
        # Skia graba en el binario la ruta de sus fuentes; quítala para no
        # arrastrar todo el árbol de Skia en el cierre.
        find $out/bin -type f -executable 2>/dev/null \
            | xargs -r remove-references-to -t ${skia-source}

        for b in $out/bin/*; do
            [ -f "$b" ] && [ ! -L "$b" ] || continue
            if head -c2 "$b" | grep -aq '#!'; then continue; fi
            wrapProgram "$b" --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
        done
    '';

    doCheck = false;

    disallowedReferences = [ skia-source ];

    passthru.providedSessions = [ "otto" ];

    meta = {
        description = "Wayland compositor with a macOS-like desktop (Otto)";
        homepage = "https://github.com/nongio/otto";
        license = lib.licenses.mit;
        mainProgram = "otto";
        platforms = [ "x86_64-linux" "aarch64-linux" ];
    };
}
