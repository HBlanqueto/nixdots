{}: 

''

local col = {}

col.foreground = "#C6CDD5"
col.background = "#010409"
col.cursor_bg = "#C6CDD5"
col.cursor_fg = "#C6CDD5"
col.cursor_border = "#C6CDD5"
col.split = "#3B4B58"

col.ansi = {

    "#3B4B58", "#FF958E", "#9DFAAA", "#FBDF90", "#BDfBff", "#E3C9FF", "#B8FFB2", "#F6FAFD"
}
col.brights = {
    "#363B42", "#ea746c", "#7CE38B", "#D9BE74", "#BEDFE8", "#BD89F5", "#94E4A5", "#F6FAFD"
}

col.tab_bar = {
    background = "#010409",
    inactive_tab_edge = "#010409",

    active_tab = {
        bg_color = "#131a21",
        fg_color = "#C6CDD5",
        intensity = "Half"
    },
    inactive_tab = {bg_color = "#010409", fg_color = "#3B4B58"},
    inactive_tab_hover = {bg_color = "#010409", fg_color = "#3B4B58"}
}

return col

''