use crate::ecs::{Facing, MapOrigin, Screen, SokobanResources};
use crate::maps::map_count;
use crate::palette::palette_for;
use crate::rules::{covered, map_solved};
use crate::systems::world::build::{CRATE_Y, PLAYER_Y, world_position};
use crate::systems::world::motion::{is_busy, start_motion};
use nightshade::prelude::*;

const SOLVE_DELAY: f32 = 0.4;

pub fn check_solved(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    // A gallery board is a demonstration, not a puzzle. Finishing one is not an
    // ending, and latching it would stop the player carrying on with the board
    // they were sent there to play with.
    if matches!(game.origin, MapOrigin::Lesson) {
        return;
    }
    if game.solved_announced || !map_solved(&game.map, &game.state) || is_busy(world) {
        return;
    }
    game.solved_delay += world.res::<Time>().delta_time;
    if game.solved_delay < SOLVE_DELAY {
        return;
    }

    game.solved_announced = true;

    let index = match game.origin {
        MapOrigin::Authored => {
            game.editor.status = format!("test run solved in {} moves", game.state.moves);
            next_state(world, Screen::Editor);
            return;
        }
        MapOrigin::Lesson => return,
        MapOrigin::Random => {
            game.total_moves += game.state.moves;
            next_state(world, Screen::MapComplete);
            return;
        }
        MapOrigin::Story(level) => {
            game.total_moves += game.state.moves;
            crate::systems::world::story::finish_puzzle(game, world, level);
            return;
        }
        MapOrigin::Overworld => return,
        MapOrigin::Endless => {
            game.total_moves += game.state.moves;
            crate::systems::world::work::make(game, crate::ecs::Making::RunNext);
            return;
        }
        MapOrigin::Campaign(index) => index,
    };

    game.total_moves += game.state.moves;
    let next = index + 1;
    if next >= map_count() {
        next_state(world, Screen::CampaignComplete);
    } else {
        game.selected_map = next;
        next_state(world, Screen::MapComplete);
    }
}

/// Puts every actor where the state says it is. Undo and restart both land
/// here, so the world follows the state rather than the other way round.
pub fn restore_entities(game: &mut SokobanResources, world: &mut World, seconds_per_step: f32) {
    // Every member, not only the one being played. Undo and restart put the
    // whole party back, and a member left behind would be a member standing
    // somewhere the rules say nobody is.
    for index in 0..game.state.members.len() {
        let Some(body) = game.entities.members.get(index).copied() else {
            continue;
        };
        let target = world_position(game.state.members[index], PLAYER_Y);
        start_motion(world, body, vec![target], seconds_per_step, 0.0, false);
    }
    let yaw = game.state.facing.yaw();
    if let Some(facing) = world.get_mut::<Facing>(game.active_body()) {
        facing.target = yaw;
    }

    for index in 0..game.entities.crates.len() {
        let entity = game.entities.crates[index];
        let state = game.state.crates[index];
        let target = world_position(state.at, CRATE_Y);
        start_motion(
            world,
            entity,
            vec![target],
            seconds_per_step,
            0.0,
            state.sunk,
        );
    }

    crate::systems::world::gems::restore(game, world, seconds_per_step);
    crate::systems::world::watchers::restore(game, world, seconds_per_step);
    refresh_materials(game, world);
}

/// Drops every marker's light to nothing while a death plays out, and puts it
/// back when the board does.
pub fn fade_goals(game: &SokobanResources, world: &mut World, out: bool) {
    let palette = palette_for(game.map.skin);
    for index in 0..game.entities.goal_markers.len() {
        let name = format!("sokoban_goal_{index}");
        let lit = game
            .entities
            .goal_covered
            .get(index)
            .copied()
            .unwrap_or(false);
        mutate_material(world, &name, |material| {
            material.emissive_factor = if out {
                [0.0, 0.0, 0.0]
            } else if lit {
                [0.18, 0.7, 0.28]
            } else {
                [0.1, 0.42, 0.38]
            };
            material.base_color = if out {
                [0.16, 0.16, 0.18, 1.0]
            } else if lit {
                palette.goal_done
            } else {
                palette.goal
            };
        });
    }
}

pub fn refresh_materials(game: &mut SokobanResources, world: &mut World) {
    let palette = palette_for(game.map.skin);
    for index in 0..game.entities.crates.len() {
        let state = game.state.crates[index];
        let on_goal = !state.sunk && game.map.goals.contains(&state.at);
        if game.entities.crate_covered[index] == on_goal {
            continue;
        }
        game.entities.crate_covered[index] = on_goal;
        let name = format!("sokoban_crate_{index}");
        mutate_material(world, &name, |material| {
            material.base_color = if on_goal {
                palette.crate_done
            } else {
                palette.crate_body
            };
            material.emissive_factor = if on_goal {
                [0.05, 0.28, 0.08]
            } else {
                [0.0, 0.0, 0.0]
            };
        });
    }

    for index in 0..game.entities.goal_markers.len() {
        let Some(at) = game.map.goals.get(index).copied() else {
            continue;
        };
        let filled = covered(&game.state, at);
        if game.entities.goal_covered[index] == filled {
            continue;
        }
        game.entities.goal_covered[index] = filled;
        let name = format!("sokoban_goal_{index}");
        mutate_material(world, &name, |material| {
            material.base_color = if filled {
                palette.goal_done
            } else {
                palette.goal
            };
            material.emissive_factor = if filled {
                [0.18, 0.7, 0.28]
            } else {
                [0.1, 0.42, 0.38]
            };
        });
    }
}
