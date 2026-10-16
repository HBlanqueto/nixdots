# Datos compartidos que cada nixosSystem recibe por specialArgs, derivados de
# settings.nix. En el primer arranque settings.nix no trae `username` todavía,
# así que los datos de usuario caen a null y user.nix/home.nix/ai.nix omiten la
# cuenta; el resto (stateVersion, locale, teclado) sigue siendo obligatorio.
{ settings }:
let
    username = settings.username or null;
in
{
    inherit settings;

    inherit (settings) stateVersion timeZone defaultLocale keyboardLayout keyboardVariant;

    # Locale de región y formatos; zona horaria automática; accesibilidad.
    region = settings.region or null;
    timeZoneAuto = settings.timeZoneAuto or false;
    a11y = settings.a11y or { };

    inherit username;
    userdescription = settings.userdescription or null;
    # Derivado del usuario: no se guarda en ningún archivo.
    hashedPasswordFile =
        if username == null then null else "/persist/secrets/${username}.hash";
    gitName = settings.gitName or null;
    gitEmail = settings.gitEmail or null;
}
