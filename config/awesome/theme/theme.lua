local theme_assets = require("beautiful.theme_assets")
local xresources = require("beautiful.xresources")
local beautiful = require("beautiful")

local dpi = xresources.apply_dpi
local xrdb = xresources.get_current_theme()

local gears = require("gears")
local gfs = require("gears.filesystem")

local theme = {}

-- [ Images ] 
--theme.wallpaper = gfs.get_configuration_dir() .. "theme/images/space-dark.png"
theme.me = gfs.get_configuration_dir() .. "theme/images/me.png"
--theme.me = gears.surface.load_uncached(gfs.get_configuration_dir() .. "images/me.png")

-- [ Xresources ]
-- But I don't use them, heh.
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

-- [ Exitscreen ]
theme.exit_screen_fg = theme.xcolor15
theme.exit_screen_bg = theme.xcolor8

-- [ Lockscreen ]
theme.widget_border_radius = dpi(7)

-- [ Fonts ]
theme.icon_font_name = "JetBrainsMono Nerd Font "
theme.font_name      = "Cantarell "
theme.font           = theme.font_name .. "9"
theme.titlebar_font  = theme.font_name .. "9"
theme.fg_normal      = theme.xcolor15
theme.fg_focus       = theme.xcolor7
theme.fg_urgent      = theme.xcolor7
theme.fg_minimize    = theme.xcolor7

-- [ Windows ]
theme.bg_focus      = theme.xcolor0
theme.bg_normal     = theme.xcolor8
theme.bg_urgent     = theme.xcolor8
theme.bg_minimize   = theme.xcolor0
theme.border_normal = theme.xcolor16
theme.border_focus  = theme.xcolor16
theme.border_marked = theme.xcolor16
theme.useless_gap   = dpi(7)
theme.border_width  = dpi(1) 

theme.menu_height = dpi(20)
theme.menu_width  = dpi(140)

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
