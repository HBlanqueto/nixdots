{...}: {

  imports = [ 
    ./modules/init.nix
    ./modules/tabs.nix
    ./modules/keys.nix

    ./modules/nighthub/init.nix
    ./modules/nighthub/theme.nix

    ./lib/signals.nix
    ./lib/mytable.nix
  ];

home.file.".config/wezterm/wezterm.lua".text = ''

local wezterm = require('wezterm')
local mytable = require('lib/mytable').mytable

-- Local configuration
local theme = require('modules.nighthub')
local modules = require('modules')
local colors = {colors = theme.colors}
local tabconf = modules.tabs
local keys = modules.keys

function font_with_fallback(name, params)
    local names = {name, 'JetBrains Mono', 'Blobmoji'}
    return wezterm.font_with_fallback(names, params)
end

local base = {

    -- OpenGL for GPU acceleration
    front_end = 'OpenGL',
    prefer_egl = true,

    -- Font Stuff
    font = font_with_fallback('JetBrainsMono Nerd Font'),
    font_rules = {
        {
            italic = true,
            font = font_with_fallback('JetBrainsMono Nerd Font Italic',
                                      {italic = true})
        }, {
            italic = true,
            intensity = 'Bold',
            font = font_with_fallback('JetBrainsMono Nerd Font Bold',
                                      {bold = true, italic = true})
        }, {
            intensity = 'Bold',
            font = font_with_fallback('JetBrainsMono Nerd Font', {bold = true})
        },
        {
            intensity = 'Half',
            font = font_with_fallback('JetBrainsMono Nerd Font')
        }
    },

    -- No updates
    check_for_updates = false,

    -- Basic font configuration
    font_size = 9.0,
    font_shaper = 'Harfbuzz',
    line_height = 1.0,
    freetype_load_target = 'HorizontalLcd',
    freetype_render_target = 'HorizontalLcd',

    -- Cursor style
    default_cursor_style = 'BlinkingBar',

    -- Activate this in Gnome
    enable_wayland = true,

    -- Bright bold colors
    bold_brightens_ansi_colors = true,

    -- Get rid of close prompt
    window_close_confirmation = 'NeverPrompt',

    -- Padding
    window_padding = {left = 18, right = 18, top = 18, bottom = 18},

    -- Opacity
    inactive_pane_hsb = {saturation = 1.0, brightness = 1.0}

}

-- Merge everything and return
local config = mytable.merge_all(base, tabconf, keys, colors)

return config

'';
}