{
    # Host-independent values. Per-host values (system, hostName) live in
    # hosts/<host>/default.nix, not here.
    stateVersion = "26.05";

    timeZone = "America/Merida";
    defaultLocale = "es_MX.UTF-8";

    username = "humbe";
    userdescription = "H. Blanqueto";
    hashedPasswordFile = "/persist/secrets/humbe.hash";

    gitName = "H. Blanqueto";
    gitEmail = "mc4w6wmkrv@privaterelay.appleid.com";
}
