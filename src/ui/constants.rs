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
// SIZES
//
pub const APP_ROUNDING: gpui::Pixels = px(6.0);
