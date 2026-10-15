{
    # Host-independent values. Per-host values (system, hostName) live in
    # hosts/<host>/default.nix, not here. Los datos de usuario viven en
    # ${NIXDOTS_ROOT}/persist/nixdots/user.json, no aquí.
    stateVersion = "26.05";

    timeZone = "America/Merida";
    defaultLocale = "es_MX.UTF-8";
    keyboardLayout = "es";
    keyboardVariant = "";
}
