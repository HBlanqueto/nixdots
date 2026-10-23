# Constructor común de los binarios Rust del repo. `root` es la raíz del flake
# (para el fileset y Cargo.lock); `extra` añade o sobreescribe atributos de
# buildRustPackage.
{
  pkgs,
  root,
  supportedSystems,
}:
{
  pname,
  subdir,
  description,
  runtimeInputs ? [ ],
  extra ? { },
}:
let
  base = {
    inherit pname;
    version = "0.1.0";

    # Solo el código Rust y lo que embebe: cambios en el resto del repo (por
    # ejemplo un .nix) no recompilan estas herramientas.
    src = pkgs.lib.fileset.toSource {
      inherit root;
      fileset = pkgs.lib.fileset.unions [
        (root + "/Cargo.toml")
        (root + "/Cargo.lock")
        (root + "/welcome/cli")
        (root + "/welcome/gui")
        (root + "/installer/Cargo.toml")
        (root + "/installer/src")
        (root + "/installer/assets")
        (root + "/system/services")
        (root + "/kits/nixcore")
        (root + "/kits/localekit")
        (root + "/kits/userd-core")
        (root + "/.cargo")
        (root + "/LICENSE")
        # El test del instalador lee la paleta del tema.
        (root + "/kits/neptunia/neptunia.nix")
        # Import de welcome/gui/ui/theme.slint.
        (root + "/kits/neptunia/generated/neptunia.slint")
        # include_str! de nixdots-home-init.
        (root + "/system/templates/home")
      ];
    };
    # buildAndTestSubdir makes the hooks run cargo inside the crate dir;
    # the Cargo.lock lives at the workspace root.
    buildAndTestSubdir = subdir;
    cargoLock.lockFile = root + "/Cargo.lock";

    nativeBuildInputs = [ pkgs.makeWrapper ];

    postInstall = ''
      wrapProgram $out/bin/${pname} \
          --prefix PATH : ${pkgs.lib.makeBinPath runtimeInputs}
    '';

    meta = {
      inherit description;
      mainProgram = pname;
      license = pkgs.lib.licenses.gpl2Only;
      platforms = supportedSystems;
    };
  };
in
pkgs.rustPlatform.buildRustPackage (base // extra)
