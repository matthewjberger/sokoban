use crate::ecs::{LAYER_TAG, LayerTag, SokobanResources};
use crate::rules::GemSpot;
use crate::schema::{Position, Tile, map_elevator_target, map_layers, map_positions, map_tile};
use crate::systems::world::build::{block, focus_layer, world_position};
use crate::systems::world::motion::is_moving;
use nightshade::prelude::*;

/// How the mark under something standing on the storey above is drawn. It lies
/// flat on the floor below and reads as a shadow of what is over it.
const FOOTPRINT_Y: f32 = 0.14;
const FOOTPRINT_THICKNESS: f32 = 0.02;
const FOOTPRINT_EXTENT: f32 = 0.7;
const FOOTPRINT_COLOR: [f32; 4] = [0.1, 0.11, 0.14, 1.0];

/// How high the sign over a shaft floats, and how far it leans the way it goes.
const SIGN_Y: f32 = 1.0;
const SIGN_LEAN: f32 = 0.22;
const SIGN_CLEAR: [f32; 4] = [0.45, 0.85, 0.95, 1.0];
const SIGN_TAKEN: [f32; 4] = [0.95, 0.5, 0.25, 1.0];

/// Shows the storey in play and hides the rest. Every square is built once, so
/// changing floors is a visibility flip and a camera move rather than a
/// teardown, and a crate that rides a shaft simply belongs to a different
/// storey from one move to the next.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;

    // An actor belongs to the storey it has arrived at, not the one it is
    // headed for, so a crate riding a shaft stays on screen for the whole ride
    // and only then joins the storey below.
    for index in 0..game.entities.crates.len() {
        let Some(state) = game.state.crates.get(index) else {
            continue;
        };
        let body = game.entities.crates[index];
        if is_moving(world, body) {
            continue;
        }
        retag(world, body, state.at.layer);
        for part in game.entities.crate_parts[index] {
            retag(world, part, state.at.layer);
        }
    }

    // A gem belongs to the storey it is lying on, or to the storey of whoever
    // is carrying it, since a gem in a lift goes up with the hands holding it.
    for index in 0..game.entities.gems.len() {
        let body = game.entities.gems[index];
        if is_moving(world, body) {
            continue;
        }
        let layer = match game.state.gems.get(index).copied() {
            Some(GemSpot::Held(member)) => game.state.members.get(member).map(|at| at.layer),
            Some(spot) => spot.square().map(|at| at.layer),
            None => None,
        };
        if let Some(layer) = layer {
            retag(world, body, layer);
        }
    }

    let player = game.active_body();
    if !is_moving(world, player) {
        let layer = game.state.player().layer;
        retag(world, player, layer);
        for part in game.entities.player_parts.clone() {
            retag(world, part, layer);
        }
        if game.entities.layer != layer {
            focus_layer(game, layer);
        }
    }

    // The storey under the one in play is shown rather than hidden, because a
    // stack of floors nobody can see the rest of is a stack nobody can hold in
    // their head. It sits a storey lower and reads as underneath from that
    // alone. A board with one floor has nothing under it and pays none of this.
    let active = game.entities.layer;
    let stacked = map_layers(&game.map).len() > 1;
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(LAYER_TAG).collect();
    for entity in entities {
        let Some(tag) = world.get::<LayerTag>(entity).copied() else {
            continue;
        };
        let visible = tag.layer == active || (stacked && tag.layer == active - 1);
        if world
            .get::<Visibility>(entity)
            .is_some_and(|visibility| visibility.visible != visible)
        {
            world.set(entity, Visibility { visible });
        }
    }

    footprints(game, world, stacked);
    signposts(game, world, stacked);
}

