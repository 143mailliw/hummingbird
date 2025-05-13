# Theming
Muzak can be themed with a `theme.json` file located in the following places:

| Platform | Location                                                       |
|----------|----------------------------------------------------------------|
| Linux    | `~/.local/share/muzak/theme.json`                              |
| macOS    | `~/Library/Application Support/me.william341.muzak/theme.json` |
| Windows  | `%appdata%\william341\muzak\data\theme.json`                   |

When this file is created, deleted, or modified, the theme is reloaded. If your
theme produces the default theme with no modified properties, it is likely that
your theme failed to parse - running with `RUST_LOG=muzak=info` may give you
more information.

Colors are specified as CSS-style hex codes (`#ABCDEF`). If a color is not
specified, the color from the default theme is used.

## Example
A `theme.json` for the default theme is provided here. Note the colors may be
out of date, an effort is made to ensure all possible fields are represented
in this example.

```json
{
  "background_primary": "#0C1116",
  "background_secondary": "#161A22",
  "background_tertiary": "#222831",

  "border_color": "#272D37",

  "album_art_background": "#4C5974",
  "text": "#F4F5F6",
  "text_secondary": "#BEC4CA",

  "nav_button_hover": "#161A22",
  "nav_button_active": "#0A0E12",

  "playback_button": "#282F3D00",
  "playback_button_hover": "#282F3D",
  "playback_button_active": "#0D1014",
  "playback_button_border": "#37404E00",

  "window_button": "#282F3D00",
  "window_button_hover": "#282F3D",
  "window_button_active": "#0D1014",

  "queue_item": "#161A2200",
  "queue_item_hover": "#161A22",
  "queue_item_active": "#0C1116",
  "queue_item_current": "#272D37",

  "close_button": "#282F3D00",
  "close_button_hover": "#AE0909",
  "close_button_active": "#7A0606",

  "button_primary": "#0667B2",
  "button_primary_hover": "#087AD1",
  "button_primary_active": "#065D9F",
  "button_primary_text": "#E0F1FE",

  "button_secondary": "#37404E",
  "button_secondary_hover": "#495467",
  "button_secondary_active": "#262C36",
  "button_secondary_text": "#BEC4CA",

  "button_warning": "#EDB407",
  "button_warning_hover": "#F8C017",
  "button_warning_active": "#D6A207",
  "button_warning_text": "#FEF8E5",

  "button_danger": "#CD0B0B",
  "button_danger_hover": "#E80C0C",
  "button_danger_active": "#B70A0A",
  "button_danger_text": "#FEE3E3",

  "slider_foreground": "#0673C6",
  "slider_background": "#37404E",

  "elevated_background": "#161A22",
  "elevated_border_color": "#272D37",

  "menu_item": "#282F3D00",
  "menu_item_hover": "#282F3D",
  "menu_item_active": "#0D1014",

  "modal_overlay_bg": "#0C111655",

  "text_input_selection": "#0673C688",
  "caret_color": "#F4F5F6",

  "palette_item_hover": "#282F3D",
  "palette_item_active": "#0D1014"
}
```
