pcall(require, "luarocks.loader")

-- Standard awesome library
local awful = require("awful")

-- Theme handling library
local bling = require("extra.bling")
local beautiful = require("beautiful")

-- Notification library
local naughty = require("naughty")

-- Filesystem
local gfs = require("gears.filesystem")
local themes_path = gfs.get_configuration_dir()
local gears = require("gears")

require("awful.hotkeys_popup.keys")


-- Initialize selected theme.
beautiful.init(themes_path .. "theme/nighthub.lua")

-- Init all modules (You can add/remove active modules here)
require("modules.auto-start")

-- Setup UI Elements
require('ui')

-- Setup all configurations
require('configuration.tags')
require('configuration.client')
require('configuration.init')
_G.root.keys(require('configuration.keys.global'))
_G.root.buttons(require('configuration.mouse.desktop'))

-- Bling Wallpaper
awful.screen.connect_for_each_screen(function(s) -- that way the wallpaper is applied to every screen

    -- ﰕ
    -- 
    bling.module.tiled_wallpaper("", s, {
        fg = "#4c5b75", -- "#262F3F",
        bg = "#010409",
        offset_y = 28,
        offset_x = 28,
        font = "JetBrainsMono Nerd Font",
        font_size = 14,
        padding = 90,
        zickzack = true
    })
end)

-- {{{ Error handling
-- Check if awesome encountered an error during startup and fell back to
-- another config (This code will only ever execute for the fallback config)
if awesome.startup_errors then
    naughty.notify({ preset = naughty.config.presets.critical,
                     title = "Oops, there were errors during startup!",
                     text = awesome.startup_errors })
end

-- Handle runtime errors after startup
do
    local in_error = false
    awesome.connect_signal("debug::error", function (err)
        -- Make sure we don't go into an endless error loop
        if in_error then return end
        in_error = true

        naughty.notify({ preset = naughty.config.presets.critical,
                         title = "Oops, an error happened!",
                         text = tostring(err) })
        in_error = false
    end)
end
-- }}}

-- {{{ Signals
-- Signal function to execute when a new client appears.
client.connect_signal("manage", function (c)
    -- Set the windows at the slave,
    -- i.e. put it at the end of others instead of setting it master.
    -- if not awesome.startup then awful.client.setslave(c) end

    if awesome.startup
      and not c.size_hints.user_position
      and not c.size_hints.program_position then
        -- Prevent clients from being unreachable after screen count changes.
        awful.placement.no_offscreen(c)
    end
end)