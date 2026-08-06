//! The gems, shown. Where each one is comes from the rules, and this only puts
//! the crystal there: on the floor where it was dropped, on the plinth it was
//! seated in, or in the hands of whoever picked it up.

use crate::ecs::{
    GEM_VISUAL, GemVisual, SOCKET_VISUAL, SocketVisual, SokobanResources, TileMotion,
};
use crate::rules::{GemSpot, seated_at};
use crate::systems::world::build::{GEM_CARRIED_Y, GEM_SEATED_Y, GEM_Y, world_position};
use crate::systems::world::motion::start_motion;
use nightshade::prelude::*;

/// How fast a gem turns where it lies, which is slow enough to read as
/// something resting rather than something running.
const SPIN_RATE: f32 = 1.1;
const BOB_RATE: f32 = 2.0;
const BOB_HEIGHT: f32 = 0.05;
const SOCKET_RATE: f32 = 8.0;

/// Where the crystal for this gem belongs, given where the rules say the gem
/// is. A gem in somebody's hands has no square of its own, which is what the
/// missing answer means.
fn resting_place(game: &SokobanResources, index: usize) -> Option<Vec3> {
    match game.state.gems.get(index).copied()? {
        GemSpot::Loose(at) => Some(world_position(at, GEM_Y)),
        GemSpot::Seated(at) => Some(world_position(at, GEM_SEATED_Y)),
        GemSpot::Held(_) => None,
    }
}

/// Sends every gem that is lying somewhere to where it is lying now. Undo,
/// restart and every move that touches a gem land here, so the crystals follow
/// the state rather than being moved alongside it.
///
/// A gem in somebody's hands is not sent anywhere, because it has no square to
/// be sent to. Where it rides is worked out every frame from where its carrier
/// has got to, which is the only thing that can keep up with a body that is
/// still walking.
pub fn restore(game: &mut SokobanResources, world: &mut World, seconds_per_step: f32) {
    for index in 0..game.entities.gems.len() {
        let entity = game.entities.gems[index];
        if matches!(game.state.gems.get(index), Some(GemSpot::Held(_))) {
            // A journey of its own and a ride in somebody's hands are two
            // answers to where it is, and only one of them can be right.
            if let Some(motion) = world.get_mut::<TileMotion>(entity) {
                motion.active = false;
            }
            continue;
        }
        let Some(target) = resting_place(game, index) else {
            continue;
        };
        start_motion(world, entity, vec![target], seconds_per_step, 0.0, false);
    }
}

/// Turns the loose gems where they lie and lights the sockets that are holding
/// one. A gem in a socket stands still, because it is doing its job rather than
/// waiting to be picked up.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if game.map.gems.is_empty() {
        return;
    }
    let elapsed = game.elapsed;
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(GEM_VISUAL).collect();
    for entity in entities {
        let Some(gem) = world.get::<GemVisual>(entity).copied() else {
            continue;
        };
        // A gem being carried rides over the head of whoever is carrying it,
        // and it has to be put there after they have moved rather than before,
        // which is why it is read off their body every frame.
        if let Some(GemSpot::Held(member)) = game.state.gems.get(gem.index).copied() {
            let carried = game
                .entities
                .members
                .get(member)
                .copied()
                .and_then(|body| world.get::<LocalTransform>(body))
                .map(|transform| transform.translation);
            if let Some(over) = carried
                && let Some(transform) = world.get_mut::<LocalTransform>(entity)
            {
                transform.translation = over + Vec3::new(0.0, GEM_CARRIED_Y, 0.0);
                transform.rotation = nalgebra_glm::quat_angle_axis(
                    elapsed * SPIN_RATE + gem.phase,
                    &nalgebra_glm::Vec3::new(0.0, 1.0, 0.0),
                );
            }
            continue;
        }
        let seated = matches!(game.state.gems.get(gem.index), Some(GemSpot::Seated(_)));
        let turn = if seated {
            elapsed * SPIN_RATE * 0.4 + gem.phase
        } else {
            elapsed * SPIN_RATE + gem.phase
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.rotation =
                nalgebra_glm::quat_angle_axis(turn, &nalgebra_glm::Vec3::new(0.0, 1.0, 0.0));
        }
        // A gem bobs only where it lies. One being carried is held, and one in
        // a plinth is seated in it, and neither of those floats.
        if seated || !matches!(game.state.gems.get(gem.index), Some(GemSpot::Loose(_))) {
            continue;
        }
        let Some(rest) = resting_place(game, gem.index) else {
            continue;
        };
        if crate::systems::world::motion::is_moving(world, entity) {
            continue;
        }
        let rise = ((elapsed * BOB_RATE) + gem.phase).sin() * BOB_HEIGHT;
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = rest + Vec3::new(0.0, rise, 0.0);
        }
    }

    let delta = world.res::<Time>().delta_time;
    let sockets: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(SOCKET_VISUAL)
        .collect();
    for entity in sockets {
        let (base, lit) = {
            let Some(socket) = world.get_mut::<SocketVisual>(entity) else {
                continue;
            };
            let holding = seated_at(&game.state, socket.at).is_some();
            let target = if holding { 1.0 } else { 0.0 };
            socket.lit += (target - socket.lit) * (1.0 - (-delta * SOCKET_RATE).exp());
            (socket.base, socket.lit)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y + lit * 0.06, base.z);
            let width = 0.38 - lit * 0.04;
            transform.scale = Vec3::new(width, 0.36 + lit * 0.06, width);
        }
    }
}
