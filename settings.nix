# NOTE: Modifying these variables will affect the global configuration.
# This file is the single source of truth for all global user values.
# Do not hardcode these elsewhere; inherit them via specialArgs.

{
    system = "x86_64-linux";
    stateVersion = "26.05";

    hostName = "nixos";
    timeZone = "America/Merida";
    defaultLocale = "es_MX.UTF-8";

    username = "humbe";
    userdescription = "H. Blanqueto";
    hashedpassword = "$6$rvI2ZNaKpc69XPeZ$R6iSUJ3l7iYlFc6eJz4pue1cl51d0H0dBNYkJcTm5BddRohQkdCC7sHmS50UczcPKESV//lw0CpO071roxsB21";

    # Git user details. This modifies home-manager data in ./home/humbe/apps.nix
    gitName = "H. Blanqueto";
    gitEmail = "mc4w6wmkrv@privaterelay.appleid.com";
}