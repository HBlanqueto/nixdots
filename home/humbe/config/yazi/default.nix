let
    theme = import ../../../../share/themes { };
    icons = import ./icons-white.nix;
in

with theme.colors;

{
    settings = {
        mgr = {
            ratio = [ 0 1 1 ];
        };

        opener = {
            play_ncmpcpp = [
                {
                    run = "mpc clear && mpc add \"file://$@\" && mpc play && ncmpcpp";
                    block = true;
                }
            ];
        };

        open = {
            rules = [
                {
                    mime = "audio/*";
                    use = [ "play_ncmpcpp" ];
                }
                {
                    url = "*.mp3";
                    use = [ "play_ncmpcpp" ];
                }
            ];
        };
    };

    theme = {
        mgr = {
            border_style = {
                fg = "white";
            };
            border_symbol = " ";
        };

        indicator = {
            padding = {
                open = "█";
                close = "█";
            };

            current = { 
                reversed = false; 
                fg = "#${bg}";
                bg = "#ffffff";
            };

            parent = {
                reversed = false;
                fg = "#${bg}";
                bg = "#ffffff";
            };
        };

        filetype = {
            rules = [
                {
                    url = "*";
                    is = "orphan";
                    fg = "white";
                }
                {
                    url = "*";
                    is = "exec";
                    fg = "white";
                }
                {
                    url = "*/";
                    fg = "white";
                }
                {
                    url = "*";
                    fg = "white";
                }
            ];
        };

        icon = icons;
    };
}