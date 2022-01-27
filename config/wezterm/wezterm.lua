local wezterm = require('wezterm')
local mytable = require("lib/mystdlib").mytable
local theme = require('theme')
local gui = require('gui')

local base = {

    -- OpenGL for GPU acceleration, Software for CPU
    front_end = "OpenGL",

    -- No updates
    check_for_updates = false,

    -- Basic font configuration
    font_size = 9.0,
    font_shaper = "Harfbuzz",
    line_height = 1.0,
    freetype_load_target = "HorizontalLcd",
    freetype_render_target = "HorizontalLcd",

    -- Cursor style
    default_cursor_style = "SteadyUnderline",

    -- Wayland
    enable_wayland = false,

    -- Bright bold colors
    bold_brightens_ansi_colors = true,

    -- Get rid of close prompt
    window_close_confirmation = "NeverPrompt",

    -- Padding
    window_padding = {left = 10, right = 10, top = 10, bottom = 10},

    -- Opacity
    inactive_pane_hsb = {saturation = 1.0, brightness = 1.0},
    
}

-- Colors
local themes = {colors = theme.colors}

-- Tab Style (like shape)
local tabs = gui.tabs

-- Keys
local keys = gui.keys

-- Merge everything and return
local config = mytable.merge_all(base, themes, tabs, keys)
return config
