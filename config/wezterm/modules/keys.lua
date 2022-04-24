local wezterm = require("wezterm")
local keys = {}

keys.disable_default_key_bindings = true

keys.keys = {
    {
        mods = "CTRL",
        key = [[p]],
        action = wezterm.action {
            SplitHorizontal = {domain = "CurrentPaneDomain"}
        }
    }, {
        mods = "CTRL",
        key = [[ñ]],
        action = wezterm.action {SplitVertical = {domain = "CurrentPaneDomain"}}
    }, -- browser-like bindings for tabbing
    {
        key = "t",
        mods = "CTRL",
        action = wezterm.action {SpawnTab = "CurrentPaneDomain"}
    }, {
        key = "w",
        mods = "CTRL",
        action = wezterm.action {CloseCurrentTab = {confirm = false}}
    },
    {
        mods = "CTRL",
        key = "Tab",
        action = wezterm.action {ActivateTabRelative = 1}
    }, {
        mods = "CTRL|SHIFT",
        key = "Tab",
        action = wezterm.action {ActivateTabRelative = -1}
    }, -- standard copy/paste bindings
    {key = "x", mods = "CTRL", action = "ActivateCopyMode"}, {
        key = "v",
        mods = "CTRL|SHIFT",
        action = wezterm.action {PasteFrom = "Clipboard"}
    }, {
        key = "c",
        mods = "CTRL|SHIFT",
        action = wezterm.action {CopyTo = "ClipboardAndPrimarySelection"}
    }
}

return keys
