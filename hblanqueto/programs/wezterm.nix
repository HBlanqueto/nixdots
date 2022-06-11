{ theme }:

with theme;

''

  local wezterm = require('wezterm')
  local font_name = "JetbrainsMono Nerd Font"
  
  function font_with_fallback(name, params)
    local names = {name, "Twitter Color Emoji"}
    return wezterm.font_with_fallback(names, params)
  end


  return {
    -- [ General settings ]
    front_end = 'OpenGL',
  
    enable_wayland = true,
    
    check_for_updates = false, 
    
    bold_brightens_ansi_colors = true,  
   
    default_cursor_style = 'SteadyUnderline',
    
    window_close_confirmation = 'NeverPrompt',
    
    inactive_pane_hsb = {saturation = 1.0, brightness = 1.0},
    
    window_padding = {left = 34, right = 34, top = 23, bottom = 23},
    


    -- [ Tabbar settings ]
    enable_tab_bar = true,
    
    tab_bar_at_bottom = true,
    
    hide_tab_bar_if_only_one_tab = true,
    
    show_tab_index_in_tab_bar = false,

  
    -- [ Font rules stolen from javacafe01 ]
    font_size = 9.0,
    line_height = 1.0,
    
    font = font_with_fallback(font_name),
    font_rules = {
          {
            italic = true,
            font = font_with_fallback(font_name, {italic = true})}, 
          {
            italic = true,
            intensity = 'Bold',
            font = font_with_fallback(font_name, {bold = true, italic = true})}, 
          {
            intensity = 'Bold',
            font = font_with_fallback(font_name, {bold = true})},
          {
            intensity = 'Half',
            font = font_with_fallback(font_name)
          }
        },

    -- [ Theme & tabbars colors ]

      colors = {
          foreground = "#${fg}",
          background = "#${bg}",
          cursor_bg = "#${c4}",
          cursor_fg = "#${c4}",
          cursor_border = "#${c4}",
          split = "#${lbg}",
          ansi = {
              "#${c0}", "#${c1}", "#${c2}", "#${c3}", "#${c4}", "#${c5}",
              "#${c6}", "#${c7}"
          },
          brights = {
              "#${c8}", "#${c9}", "#${c10}", "#${c11}", "#${c12}", "#${c13}",
              "#${c14}", "#${c15}"
          },
          tab_bar = {
              active_tab = {
                  bg_color = "#${bg}",
                  fg_color = "#${c8}",
                  italic = true
              },
              inactive_tab = {bg_color = "#${dbg}", fg_color = "#${c8}"},
              inactive_tab_hover = {bg_color = "#${c0}", fg_color = "#${bg}"}
          }
      },

    -- [ Keybindings ]
      disable_default_key_bindings = true,
      keys = {
            {
              mods = "CTRL",
              key = [[0]],
              action = wezterm.action {
              SplitHorizontal = {domain = "CurrentPaneDomain"}
              }
            }, 
            {
              mods = "CTRL",
              key = [[p]],
              action = wezterm.action {
              SplitVertical = {domain = "CurrentPaneDomain"}
              }
            },
           -- browser-like bindings for tabbing
           {
              key = "t",
              mods = "CTRL",
              action = wezterm.action {SpawnTab = "CurrentPaneDomain"}
           }, 
           {
              key = "w",
              mods = "CTRL",
              action = wezterm.action {CloseCurrentTab = {confirm = false}}
           },
           {
              mods = "CTRL",
              key = "Tab",
              action = wezterm.action {ActivateTabRelative = 1}
           }, 
           {
              mods = "CTRL|SHIFT",
              key = "Tab",
              action = wezterm.action {ActivateTabRelative = -1}
           }, 
           -- standard copy/paste bindings
           {
              key = "v",
              mods = "CTRL|SHIFT",
              action = wezterm.action {PasteFrom = "Clipboard"}
           },
           {
              key = "c",
              mods = "CTRL|SHIFT",
              action = wezterm.action {CopyTo = "ClipboardAndPrimarySelection"}
           }
         }
}''
