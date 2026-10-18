# Único punto de lectura de los datos externos (no versionados), bajo
# ${NIXDOTS_ROOT}/persist/nixdots: machine.json (sistema), users/<usuario>.json
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

    readJson = file:
        let path = "${persist}/${file}";
        in if impure && builtins.pathExists path
        then builtins.fromJSON (builtins.readFile path)
        else { };

    # Datos del sistema: hostName, cifrado, extras y los campos que elige el
    # asistente (defaultLocale, region, timeZone, timeZoneAuto, keyboardLayout,
    # keyboardVariant, prettyHostName, autoLoginUser).
    machine = readJson "machine.json";

    # Una cuenta por archivo users/<usuario>.json. Solo *.json, y el nombre del
    # archivo debe coincidir con el campo `username` (ver `usersValid`).
    usersDir = "${persist}/users";
    userFiles =
        if impure && builtins.pathExists usersDir
        then builtins.filter (n: builtins.match ".*\\.json" n != null)
            (builtins.attrNames (builtins.readDir usersDir))
        else [ ];
    usersFromDir = builtins.listToAttrs (map (file:
        let
            name = builtins.head (builtins.match "(.*)\\.json" file);
            value = builtins.fromJSON (builtins.readFile "${usersDir}/${file}");
        in
        {
            inherit name;
            value = value // { username = value.username or name; };
        }
    ) userFiles);
    mismatched = builtins.filter
        (file:
            let name = builtins.head (builtins.match "(.*)\\.json" file);
            in usersFromDir.${name}.username != name)
        userFiles;

    # Respaldo (se retira cuando exista el comando de migración): el user.json
    # antiguo como un único usuario admin con la plantilla completa.
    legacy = readJson "user.json";
    legacyUsers = if legacy ? username then {
        ${legacy.username} = legacy // {
            username = legacy.username;
            fullName = legacy.fullName or legacy.userdescription or "";
            admin = true;
            template = "nixdots";
        } // (if legacy ? gitName && legacy ? gitEmail then {
            git = { name = legacy.gitName; email = legacy.gitEmail; };
        } else { });
    } else { };

    users = if usersFromDir != { } then usersFromDir else legacyUsers;
    hasUsers = users != { };

    userNames = builtins.attrNames users;
    soleUser = if builtins.length userNames == 1 then users.${builtins.head userNames} else null;

    # settings.nix son los valores del sistema por defecto; machine.json manda
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
        "En ${usersDir} cada cuenta debe llamarse <username>.json y su campo "
        + "`username` coincidir con el nombre del archivo. Revisa: "
        + builtins.concatStringsSep ", " mismatched;

    # Primer arranque en un host real: impuro y sin ninguna cuenta. En la VM de
    # prueba el flake fuerza firstBoot = !hasUsers.
    firstBoot = impure && !hasUsers;

    # Existencia de un archivo bajo root (p. ej. el hash de contraseña), para
    # no repetir pathExists fuera de aquí.
    exists = path: impure && builtins.pathExists (root + path);
}
