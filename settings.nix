{
    # Value defaults del sistema. Los valores de cada máquina (hostName, locale,
    # zona, teclado…) viven en ${NIXDOTS_ROOT}/persist/nixdots/machine.json y
    # mandan sobre estos; las cuentas, en users/<usuario>.json. Aquí solo
    # stateVersion (que no cambia) y los defaults de arriba.
    stateVersion = "26.05";

    timeZone = "America/Merida";
    defaultLocale = "es_MX.UTF-8";
    keyboardLayout = "es";
    keyboardVariant = "";
}
