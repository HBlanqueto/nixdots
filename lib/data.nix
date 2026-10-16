# Único punto de lectura de los datos externos (no versionados): user.json,
# machine.json y la ruta del informe de facter, bajo ${NIXDOTS_ROOT}/persist/nixdots.
# El flake y los módulos consumen este attrset; nadie más llama a getEnv,
# pathExists ni readFile para leerlos.
#
# `impure` por defecto es `builtins ? currentSystem` (evaluación pura => valores
# seguros). Las variantes de CI lo fuerzan a false para NO leer /persist ni
# siquiera con --impure.
{ impure ? (builtins ? currentSystem) }:
let
    root = builtins.getEnv "NIXDOTS_ROOT";

    persist = "${root}/persist/nixdots";

    readJson = file:
        let path = "${persist}/${file}";
        in if impure && builtins.pathExists path
        then builtins.fromJSON (builtins.readFile path)
        else { };

    user = readJson "user.json";
    machine = readJson "machine.json";

    facterFile = "${persist}/facter.json";

    hasUser = user ? username;
in
{
    inherit impure root user machine;

    # Ruta del informe de facter, o null si no hay (evaluación pura, o máquina
    # sin informe). nixpkgs lo lee y lo parsea él mismo.
    facterReport = if impure && builtins.pathExists facterFile then facterFile else null;

    inherit hasUser;

    # Primer arranque en un host real: impuro y sin user.json. En la VM de
    # prueba el flake fuerza firstBoot = !hasUser.
    firstBoot = impure && !hasUser;

    # Existencia de un archivo bajo root (p. ej. el hash de contraseña), para
    # no repetir pathExists fuera de aquí.
    exists = path: impure && builtins.pathExists (root + path);

    # settings.nix del repo más los datos personales que escribe el asistente.
    settings = import ../settings.nix // user;
}
