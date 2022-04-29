{...}: {

home.file.".config/wezterm/modules/tabs.lua".text = ''

local wezterm = require('wezterm')

local tabs = {}

-- Tab Bar Options
tabs.enable_tab_bar = true
tabs.tab_bar_at_bottom = true
tabs.hide_tab_bar_if_only_one_tab = true
tabs.show_tab_index_in_tab_bar = false

return tabs

'';
}