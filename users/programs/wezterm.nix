{}:

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
      foreground = "#C6CDD5",
      background = "#131a21",
      cursor_bg = "#C6CDD5",
      cursor_fg = "#C6CDD5",
      cursor_border = "#C6CDD5",
      split = "#3B4B58",
      
      ansi = {
        "#3B4B58", "#FF958E", "#9DFAAA", "#FBDF90", "#BDfBff", "#E3C9FF", "#B8FFB2", "#F6FAFD"
      },
      
      brights = {
        "#363B42", "#ea746c", "#7CE38B", "#D9BE74", "#BEDFE8", "#BD89F5", "#94E4A5", "#F6FAFD"
       },
      tab_bar = {
        background = "#010409",
        inactive_tab_edge = "#010409",
      
      active_tab = {
        bg_color = "#010409",
        fg_color = "#C6CDD5",
        intensity = "Half"
      },
      
      inactive_tab = {bg_color = "#010409", fg_color = "#3B4B58"},
      inactive_tab_hover = {bg_color = "#010409", fg_color = "#3B4B58"}
      }
    },

    -- [ Keybindings ]
      disable_default_key_bindings = true,
      keys = {
            {
              mods = "CTRL",
              key = [[o]],
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
