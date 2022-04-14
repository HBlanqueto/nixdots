local theme_assets = require("beautiful.theme_assets")
local xresources = require("beautiful.xresources")
local beautiful = require("beautiful")

local dpi = xresources.apply_dpi
local xrdb = xresources.get_current_theme()

local gfs = require("gears.filesystem")

local theme = {}

theme.xbackground = "#010409"
theme.xforeground = "#C6CDD5"

theme.xcolor0  = "#010409"
theme.xcolor1  = "#FF958E"
theme.xcolor2  = "#9DFAAA"
theme.xcolor3  = "#FBDF90"
theme.xcolor4  = "#CEE9FF"
theme.xcolor5  = "#E3C9FF"
theme.xcolor6  = "#A5D5FF"
theme.xcolor7  = "#F6FAFD"
theme.xcolor8  = "#0d1117"
theme.xcolor9  = "#FA7970"
theme.xcolor10 = "#7CE38B" 
theme.xcolor11 = "#FAA356" 
theme.xcolor12 = "#CEE9FF"
theme.xcolor13 = "#E3C9FF"
theme.xcolor14 = "#A5D5FF"
theme.xcolor15 = "#C6CDD5"
theme.xcolor16 = "#30363d"

--theme.wallpaper = gfs.get_configuration_dir() .. "theme/images/space-dark.png"

theme.nerd_font_name = "JetBrainsMono Nerd Font "
theme.font_name      = "Cantarell "
theme.font           = theme.font_name .. "10"

theme.bg_focus      = theme.xcolor0
theme.bg_normal     = theme.xcolor8
theme.bg_urgent     = theme.xcolor8
theme.bg_minimize   = theme.xcolor0

theme.fg_normal     = theme.xcolor15
theme.fg_focus      = theme.xcolor7
theme.fg_urgent     = theme.xcolor7
theme.fg_minimize   = theme.xcolor7

theme.useless_gap   = dpi(7)
theme.border_width  = dpi(1) 
theme.border_normal = theme.xcolor16
theme.border_focus  = theme.xcolor16
theme.border_marked = theme.xcolor16

theme.menu_height = dpi(20)
theme.menu_width  = dpi(140)

-- tabbar general
theme.tabbar_ontop = false
theme.tabbar_radius = 0 -- border radius of the tabbar
theme.tabbar_style = "default" -- style of the tabbar ("default", "boxes" or "modern")
theme.tabbar_font = "Sans 11" -- font of the tabbar
theme.tabbar_size = 40 -- size of the tabbar
theme.tabbar_position = "top" -- position of the tabbar
theme.tabbar_bg_normal = "#000000" -- background color of the focused client on the tabbar
theme.tabbar_fg_normal = "#ffffff" -- foreground color of the focused client on the tabbar
theme.tabbar_bg_focus = "#1A2026" -- background color of unfocused clients on the tabbar
theme.tabbar_fg_focus = "#ff0000" -- foreground color of unfocused clients on the tabbar

-- mstab
theme.mstab_bar_ontop = false -- whether you want to allow the bar to be ontop of clients
theme.mstab_dont_resize_slaves = false -- whether the tabbed stack windows should be smaller than the
-- currently focused stack window (set it to true if you use
-- transparent terminals. False if you use shadows on solid ones
theme.mstab_bar_padding = "default" -- how much padding there should be between clients and your tabbar
-- by default it will adjust based on your useless gaps. 
-- If you want a custom value. Set it to the number of pixels (int)
theme.mstab_border_radius = 0 -- border radius of the tabbar
theme.mstab_bar_height = 40 -- height of the tabbar
theme.mstab_tabbar_position = "top" -- position of the tabbar (mstab currently does not support left,right)
theme.mstab_tabbar_style = "default" -- style of the tabbar ("default", "boxes" or "modern")
-- defaults to the tabbar_style so only change if you want a
-- different style for mstab and tabbed

-- the following variables are currently only for the "modern" tabbar style 
theme.tabbar_color_close = "#f9929b" -- chnges the color of the close button
theme.tabbar_color_min = "#fbdf90" -- chnges the color of the minimize button
theme.tabbar_color_float = "#ccaced" -- chnges the color of the float button

-- Generate taglist squares:
local taglist_square_size = dpi(7)
theme.taglist_squares_sel = theme_assets.taglist_squares_sel(
    taglist_square_size, theme.fg_normal
)
theme.taglist_squares_unsel = theme_assets.taglist_squares_unsel(
    taglist_square_size, theme.fg_normal
)

theme.layout_fairh      = gfs.get_configuration_dir() .. "theme/layouts/fairhw.png"
theme.layout_fairv      = gfs.get_configuration_dir() .. "theme/layouts/fairvw.png"
theme.layout_floating   = gfs.get_configuration_dir() .. "theme/layouts/floatingw.png"
theme.layout_magnifier  = gfs.get_configuration_dir() .. "theme/layouts/magnifierw.png"
theme.layout_max        = gfs.get_configuration_dir() .. "theme/layouts/maxw.png"
theme.layout_fullscreen = gfs.get_configuration_dir() .. "theme/layouts/fullscreenw.png"
theme.layout_tilebottom = gfs.get_configuration_dir() .. "theme/layouts/tilebottomw.png"
theme.layout_tileleft   = gfs.get_configuration_dir() .. "theme/layouts/tileleftw.png"
theme.layout_tile       = gfs.get_configuration_dir() .. "theme/layouts/tilew.png"
theme.layout_tiletop    = gfs.get_configuration_dir() .. "theme/layouts/tiletopw.png"
theme.layout_spiral     = gfs.get_configuration_dir() .. "theme/layouts/spiralw.png"
theme.layout_dwindle    = gfs.get_configuration_dir() .. "theme/layouts/dwindlew.png"
theme.layout_cornernw   = gfs.get_configuration_dir() .. "theme/layouts/cornernww.png"
theme.layout_cornerne   = gfs.get_configuration_dir() .. "theme/layouts/cornernew.png"
theme.layout_cornersw   = gfs.get_configuration_dir() .. "theme/layouts/cornersww.png"
theme.layout_cornerse   = gfs.get_configuration_dir() .. "theme/layouts/cornersew.png"

theme.awesome_icon = 
  theme_assets.awesome_icon(
    theme.menu_height,
    theme.bg_focus,
    theme.fg_focus
)

theme.icon_theme = nil

return theme