/// Puts a sign over every shaft on the storey in play saying which way it goes
/// and whether anything is standing at the other end. A lift that reads as an
/// ordinary pad until you are on it is a lift nobody plans around.
///
/// Where a shaft goes is the schema's own answer, so what is drawn and what a
/// ride does cannot come apart.
fn signposts(game: &mut SokobanResources, world: &mut World, stacked: bool) {
    let active = game.entities.layer;
    let mut wanted: Vec<(Position, i32, bool)> = Vec::new();
    if stacked {
        for at in map_positions(&game.map)
            .into_iter()
            .filter(|at| at.layer == active && map_tile(&game.map, *at) == Tile::Elevator)
        {
            for way in [1, -1] {
                let Some(target) = map_elevator_target(&game.map, at, way) else {
                    continue;
                };
                let taken = game.state.members.contains(&target)
                    || game
                        .state
                        .crates
                        .iter()
                        .any(|entry| !entry.sunk && entry.at == target)
                    || game.state.watchers.contains(&target);
                wanted.push((at, way, taken));
            }
        }
    }
    wanted.sort_unstable_by_key(|(at, way, _)| (at.cell.1, at.cell.0, *way));
    if wanted == game.signpost_shape {
        return;
    }

    for entity in std::mem::take(&mut game.signposts) {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    for (at, way, taken) in &wanted {
        // The sign leans the way the shaft goes, and wears the colour of what
        // is waiting at the other end: clear, or somebody already standing on
        // the square this would put you on.
        let lift = SIGN_Y + *way as f32 * SIGN_LEAN;
        let colour = if *taken { SIGN_TAKEN } else { SIGN_CLEAR };
        let sign = block(
            world,
            "Cone",
            world_position(*at, lift),
            Vec3::new(0.3, 0.26 * *way as f32, 0.3),
            if *taken {
                "sokoban_signpost_taken"
            } else {
                "sokoban_signpost"
            },
            Material {
                base_color: colour,
                emissive_factor: [colour[0] * 0.6, colour[1] * 0.6, colour[2] * 0.6],
                roughness: 0.4,
                ..Default::default()
            },
        );
        world.set(sign, LayerTag { layer: at.layer });
        game.signposts.push(sign);
    }
    game.signpost_shape = wanted;
}

/// Drops a mark on the storey below under everything standing on the storey in
/// play, so a crate on the floor above reads as something that could come down
/// rather than as something on another board entirely.
///
/// The marks are laid and taken up again as the board moves under them, so this
/// pass owns them: they are never handed to the map's teardown list.
fn footprints(game: &mut SokobanResources, world: &mut World, stacked: bool) {
    let active = game.entities.layer;
    let mut wanted: Vec<Position> = Vec::new();
    if stacked {
        let below = active - 1;
        for at in game
            .state
            .members
            .iter()
            .copied()
            .chain(
                game.state
                    .crates
                    .iter()
                    .filter(|entry| !entry.sunk)
                    .map(|entry| entry.at),
            )
            .filter(|at| at.layer == active)
        {
            let under = Position::new(below, at.cell);
            // A mark under a square that is not there is a mark floating in the
            // air, so only ground gets one.
            if map_tile(&game.map, under) != Tile::Void {
                wanted.push(under);
            }
        }
    }
    wanted.sort_unstable_by_key(|at| (at.cell.1, at.cell.0));
    wanted.dedup();
    if wanted == game.footprint_shape {
        return;
    }

    for entity in std::mem::take(&mut game.footprints) {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    for at in &wanted {
        let mark = block(
            world,
            "Cube",
            world_position(*at, FOOTPRINT_Y),
            Vec3::new(FOOTPRINT_EXTENT, FOOTPRINT_THICKNESS, FOOTPRINT_EXTENT),
            "sokoban_footprint",
            Material {
                base_color: FOOTPRINT_COLOR,
                emissive_factor: [0.06, 0.07, 0.09],
                roughness: 0.9,
                ..Default::default()
            },
        );
        world.set(mark, LayerTag { layer: at.layer });
        game.footprints.push(mark);
    }
    game.footprint_shape = wanted;
}

fn retag(world: &mut World, entity: Entity, layer: i32) {
    if entity == Entity::default() {
        return;
    }
    if world
        .get::<LayerTag>(entity)
        .is_some_and(|tag| tag.layer == layer)
    {
        return;
    }
    world.set(entity, LayerTag { layer });
}
