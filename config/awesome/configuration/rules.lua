local gears = require("gears")
local awful = require("awful")
local beautiful = require("beautiful")

awful.rules.rules = {

    {
        rule = {},
        properties = {

            border_width = beautiful.border_width,
            border_color = beautiful.border_normal,
            focus = awful.client.focus.filter,
            raise = true,
            keys = clientkeys,
            buttons = clientbuttons,
            screen = awful.screen.preferred,
            placement = awful.placement.centered

        }
    }, {
        rule_any = {
            instance = {"DTA", "copyq", "pinentry"},

            class = {
                "Arandr", "Blueman-manager", "Gpick", "Kruler", "MessageWin",
                "Sxiv", "Tor Browser", "Wpa_gui", "veromix", "xtightvncviewer"
            },

            name = {"Event Tester"},
            role = {"AlarmWindow", "ConfigManager", "pop-up"}
        },

        properties = {floating = true}
    }, {
        rule_any = {type = {"normal", "dialog"}},

        properties = {titlebars_enabled = true}
    }, {rule = {class = "Code"}, properties = {titlebars_enabled = false}}

}
