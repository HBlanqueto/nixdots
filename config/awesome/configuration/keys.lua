local gears = require("gears")
local awful = require("awful")
local hotkeys_popup = require("awful.hotkeys_popup")

globalkeys = gears.table.join(


 -- [ Volume ]
    awful.key({ }, "XF86AudioRaiseVolume", function ()
       awful.util.spawn("pamixer -i 5") end, {description = "Raise Volume", group = "Volume"}),

    awful.key({ }, "XF86AudioLowerVolume", function ()
       awful.util.spawn("pamixer -d 5") end, {description = "Low Volume", group = "Volume"}),

    awful.key({ }, "XF86AudioMute", function ()  
       awful.util.spawn("pamixer -t") end, {description = "Mute Volume", group = "Volume"}),


-- [ Rofi ]
    awful.key({ altkey, }, "space", function () 
       awful.util.spawn("rofi -show drun") end, {description = "Show apps", group = "Rofi"}),


-- [ AwesomeWM ]
    awful.key({ modkey, }, "s", hotkeys_popup.show_help,
       {description="Show help", group="Awesome"}),

    awful.key({ modkey, "Control" }, "r", 
       awesome.restart, {description = "Reload awesome", group = "Awesome"}),

    awful.key({ altkey, shift }, "q", 
       awesome.quit, {description = "Quit awesome", group = "Awesome"}),
    
    awful.key({ modkey, }, "w", function ()
       mymainmenu:show() end, {description = "Show main menu", group = "Awesome"}),

    awful.key({ modkey }, "x",
       function ()
           awful.prompt.run {
             prompt       = "Run Lua code: ",
             textbox      = awful.screen.focused().mypromptbox.widget,
             exe_callback = awful.util.eval,
             history_path = awful.util.get_cache_dir() .. "/history_eval"
           }
       end,
       {description = "Lua execute prompt", group = "Awesome"}),


-- [ Tags ]
    awful.key({ modkey, }, "Left", awful.tag.viewprev,
       {description = "View previous", group = "Tag"}),

    awful.key({ modkey, }, "Right", awful.tag.viewnext,
       {description = "View next", group = "Tag"}),

    awful.key({ modkey, }, "Escape", awful.tag.history.restore,
       {description = "Go back", group = "Tag"}),


-- [ Client ]       
    awful.key({ modkey, }, "j", function ()
       awful.client.focus.byidx( 1) end, {description = "Focus next by index", group = "Client"}),

    awful.key({ modkey, }, "k", function ()
       awful.client.focus.byidx(-1) end, {description = "Focus previous by index", group = "Client"}),

    awful.key({ modkey, shift   }, "j", function () 
        awful.client.swap.byidx(  1) end, {description = "Swap with next client by index", group = "Client"}),
 
    awful.key({ modkey, shift   }, "k", function () 
        awful.client.swap.byidx( -1) end, {description = "Swap with previous client by index", group = "Client"}),

    awful.key({ modkey, }, "u", 
        awful.client.urgent.jumpto, {description = "Jump to urgent client", group = "Client"}),

    awful.key({ modkey, "Control" }, "n",
       function ()
            local c = awful.client.restore()
            -- Focus restored client
            if c then
              c:emit_signal(
                  "request::activate", "key.unminimize", {raise = true}
              )
            end
        end,
        {description = "Restore minimized", group = "Client"}),

    awful.key({ modkey, }, "Tab", function ()
            awful.client.focus.history.previous()
                 if client.focus then
                     client.focus:raise()
                 end
             end,
         {description = "go back", group = "Client"}),


-- [ Layout ]
    awful.key({ modkey, }, "l", function () 
       awful.tag.incmwfact( 0.05) end, {description = "increase master width factor", group = "Layout"}),

    awful.key({ modkey, }, "h", function () 
       awful.tag.incmwfact(-0.05) end, {description = "decrease master width factor", group = "Layout"}),

    awful.key({ modkey, shift }, "h", function ()
       awful.tag.incnmaster( 1, nil, true) end, {description = "increase the number of master clients", group = "Layout"}),

    awful.key({ modkey, shift }, "l", function () 
       awful.tag.incnmaster(-1, nil, true) end, {description = "decrease the number of master clients", group = "Layout"}),

    awful.key({ modkey, ctrl }, "h", function () 
       awful.tag.incncol( 1, nil, true) end, {description = "Increase the number of columns", group = "Layout"}),

    awful.key({ modkey, ctrl }, "l", function ()
      awful.tag.incncol(-1, nil, true) end, {description = "Decrease the number of columns", group = "Layout"}),

    awful.key({ modkey, }, "space", function ()
       awful.layout.inc( 1) end, {description = "Select next", group = "Layout"}),

    awful.key({ modkey, shift }, "space", function () 
       awful.layout.inc(-1) end, {description = "Select previous", group = "Layout"}),


-- [ Screen ]
    awful.key({ modkey, ctrl }, "j", function () 
       awful.screen.focus_relative( 1) end, {description = "Focus the next screen", group = "Screen"}),

    awful.key({ modkey, ctrl }, "k", function ()
       awful.screen.focus_relative(-1) end, {description = "Focus the previous screen", group = "Screen"}),


-- [ Launcher ]
    awful.key({ modkey, }, "Return", function () -- Standart Program
       awful.spawn(terminal) end, {description = "Open a terminal", group = "Launcher"}),

    awful.key({ modkey }, "r", function () awful.screen.focused().mypromptbox:run() end, -- Prompt
              {description = "run prompt", group = "Launcher"}),

    awful.key({ modkey }, "p", function() menubar.show() end,
              {description = "show the menubar", group = "Launcher"})
)


