use gpui::px;

//
// FONTS
//

#[cfg(target_os = "windows")]
pub const FONT_AWESOME: &str = "Font Awesome 6 Free Solid";
#[cfg(not(target_os = "windows"))]
pub const FONT_AWESOME: &str = "Font Awesome 6 Free";

#[cfg(target_os = "windows")]
pub const FONT_AWESOME_BRANDS: &str = "Font Awesome 6 Brands";
#[cfg(not(target_os = "windows"))]
pub const FONT_AWESOME_BRANDS: &str = "Font Awesome 6 Brands";

//
// ICONS
//

/// icon: `bars`
/// https://fontawesome.com/icons/bars?f=classic&s=solid
pub const ICON_BARS: &str = "\u{f0c9}";
/// icon: `volume-xmark`
/// https://fontawesome.com/icons/volume-xmark?f=classic&s=solid
pub const ICON_VOLUME_XMARK: &str = "\u{f6a9}";
/// icon: `volume-high`
/// https://fontawesome.com/icons/volume-high?f=classic&s=solid
pub const ICON_VOLUME_HIGH: &str = "\u{f028}";
/// icon: `repeat`
/// https://fontawesome.com/icons/repeat?f=classic&s=solid
pub const ICON_REPEAT: &str = "\u{f363}";
/// icon: `forward-step`
/// https://fontawesome.com/icons/forward-step?f=classic&s=solid
pub const ICON_FORWARD_STEP: &str = "\u{f051}";
/// icon: `pause`
/// https://fontawesome.com/icons/pause?f=classic&s=solid
pub const ICON_PAUSE: &str = "\u{f04c}";
/// icon: `play`
/// https://fontawesome.com/icons/play?f=classic&s=solid
pub const ICON_PLAY: &str = "\u{f04b}";
/// icon: `backward-step`
/// https://fontawesome.com/icons/backward-step?f=classic&s=solid
pub const ICON_BACKWARD_STEP: &str = "\u{f048}";
/// icon: `shuffle`
/// https://fontawesome.com/icons/shuffle?f=classic&s=solid
pub const ICON_SHUFFLE: &str = "\u{f074}";

/// icon: `arrow-left`
/// https://fontawesome.com/icons/arrow-left?f=classic&s=solid
pub const ICON_ARROW_LEFT: &str = "\u{f060}";

/// icon: `xmark`
/// https://fontawesome.com/icons/xmark?f=classic&s=solid
pub const ICON_XMARK: &str = "\u{f00d}";
/// icon: `minus`
/// https://fontawesome.com/icons/minus?f=classic&s=solid
pub const ICON_MINUS: &str = "\u{f068}";
/// icon: `expand`
/// https://fontawesome.com/icons/expand?f=classic&s=solid
pub const ICON_EXPAND: &str = "\u{f065}";
/// icon: `magnifying-glass`
/// https://fontawesome.com/icons/magnifying-glass?f=classic&s=solid
pub const ICON_MAGNIFYING_GLASS: &str = "\u{f002}";
/// icon: `check`
/// https://fontawesome.com/icons/check?f=classic&s=solid
pub const ICON_CHECK: &str = "\u{f00c}";
/// icon: `lastfm`
/// https://fontawesome.com/icons/lastfm?f=classic&s=brands
pub const ICON_LASTFM: &str = "\u{f202}";

/// icon: `trash-can`
/// https://fontawesome.com/icons/trash-can?f=classic&s=solid
pub const ICON_TRASH_CAN: &str = "\u{f2ed}";

//
// SIZES
//
pub const APP_ROUNDING: gpui::Pixels = px(6.0);
