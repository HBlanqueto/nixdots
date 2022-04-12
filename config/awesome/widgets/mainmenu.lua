local gears = require("gears")
local awful = require("awful")
local beautiful = require("beautiful")
local hotkeys_popup = require("awful.hotkeys_popup")
require("configuration.apps")

-- {{{ Menu
-- Create a launcher widget and a main menu
myawesomemenu = {
  { "Hotkeys", function() hotkeys_popup.show_help(nil, awful.screen.focused()) end },
  { "Manual", apps.terminal .. " -e man awesome" },
  { "Edit Config", apps.editor_cmd .. " " .. awesome.conffile },
  { "Restart Session", awesome.restart },
  { "Quit", function() awesome.quit() end },
}

mymainmenu = awful.menu({ items = { 
  { "Awesome", myawesomemenu, beautiful.awesome_icon },
  { "Open terminal", apps.terminal },
  { "Browser", apps.browser }
}
                       })


