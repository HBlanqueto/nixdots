# Datos compartidos que cada nixosSystem recibe por specialArgs, derivados de
# settings.nix. En el primer arranque settings.nix no trae `username` todavía,
# así que los datos de usuario caen a null y user.nix/home.nix/ai.nix omiten la
# cuenta; el resto (stateVersion, locale, teclado) sigue siendo obligatorio.
{ settings }:
{
    inherit settings;

    inherit (settings) stateVersion timeZone defaultLocale keyboardLayout keyboardVariant;

    username = settings.username or null;
    userdescription = settings.userdescription or null;
    hashedPasswordFile = settings.hashedPasswordFile or null;
    gitName = settings.gitName or null;
    gitEmail = settings.gitEmail or null;
}
