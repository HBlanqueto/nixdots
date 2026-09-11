{ stdenvNoCC, inputs, ... }:

stdenvNoCC.mkDerivation {
    pname = "sf-mono-liga-bin";
    version = "dev";

    src = inputs.sf-mono-liga-src;

    dontConfigure = true;
    dontBuild = true;

    installPhase = ''
        mkdir -p $out/share/fonts/opentype
        cp -R $src/*.otf $out/share/fonts/opentype/
    '';

    meta = {
        description = "Apple SFMono font with ligatures and Nerd Font patches";
    };
}