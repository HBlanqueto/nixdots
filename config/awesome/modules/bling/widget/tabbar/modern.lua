local awful = require("awful")
local gears = require("gears")
local wibox = require("wibox")
local beautiful = require("beautiful")
local xresources = require("beautiful.xresources")
local dpi = xresources.apply_dpi
local helpers = require(tostring(...):match(".*bling") .. ".helpers")

local bg_normal     = "#0d1117"
local fg_normal     = "#C6CDD5"
local bg_focus      = "#010409"
local fg_focus      = "#ffffff"
local font          = "Inter 9"
local size          = dpi(39)
local border_radius = 8
local position      = "top"
local close_color   = "#FF958E"
local min_color     = "#FBDF90"
local float_color   = "#E3C9FF"

-- Helper to create buttons
local function create_title_button(c, color_focus, color_unfocus)
    local tb_color = wibox.widget {
        wibox.widget.textbox(),
        forced_width = dpi(8),
        forced_height = dpi(8),
        bg = color_focus,
        shape = gears.shape.circle,
        widget = wibox.container.background
    }

    local tb = wibox.widget {
        tb_color,
        width = dpi(23),
        height = dpi(23),
        strategy = "min",
        layout = wibox.layout.constraint
    }

    local function update()
        if client.focus == c then
            tb_color.bg = color_focus
        else
            tb_color.bg = color_unfocus
        end
    end
    update()
    c:connect_signal("focus", update)
    c:connect_signal("unfocus", update)

    tb:connect_signal("mouse::enter",
                      function() tb_color.bg = color_focus .. "70" end)

    tb:connect_signal("mouse::leave", function() tb_color.bg = color_focus end)

    tb.visible = true
    return tb
end

local function create(c, focused_bool, buttons)
    -- local flexlist = wibox.layout.flex.horizontal()
    local title_temp = c.name or c.class or "-"
    local bg_temp = bg_normal
    local fg_temp = fg_normal
    if focused_bool then
        bg_temp = bg_focus
        fg_temp = fg_focus
    end
    local text_temp = wibox.widget.textbox()
    text_temp.align = "center"
    text_temp.valign = "center"
    text_temp.font = font
    text_temp.markup = "<span foreground='" .. fg_temp .. "'>" .. title_temp ..
                           "</span>"
    c:connect_signal("property::name", function(_)
        local title_temp = c.name or c.class or "-"
        text_temp.markup =
            "<span foreground='" .. fg_temp .. "'>" .. title_temp .. "</span>"
    end)

    local tab_content = wibox.widget {
        {
            top = dpi(6),
            left = dpi(15),
            bottom = dpi(6),
            widget = wibox.container.margin
        },
        text_temp,
        nill,
        awful.widget.clienticon(c),
        expand = "none",
        layout = wibox.layout.align.horizontal
    }

    local close = create_title_button(c, close_color, bg_normal)
    close:connect_signal("button::press", function() c:kill() end)

    local floating = create_title_button(c, float_color, bg_normal)
    floating:connect_signal("button::press",
                            function() c.floating = not c.floating end)

    local min = create_title_button(c, min_color, bg_normal)
    min:connect_signal("button::press", function() c.minimized = true end)

    if focused_bool then
        tab_content = wibox.widget {
            {
                {close, floating, min, layout = wibox.layout.fixed.horizontal},
                top = dpi(10),
                left = dpi(15),
                bottom = dpi(10),
                widget = wibox.container.margin
            },
            text_temp,
            {
                awful.widget.clienticon(c),
                top = dpi(10),
                right = dpi(10),
                bottom = dpi(10),
                widget = wibox.container.margin
            },
            expand = "none",
            layout = wibox.layout.align.horizontal
        }
    end

    local main_content = nil
    local left_shape = nil
    local right_shape = nil

    if position == "top" then
        main_content = wibox.widget {
            {
                tab_content,
                bg = bg_temp,
                shape = helpers.shape.prrect(border_radius, true, true, false,
                                             false),
                widget = wibox.container.background
            },
            top = dpi(8),
            widget = wibox.container.margin
        }

        left_shape = helpers.shape.prrect(border_radius, false, false, true,
                                          false)
        right_shape = helpers.shape.prrect(border_radius, false, false, false,
                                           true)
    else
        main_content = wibox.widget {
            {
                tab_content,
                bg = bg_temp,
                shape = helpers.shape.prrect(border_radius, false, false, true,
                                             true),
                widget = wibox.container.background
            },
            bottom = dpi(8),
            widget = wibox.container.margin
        }

        left_shape = helpers.shape.prrect(border_radius, false, true, false,
                                          false)
        right_shape = helpers.shape.prrect(border_radius, true, false, false,
                                           false)
    end

    local wid_temp = wibox.widget({
        buttons = buttons,
        {
            {
                {
                    wibox.widget.textbox(),
                    bg = bg_normal,
                    shape = left_shape,
                    widget = wibox.container.background
                },
                bg = bg_temp,
                shape = gears.rectangle,
                widget = wibox.container.background
            },
            width = border_radius + (border_radius / 2),
            height = size,
            strategy = "exact",
            layout = wibox.layout.constraint
        },
        main_content,
        {
            {
                {
                    wibox.widget.textbox(),
                    bg = bg_normal,
                    shape = right_shape,
                    widget = wibox.container.background
                },
                bg = bg_temp,
                shape = gears.rectangle,
                widget = wibox.container.background
            },
            width = border_radius + (border_radius / 2),
            height = size,
            strategy = "exact",
            layout = wibox.layout.constraint
        },

        layout = wibox.layout.align.horizontal
    })
    return wid_temp
end

return {
    layout = wibox.layout.flex.horizontal,
    create = create,
    position = position,
    size = size,
    bg_normal = bg_normal,
    bg_focus = bg_focus
}
