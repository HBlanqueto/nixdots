local dpi = require("beautiful.xresources").apply_dpi
local menubar = require("menubar")

apps = {

  terminal        = "wezterm",

  editor          = os.getenv("EDITOR") or "vim",

  explorer        = "thunar",

  browser         = "google-chrome-unstable",
}

apps.editor_cmd   = apps.terminal .. " -e " .. apps.editor
apps.explorer_cmd = apps.terminal .. " -e " .. apps.explorer
menubar.utils.terminal = apps.terminal -- Set the terminal for applications that require it


return apps
