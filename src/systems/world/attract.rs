//! The front screen has no board on it. It is a menu against a sky: nothing is
//! built, nothing is solved, and nothing is playing, which is why the game is up
//! as soon as it is asked for.
//!
//! The sky is still a skin, so the screen behind the menu is the same weather
//! the boards are lit by rather than a flat colour.

use crate::ecs::{MapOrigin, SokobanResources};
use crate::palette::palette_for;
use crate::schema::Skin;
use crate::systems::world::build;
use nightshade::prelude::*;

/// Where the camera sits while the menu is up. Nothing is in front of it, so
/// this only decides which part of the sky is.
const LOOK_FROM: Vec3 = Vec3::new(0.0, 6.0, 14.0);
const LOOK_AT: Vec3 = Vec3::new(0.0, 2.0, 0.0);

/// Clears the board and puts up the sky. Called when the front screen is
/// entered, so whatever was being played is taken down with it.
pub fn start(game: &mut SokobanResources, world: &mut World) {
    build::clear(game, world);
    game.map = crate::schema::Map::default();
    game.state = crate::rules::initial_state(&game.map);
    // Where the board came from goes with the board. A run left behind must not
    // still be answering for what is on screen, or the dials that only a run
    // sets would keep their hold on a menu.
    game.origin = MapOrigin::default();
    game.camera.extent = Vec2::new(0.0, 0.0);

    build::apply_skin(&palette_for(Skin::Drift), world);

    let forward = (LOOK_AT - LOOK_FROM).normalize();
    let rotation = nalgebra_glm::quat_inverse(&nalgebra_glm::quat_look_at_rh(
        &forward,
        &Vec3::new(0.0, 1.0, 0.0),
    ));
    let camera = game.camera.entity;
    if let Some(transform) = world.get_mut::<LocalTransform>(camera) {
        transform.translation = LOOK_FROM;
        transform.rotation = rotation;
    }
}
