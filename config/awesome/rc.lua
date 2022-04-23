-- [ Awesome libraries ]
pcall(require, "luarocks.loader")

local awful = require("awful")
local beautiful = require("beautiful")
local gears = require("gears")
local gfs = require("gears.filesystem")


-- [ Theme ]
beautiful.init(gfs.get_configuration_dir() .. "theme/theme.lua")


-- [ My modules ]
require("configuration")
require('ui')


-- [ Bling ]
local bling = require("modules.bling")
awful.screen.connect_for_each_screen(function(s)
    -- ﰕ
    -- 
    bling.module.tiled_wallpaper("", s, {
        fg = beautiful.xcolor17,
        bg = beautiful.xcolor8,
        offset_y = 35,
        offset_x = 35,
        font = "JetBrainsMono Nerd Font",
        font_size = 14,
        padding = 105,
        zickzack = true
    })
end)