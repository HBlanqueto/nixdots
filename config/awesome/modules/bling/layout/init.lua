local beautiful = require("beautiful")

local mstab = require(... .. ".mstab")
beautiful.layout_mstab = mstab.get_icon()

local layout = {
   -- mstab = mstab.layout
}

return layout
