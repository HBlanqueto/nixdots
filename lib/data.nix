# Único punto de lectura de los datos externos (no versionados), bajo
# ${NIXDOTS_ROOT}/persist/nixdots: machine.toml (sistema), users/<usuario>.toml
# (una cuenta por archivo) y la ruta del informe de facter. El flake y los
# módulos consumen este attrset; nadie más llama a getEnv, pathExists ni
# readFile para leerlos.
#
# `impure` por defecto es `builtins ? currentSystem` (evaluación pura => valores
# seguros). Las variantes de CI lo fuerzan a false para NO leer /persist ni
# siquiera con --impure.
{ impure ? (builtins ? currentSystem) }:
let
    root = builtins.getEnv "NIXDOTS_ROOT";

    persist = "${root}/persist/nixdots";

    readRaw = file:
        let path = "${persist}/${file}";
        in if impure && builtins.pathExists path
        then builtins.readFile path
        else null;

    # TOML primero; el .json hermano solo como respaldo transitorio.
    # shortcut: sin .toml se sigue leyendo el .json; retíralo cuando todas las
    # máquinas hayan migrado con `nixdots-setup migrate-toml`.
    readData = base:
        let toml = readRaw "${base}.toml";
        in if toml != null then builtins.fromTOML toml
        else
            let json = readRaw "${base}.json";
            in if json != null then builtins.fromJSON json else { };

    # Datos del sistema: hostName, cifrado, extras y los campos que elige el
    # asistente (defaultLocale, region, timeZone, timeZoneAuto, keyboardLayout,
    # keyboardVariant, prettyHostName, autoLoginUser).
    machine = readData "machine";

    # Una cuenta por archivo users/<usuario>.toml (o .json de respaldo). El
    # nombre del archivo debe coincidir con el campo `username` (ver
    # `usersValid`); los <usuario>.prefs.toml son la semilla de apariencia y no
    # cuentan como cuentas.
    usersDir = "${persist}/users";
    nameFrom = ext: file: builtins.head (builtins.match "([^.]+)\\.${ext}" file);
    userNames =
        let
            entries =
                if impure && builtins.pathExists usersDir
                then builtins.attrNames (builtins.readDir usersDir)
                else [ ];
            isSeed = n: builtins.match ".*\\.prefs\\.(toml|json)" n != null;
            pick = ext: builtins.filter
                (n: builtins.match ".*\\.${ext}" n != null && !(isSeed n))
                entries;
        in
        builtins.attrNames (builtins.listToAttrs (map (n: {
            name = n;
            value = null;
        }) (map (nameFrom "toml") (pick "toml") ++ map (nameFrom "json") (pick "json"))));
    usersFromDir = builtins.listToAttrs (map (name:
        let value = readData "users/${name}";
        in
        {
            inherit name;
            value = value // { username = value.username or name; };
        }
    ) userNames);
    mismatched = builtins.filter
        (name: usersFromDir.${name}.username != name)
        userNames;

    # Respaldo (se retira cuando exista el comando de migración): el user.json
    # antiguo como un único usuario admin.
    legacy = readData "user";
    legacyUsers = if legacy ? username then {
        ${legacy.username} = {
            username = legacy.username;
            fullName = legacy.fullName or legacy.userdescription or "";
            admin = true;
        };
    } else { };

    users = if usersFromDir != { } then usersFromDir else legacyUsers;
    hasUsers = users != { };

    soleUser = if builtins.length (builtins.attrNames users) == 1
        then users.${builtins.head (builtins.attrNames users)} else null;

    # settings.nix son los valores del sistema por defecto; machine.toml manda
    # en las claves que define. Los datos de cada cuenta viven en `users`.
    settings = import ../settings.nix // machine;

    facterFile = "${persist}/facter.json";

    username = if soleUser == null then null else soleUser.username;
in
{
    inherit impure root machine settings users hasUsers username;

    # Ruta del informe de facter, o null si no hay (evaluación pura, o máquina
    # sin informe). nixpkgs lo lee y lo parsea él mismo.
    facterReport = if impure && builtins.pathExists facterFile then facterFile else null;

    # Cada archivo de users/ debe llamarse como su `username`.
    usersValid = mismatched == [ ];
    usersError =
        "En ${usersDir} cada cuenta debe llamarse <username>.toml y su campo "
        + "`username` coincidir con el nombre del archivo. Revisa: "
        + builtins.concatStringsSep ", " mismatched;

    # Primer arranque en un host real: impuro y sin ninguna cuenta. En la VM de
    # prueba el flake fuerza firstBoot = !hasUsers.
    firstBoot = impure && !hasUsers;

    # Existencia de un archivo bajo root (p. ej. el hash de contraseña), para
    # no repetir pathExists fuera de aquí.
    exists = path: impure && builtins.pathExists (root + path);
}
