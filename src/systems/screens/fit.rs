//! Fitting the panels to the window. Every panel in this game is laid out in
//! absolute pixels, which is what makes the screens hold their proportions
//! instead of smearing across a wide monitor. The cost of that is a window
//! smaller than the design size, where fixed panels have nowhere to go and end
//! up on top of each other.
//!
//! The retained UI multiplies every absolute unit by the root's scale, so the
//! whole problem is one number. Work out how much of the design size the window
//! can actually show, and hand that to the root. Nothing else has to know.

use crate::ecs::SokobanResources;
use nightshade::prelude::*;

/// The window the screens were laid out against, in logical pixels. The widest
/// fixed panel is a thousand of them, and the tallest screen is the gallery with
/// its rail, so this is those plus the margins they sit in.
const DESIGN: Vec2 = Vec2::new(1120.0, 720.0);

/// How far the panels will shrink before they stop. Past this the text is not
/// worth reading, and a window that small is better off clipping than turning
/// the whole interface into a smear.
const FLOOR: f32 = 0.45;

/// Scales the panels to whatever the window can show. Runs every frame because a
/// window is resized by dragging it, which produces a new size per frame, and
/// writing the same number again costs a comparison.
pub fn update(game: Res<SokobanResources>, world: &mut World) {
    let window = world.res::<Window>();
    let Some((width, height)) = window.cached_viewport_size else {
        return;
    };
    // The layout multiplies this by the display scale on its own, so the sum
    // here is in logical pixels or the two would compound.
    let density = window.cached_scale_factor.max(0.1);
    let logical = Vec2::new(width as f32 / density, height as f32 / density);

    // Never above one. A big window should show more of the board rather than a
    // bigger menu, which is the whole reason these are absolute units.
    let fit = (logical.x / DESIGN.x)
        .min(logical.y / DESIGN.y)
        .clamp(FLOOR, 1.0);

    let root = game.ui.root;
    if !world.is_alive(root) {
        return;
    }
    if let Some(layout) = world.get_mut::<UiLayoutRoot>(root)
        && (layout.absolute_scale - fit).abs() > f32::EPSILON
    {
        layout.absolute_scale = fit;
    }
}
