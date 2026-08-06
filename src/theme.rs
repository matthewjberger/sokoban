//! The interface palette: the colours and sizes every screen draws itself
//! with. World colours live in [`crate::palette`], which the map's skin picks.

use nightshade::prelude::*;

pub const TRANSPARENT: Vec4 = Vec4::new(0.0, 0.0, 0.0, 0.0);
pub const WHITE: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0);

pub const TEXT_COLOR: Vec4 = Vec4::new(0.97, 0.93, 0.87, 1.0);
pub const TEXT_DIM: Vec4 = Vec4::new(0.84, 0.74, 0.66, 1.0);
pub const TEXT_FAINT: Vec4 = Vec4::new(0.62, 0.53, 0.48, 1.0);

pub const ACCENT: Vec4 = Vec4::new(1.0, 0.72, 0.32, 1.0);
pub const ACCENT_DIM: Vec4 = Vec4::new(0.32, 0.19, 0.08, 1.0);
pub const ACCENT_HOT: Vec4 = Vec4::new(1.0, 0.87, 0.6, 1.0);
pub const SUCCESS: Vec4 = Vec4::new(0.48, 0.92, 0.6, 1.0);

pub const PANEL_BG: Vec4 = Vec4::new(0.08, 0.06, 0.08, 0.42);
pub const PANEL_BG_DEEP: Vec4 = Vec4::new(0.06, 0.05, 0.07, 0.82);
pub const PANEL_BORDER: Vec4 = Vec4::new(1.0, 0.72, 0.32, 0.7);
pub const PANEL_HOVER: Vec4 = Vec4::new(0.2, 0.14, 0.1, 0.7);
pub const PANEL_PRESSED: Vec4 = Vec4::new(0.05, 0.04, 0.05, 0.85);

pub const BACKDROP: Vec4 = Vec4::new(0.03, 0.02, 0.04, 0.62);
pub const OUTLINE: Vec4 = Vec4::new(0.0, 0.0, 0.0, 0.85);

pub const MENU_BUTTON_HEIGHT: f32 = 46.0;
pub const MENU_BUTTON_SIZE: Vec2 = Vec2::new(340.0, MENU_BUTTON_HEIGHT);