clientkeys = gears.table.join(

    awful.key({ modkey, }, "f", function (c)
       c.fullscreen = not c.fullscreen c:raise() end, {description = "toggle fullscreen", group = "Client"}),

    awful.key({ altkey }, "q", function (c) 
       c:kill() end, {description = "Close Window", group = "Client"}),

    awful.key({ modkey, ctrl }, "space",  awful.client.floating.toggle,
       {description = "Toggle floating", group = "Client"}),

    awful.key({ modkey, ctrl }, "Return", function (c) 
       c:swap(awful.client.getmaster()) end, {description = "Move to master", group = "Client"}),

    awful.key({ modkey, }, "o", function (c) c:move_to_screen() end,
       {description = "Move to screen", group = "Client"}),

    awful.key({ modkey, }, "t", function (c) c.ontop = not c.ontop end,
       {description = "Toggle keep on top", group = "Client"}),

    awful.key({ modkey, }, "n", function (c)
        c.minimized = true end, {description = "Minimize", group = "Client"}),

    awful.key({ modkey, }, "m", function (c)
       c.maximized = not c.maximized c:raise() end, {description = "(Un)maximize", group = "Client"}),

    awful.key({ modkey, ctrl }, "m", function (c)
       c.maximized_vertical = not c.maximized_vertical c:raise() end, {description = "(Un)maximize vertically", group = "Client"}),

    awful.key({ modkey, shift }, "m", function (c)
       c.maximized_horizontal = not c.maximized_horizontal c:raise() end, {description = "(Un)maximize horizontally", group = "Client"})

)

-- [ Bind all key numbers to tags ]
-- Be careful: we use keycodes to make it work on any keyboard layout.
-- This should map on the top row of your keyboard, usually 1 to 9.
for i = 1, 9 do
    globalkeys = gears.table.join(globalkeys,
        -- View tag only.
        awful.key({ modkey }, "#" .. i + 9,
                  function ()
                        local screen = awful.screen.focused()
                        local tag = screen.tags[i]
                        if tag then
                           tag:view_only()
                        end
                  end,
                  {description = "view tag #"..i, group = "tag"}),
        -- Toggle tag display.
        awful.key({ modkey, "Control" }, "#" .. i + 9,
                  function ()
                      local screen = awful.screen.focused()
                      local tag = screen.tags[i]
                      if tag then
                         awful.tag.viewtoggle(tag)
                      end
                  end,
                  {description = "toggle tag #" .. i, group = "tag"}),
        -- Move client to tag.
        awful.key({ modkey, "Shift" }, "#" .. i + 9,
                  function ()
                      if client.focus then
                          local tag = client.focus.screen.tags[i]
                          if tag then
                              client.focus:move_to_tag(tag)
                          end
                     end
                  end,
                  {description = "move focused client to tag #"..i, group = "tag"}),
        -- Toggle tag on focused client.
        awful.key({ modkey, "Control", "Shift" }, "#" .. i + 9,
                  function ()
                      if client.focus then
                          local tag = client.focus.screen.tags[i]
                          if tag then
                              client.focus:toggle_tag(tag)
                          end
                      end
                  end,
                  {description = "Toggle focused client on tag #" .. i, group = "Tag"})
    )
end

clientbuttons = gears.table.join(
    awful.button({ }, 1, function (c)
        c:emit_signal("request::activate", "mouse_click", {raise = true})
    end),
    awful.button({ modkey }, 1, function (c)
        c:emit_signal("request::activate", "mouse_click", {raise = true})
        awful.mouse.client.move(c)
    end),
    awful.button({ modkey }, 3, function (c)
        c:emit_signal("request::activate", "mouse_click", {raise = true})
        awful.mouse.client.resize(c)
    end)
)

root.keys(globalkeys)