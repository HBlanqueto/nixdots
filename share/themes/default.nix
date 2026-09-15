rec {
    colors =
        import ./neptunia.nix;

    filecolors =
        import ./filecolors.nix;

    vscode =
        import ./vscode.nix {
            inherit colors filecolors;
        };
}