# Compositor Wayland Otto (nongio/otto) y sus aplicaciones de escritorio.
#
# Fuentes: Otto y sus dos dependencias git (nongio/layers y el fork
# nongio/smithay) se fijan aquí, una vez, por commit y hash NAR. Con
# `useMirror = true` se descargan de las copias privadas de HBlanqueto
# (otto, layers, smithay-otto); si no, de nongio. El contenido es el mismo, así
# que la ruta del store y la derivación también: da igual de dónde salga. Las dos dependencias git se compilan como dependencias
# `path` (no como `git`), porque el vendoreo de Cargo en el sandbox no tiene
# credenciales para leer un repo privado.
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
    writeText,
    # true: descargar de las copias privadas de HBlanqueto (ver arriba).
    useMirror ? false,
}:

let
    version = "1.7.1";

    # Commits fijados. `hash` es el hash NAR del árbol (sin .git), el mismo en
    # nongio y en la copia. `ref` es como los cita el Cargo.lock de Otto; al
    # subir de versión, cópialo de ahí.
    pins = {
        otto = {
            repo = "otto";
            mirror = "otto";
            rev = "0ffa2ea50bca75ddd779341be5d81e9cedfbd0b5";
            hash = "sha256-HRTc0CqQVR1leDWKQvaTOlymI24ZHvvYW3o2HdyaVlA=";
        };
        layers = {
            repo = "layers";
            mirror = "layers";
            rev = "744fb23948cce807cfe13e41583b68d056b7d387";
            ref = "tag=v1.18.2";
            hash = "sha256-H/LMPE1CWpgLitMfRjFGNXHNvKw3puL8A8aIGU4ErLs=";
        };
        smithay = {
            repo = "smithay";
            mirror = "smithay-otto";
            rev = "9341e4d190db994690b4af91ac86f83b5597e9b2";
            ref = "rev=9341e4d190db994690b4af91ac86f83b5597e9b2";
            hash = "sha256-8ZpQvdjgsXtlH1KDnDxYnt9mG9S83pcpcojwBmvEZ3E=";
        };
    };

    # Salidas de hash fijo: la ruta del store depende solo del nombre y del
    # hash, no de la URL. Copia y upstream dan la misma ruta (y la misma
    # derivación de Otto), así que cambiar de origen no recompila nada y solo se
    # descarga lo que falte en el store. La copia privada usa la API de GitHub
    # con NIX_GITHUB_PRIVATE_{USERNAME,PASSWORD} del nix-daemon (core/nix.nix).
    fetchPin =
        pin:
        fetchFromGitHub (
            if useMirror then
                {
                    owner = "HBlanqueto";
                    repo = pin.mirror;
                    private = true;
                    inherit (pin) rev hash;
                }
            else
                {
                    owner = "nongio";
                    inherit (pin) repo rev hash;
                }
        );

    src = fetchPin pins.otto;
    layers = fetchPin pins.layers;
    smithay = fetchPin pins.smithay;

    # Cargo.lock sin el `source` de las dos dependencias git: como `path` no
    # llevan source. Si un pin deja de coincidir con el lock, la assert falla
    # aquí en vez de bajar las fuentes de nongio a escondidas.
    gitSource =
        pin: "source = \"git+https://github.com/nongio/${pin.repo}?${pin.ref}#${pin.rev}\"\n";
    upstreamLock = builtins.readFile "${src}/Cargo.lock";
    lockContents =
        let
            patched =
                builtins.replaceStrings [ (gitSource pins.layers) (gitSource pins.smithay) ] [ "" "" ]
                    upstreamLock;
        in
        # (builtins.split y no lib.hasInfix: su regex `.*….*` desborda la
        # pila con un Cargo.lock de este tamaño.)
        assert lib.assertMsg
            (builtins.length (builtins.split (lib.escapeRegex ''source = "git+'') patched) == 1)
            "otto: el Cargo.lock tiene dependencias git que no están en `pins`";
        patched;
    cargoLockFile = writeText "Cargo.lock" lockContents;

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

    # Solo crates.io: laye-rs/layers-debug-server y smithay/smithay-drm-extras
    # son dependencias `path` (ver postPatch).
    cargoLock.lockFileContents = lockContents;

    # Copias escribibles de las dependencias junto a source/: el build.rs de
    # laye-rs escribe su cabecera C en el directorio del crate.
    postUnpack = ''
        mkdir -p otto-deps
        cp -r --no-preserve=mode ${layers} otto-deps/layers
        cp -r --no-preserve=mode ${smithay} otto-deps/smithay
    '';

    # Las dependencias git pasan a `path` con las mismas fuentes. --replace-fail:
    # si upstream cambia esas líneas, el build se para aquí.
    postPatch = ''
        substituteInPlace Cargo.toml \
            --replace-fail \
                'smithay = { git = "https://github.com/nongio/smithay", rev = "${pins.smithay.rev}", default-features = false }' \
                'smithay = { path = "../otto-deps/smithay", default-features = false }' \
            --replace-fail \
                'laye-rs = { git = "https://github.com/nongio/layers", tag = "v1.18.2", features = [' \
                'laye-rs = { path = "../otto-deps/layers", features = [' \
            --replace-fail \
                'git = "https://github.com/nongio/smithay"' \
                'path = "../otto-deps/smithay/smithay-drm-extras"' \
            --replace-fail \
                'rev = "${pins.smithay.rev}" # feat/vulkan-device; keep in step with smithay below' \
                ""
        install -m644 ${cargoLockFile} Cargo.lock
    '';

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
