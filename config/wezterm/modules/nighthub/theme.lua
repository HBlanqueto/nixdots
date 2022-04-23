local col = {}

col.foreground = "#C6CDD5"
col.background = "#010409"
col.cursor_bg = "#a3b8ef"
col.cursor_fg = "#a3b8ef"
col.cursor_border = "#a3b8ef"
col.split = "#3b4b58"

col.ansi = {
 
    "#3b4b58", "#FF958E", "#9DFAAA", "#FBDF90", "#CEE9FF", "#E3C9FF", "#A5D5FF",
    "#F6FAFD"
}
col.brights = {
    "#363B42", "#FA7970", "#7CE38B", "#FAA356", "#CEE9FF", "#E3C9FF", "#A5D5FF",
    "#F6FAFD"
}

col.tab_bar = {
    background = "#010409",
    inactive_tab_edge = "#010409",

    active_tab = {
        bg_color = "#131a21",        
        fg_color = "#C6CDD5",
        intensity = "Half"
    },
    inactive_tab = {
        bg_color = "#010409",
        fg_color = "#3b4b58"
    },
    inactive_tab_hover = {
        bg_color = "#010409",
        fg_color = "#3b4b58"
    }
}

return col
