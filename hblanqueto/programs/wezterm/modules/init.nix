{...}: {

home.file.".config/wezterm/modules/init.lua".text = ''

local config = {}

-- require('lib.signals')

config.keys = require('modules.keys')
config.tabs = require('modules.tabs')

return config

'';
}