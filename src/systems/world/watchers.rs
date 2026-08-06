//! The watchers, shown. What makes one dangerous is not the square it stands
//! on but the four around it, and none of that is visible in a post with an eye
//! on it, so the reach is painted on the floor: the squares it holds are the
//! squares you may not be standing on when the move ends.

use crate::ecs::{LayerTag, SokobanResources};
use crate::palette::WATCHER_REACH;
use crate::schema::{Direction, Position, map_tile};
use crate::systems::world::build::{block, world_position};
use crate::systems::world::motion::start_motion;
use nightshade::prelude::*;

/// How high the marks on the floor lie. Above everything laid flat on a square
/// and below everything that stands up on one, which is where the pools of
/// light sit for the same reason.
const REACH_Y: f32 = 0.15;
const REACH_THICKNESS: f32 = 0.02;
const REACH_EXTENT: f32 = 0.86;
const REACH_GLOW: f32 = 1.1;

/// Puts every watcher where the state says it is. A trade is the only thing
/// that moves one, so this only ever has anything to do on a board with a
/// swapper on it, and it lands here with undo and restart like everything else.
pub fn restore(game: &mut SokobanResources, world: &mut World, seconds_per_step: f32) {
    for index in 0..game.entities.watchers.len() {
        let entity = game.entities.watchers[index];
        let Some(at) = game.state.watchers.get(index).copied() else {
            continue;
        };
        start_motion(
            world,
            entity,
            vec![world_position(at, crate::systems::world::build::WATCHER_Y)],
            seconds_per_step,
            0.0,
            false,
        );
    }
}

/// Marks every square a watcher can reach, and takes the marks up again when
/// one of them has been moved. Which squares those are is the rule itself
/// rather than a decoration on it, so the board is drawn from the same answer
/// the rules kill by.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if game.state.watchers.is_empty() && game.reach_shape.is_empty() {
        return;
    }

    let mut wanted: Vec<Position> = Vec::new();
    for watcher in &game.state.watchers {
        for direction in Direction::ALL {
            let at = watcher.offset(direction.delta());
            // A square nothing could ever stand on is a square nothing needs
            // warning about.
            if map_tile(&game.map, at).blocks_forever() {
                continue;
            }
            wanted.push(at);
        }
    }
    wanted.sort_unstable_by_key(|at| (at.layer, at.cell.1, at.cell.0));
    wanted.dedup();
    if wanted == game.reach_shape {
        return;
    }

    for entity in std::mem::take(&mut game.reach) {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    for at in &wanted {
        let mark = block(
            world,
            "Cube",
            world_position(*at, REACH_Y),
            Vec3::new(REACH_EXTENT, REACH_THICKNESS, REACH_EXTENT),
            "sokoban_watcher_reach",
            Material {
                base_color: [
                    WATCHER_REACH[0] * 0.5,
                    WATCHER_REACH[1] * 0.5,
                    WATCHER_REACH[2] * 0.5,
                    1.0,
                ],
                emissive_factor: [
                    WATCHER_REACH[0] * REACH_GLOW,
                    WATCHER_REACH[1] * REACH_GLOW,
                    WATCHER_REACH[2] * REACH_GLOW,
                ],
                roughness: 0.7,
                ..Default::default()
            },
        );
        world.set(mark, LayerTag { layer: at.layer });
        game.reach.push(mark);
    }
    game.reach_shape = wanted;
}
