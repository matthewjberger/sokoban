use crate::ecs::{
    BRITTLE_VISUAL, BrittleVisual, FRAGILE_VISUAL, FragileVisual, GATE_VISUAL, GOAL_MARKER,
    GateVisual, GoalMarker, PLATE_VISUAL, PlateVisual, SENSOR_VISUAL, SPIKE_VISUAL, SPINNER,
    SWITCH_VISUAL, SensorVisual, SokobanResources, SpikeVisual, Spinner, SwitchVisual,
};
use crate::palette::gem_body;
use crate::rules::{beam_field, covered, elevator_options, gate_flags, light_lends, pressed};
use crate::schema::{CrateKind, Tile, map_tile};
use crate::systems::world::build::{
    CRATE_SUNK_Y, GATE_CLOSED_Y, GATE_OPEN_Y, PROMPT_Y, world_position,
};
use crate::systems::world::motion::is_moving;
use nightshade::prelude::*;

const GATE_RATE: f32 = 5.0;
const PLATE_RATE: f32 = 9.0;
const GLOW_RATE: f32 = 7.0;
const FALL_RATE: f32 = 7.5;
const THROW_RATE: f32 = 11.0;
const FALL_DEPTH: f32 = 1.4;
const SPIKE_RATE: f32 = 12.0;
const BREAK_RATE: f32 = 9.0;
/// How bright a body wears the colour it is standing in, and how hard it
/// flashes while the board is killing it.
const BODY_GLOW: f32 = 1.6;
const HURT_GLOW: f32 = 3.0;
const HURT_FLASHES: f32 = 26.0;
/// What a whole boulder measures, which is what a broken one is worked back
/// from and what undo puts it back to.
const STONE_SCALE: Vec3 = Vec3::new(0.46, 0.37, 0.45);
/// How far a spike travels between lying in the floor and standing in the way,
/// which is far enough to be unmistakable from overhead.
const SPIKE_RISE: f32 = 0.52;

pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    let delta = world.res::<Time>().delta_time;
    game.elapsed += delta;

    animate_spinners(game.elapsed, world);
    animate_gates(game, world, delta);
    animate_plates(game, world, delta);
    animate_goals(game, world, delta);
    animate_fragile(game, world, delta);
    animate_brittle(game, world, delta);
    animate_switches(game, world, delta);
    animate_sensors(game, world, delta);
    animate_spikes(game, world, delta);
    break_stones(game, world, delta);
    light_bodies(game, world);
    bob_floaters(game, world);
    outline_active(game, world);
    update_prompt(game, world);
}

/// The lid over a collapsed square falls out of sight, which leaves the hole
/// that was built under it doing the talking.
fn animate_fragile(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(FRAGILE_VISUAL)
        .collect();
    for entity in entities {
        let (base, fall) = {
            let Some(slab) = world.get_mut::<FragileVisual>(entity) else {
                continue;
            };
            let gone = game.state.collapsed.contains(&slab.at);
            let target = if gone { 1.0 } else { 0.0 };
            slab.fall += (target - slab.fall) * (1.0 - (-delta * FALL_RATE).exp());
            (slab.base, slab.fall)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y - fall * FALL_DEPTH, base.z);
            let width = 0.9 * (1.0 - fall * 0.35);
            transform.scale = Vec3::new(width, 0.44, width);
        }
    }
}

/// A broken wall sinks into the plinth rather than blinking out, so the square
/// opening up reads as something that happened.
fn animate_brittle(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(BRITTLE_VISUAL)
        .collect();
    for entity in entities {
        let (base, fall) = {
            let Some(wall) = world.get_mut::<BrittleVisual>(entity) else {
                continue;
            };
            let gone = game.state.broken.contains(&wall.at);
            let target = if gone { 1.0 } else { 0.0 };
            wall.fall += (target - wall.fall) * (1.0 - (-delta * FALL_RATE).exp());
            (wall.base, wall.fall)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y - fall * FALL_DEPTH, base.z);
            let width = 0.9 * (1.0 - fall * 0.5);
            transform.scale = Vec3::new(width, 0.92, width);
        }
    }
}

/// Opens the eye of every sensor a lamp is reaching. The set of lit squares is
/// the rules' own, so the pad agrees with the gate it is holding rather than
/// with how bright the floor happens to look.
fn animate_sensors(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(SENSOR_VISUAL)
        .collect();
    if entities.is_empty() {
        return;
    }
    let lit = crate::rules::lit_squares(&game.map, &game.state);
    for entity in entities {
        let (base, glow) = {
            let Some(sensor) = world.get_mut::<SensorVisual>(entity) else {
                continue;
            };
            let target = if lit.contains(&sensor.at) { 1.0 } else { 0.0 };
            sensor.lit += (target - sensor.lit) * (1.0 - (-delta * GLOW_RATE).exp());
            (sensor.base, sensor.lit)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y + glow * 0.08, base.z);
            let width = 0.34 + glow * 0.08;
            transform.scale = Vec3::new(width, width, width);
        }
    }
}

/// Stands the spikes up while their group is powered and drops them back into
/// the floor when it is not. What powers a group is the same answer a gate is
/// given, so a board can raise a bed of spikes with anything it opens a door
/// with.
fn animate_spikes(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(SPIKE_VISUAL)
        .collect();
    if entities.is_empty() {
        return;
    }
    let flags = gate_flags(&game.map, &game.state);
    for entity in entities {
        let (base, raised) = {
            let Some(bed) = world.get_mut::<SpikeVisual>(entity) else {
                continue;
            };
            let up = flags.get(bed.group).copied().unwrap_or(false);
            let target = if up { 1.0 } else { 0.0 };
            bed.raised += (target - bed.raised) * (1.0 - (-delta * SPIKE_RATE).exp());
            (bed.base, bed.raised)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y + raised * SPIKE_RISE, base.z);
        }
    }
}

/// Takes a broken boulder to pieces. Nothing swallowed it, so it does not sink:
/// it comes apart where it stood and the square is clear. Undo puts it back
/// together the same way, because the target is read off the board rather than
/// remembered.
fn break_stones(game: &SokobanResources, world: &mut World, delta: f32) {
    for index in 0..game.state.crates.len() {
        let entry = game.state.crates[index];
        if entry.kind != CrateKind::Stone {
            continue;
        }
        let Some(body) = game.entities.crates.get(index).copied() else {
            continue;
        };
        let target = if entry.sunk { 0.0 } else { 1.0 };
        let Some(transform) = world.get_mut::<LocalTransform>(body) else {
            continue;
        };
        let whole = transform.scale.x / STONE_SCALE.x;
        // A boulder that is already whole, or already gone, is left alone. A
        // transform written every frame is a transform the renderer syncs every
        // frame, and nothing on a board at rest should be doing that.
        if (whole - target).abs() < 0.001 {
            continue;
        }
        let size = whole + (target - whole) * (1.0 - (-delta * BREAK_RATE).exp());
        transform.scale = STONE_SCALE * size.clamp(0.0, 1.0);
    }
}

/// What each member's body is giving off: the colour of the light they are
/// standing in, or a red flash while the board is killing them. One pass owns
/// it, because two of them would fight over the same material.
fn light_bodies(game: &mut SokobanResources, world: &mut World) {
    let field = light_lends(&game.map).then(|| beam_field(&game.map, &game.state));
    if field.is_none()
        && game.dying <= 0.0
        && game
            .entities
            .member_glow
            .iter()
            .all(|glow| *glow == [0.0; 3])
    {
        return;
    }
    for index in 0..game.entities.members.len() {
        let mut emissive = [0.0f32; 3];
        if let Some(field) = field.as_ref()
            && let Some(at) = game.state.members.get(index)
        {
            // Two colours crossing one square lend both, so the body wears the
            // brighter of each channel rather than whichever was traced last.
            for (_, color) in field.aura.iter().filter(|(square, _)| square == at) {
                let body = gem_body(*color);
                for channel in 0..3 {
                    emissive[channel] = emissive[channel].max(body[channel] * BODY_GLOW);
                }
            }
        }
        if game.dying > 0.0 && index == game.state.active {
            let flash = (game.dying * HURT_FLASHES).sin().abs();
            emissive = [HURT_GLOW * flash, 0.02, 0.02];
        }
        let Some(held) = game.entities.member_glow.get_mut(index) else {
            continue;
        };
        if *held == emissive {
            continue;
        }
        *held = emissive;
        let name = format!("sokoban_body_{}", game.map.member_character(index).label());
        mutate_material(world, &name, |material| {
            material.emissive_factor = emissive;
        });
    }
}

fn animate_switches(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(SWITCH_VISUAL)
        .collect();
    for entity in entities {
        let (base, thrown) = {
            let Some(lever) = world.get_mut::<SwitchVisual>(entity) else {
                continue;
            };
            let latched = game
                .state
                .latched
                .get(lever.group)
                .copied()
                .unwrap_or(false);
            let target = if latched { 1.0 } else { 0.0 };
            lever.thrown += (target - lever.thrown) * (1.0 - (-delta * THROW_RATE).exp());
            (lever.base, lever.thrown)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y - thrown * 0.1, base.z);
            transform.scale = Vec3::new(0.3, 0.32 - thrown * 0.14, 0.3);
            transform.rotation = nalgebra_glm::quat_angle_axis(
                thrown * std::f32::consts::FRAC_PI_4,
                &Vec3::new(1.0, 0.0, 0.0),
            );
        }
    }
}

fn animate_spinners(elapsed: f32, world: &mut World) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(SPINNER).collect();
    for entity in entities {
        let Some(spinner) = world.get::<Spinner>(entity).copied() else {
            continue;
        };
        let bob = ((elapsed * spinner.bob_speed) + spinner.phase).sin() * spinner.bob_height;
        let spin = nalgebra_glm::quat_angle_axis(
            elapsed * spinner.spin_speed + spinner.phase,
            &Vec3::new(0.0, 1.0, 0.0),
        ) * nalgebra_glm::quat_angle_axis(0.62, &Vec3::new(1.0, 0.0, 1.0));
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = spinner.base + Vec3::new(0.0, bob, 0.0);
            transform.rotation = spin;
        }
    }
}

fn animate_gates(game: &SokobanResources, world: &mut World, delta: f32) {
    let flags = gate_flags(&game.map, &game.state);
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(GATE_VISUAL).collect();
    for entity in entities {
        let (base, openness) = {
            let Some(gate) = world.get_mut::<GateVisual>(entity) else {
                continue;
            };
            let held = flags.get(gate.group).copied().unwrap_or(false);
            let occupied = game.state.player() == gate.at || covered(&game.state, gate.at);
            let target = if held || occupied { 1.0 } else { 0.0 };
            gate.openness += (target - gate.openness) * (1.0 - (-delta * GATE_RATE).exp());
            (gate.base, gate.openness)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(
                base.x,
                base.y + (GATE_OPEN_Y - GATE_CLOSED_Y) * openness,
                base.z,
            );
        }
    }
}

fn animate_plates(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(PLATE_VISUAL)
        .collect();
    for entity in entities {
        let (base, held) = {
            let Some(plate) = world.get_mut::<PlateVisual>(entity) else {
                continue;
            };
            let target = if pressed(&game.map, &game.state, plate.at) {
                1.0
            } else {
                0.0
            };
            plate.pressed += (target - plate.pressed) * (1.0 - (-delta * PLATE_RATE).exp());
            (plate.base, plate.pressed)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y - held * 0.035, base.z);
            let width = 0.86 - held * 0.05;
            transform.scale = Vec3::new(width, 0.1, width);
        }
    }
}

fn animate_goals(game: &SokobanResources, world: &mut World, delta: f32) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(GOAL_MARKER).collect();
    for entity in entities {
        let (base, glow) = {
            let Some(marker) = world.get_mut::<GoalMarker>(entity) else {
                continue;
            };
            let filled = game
                .map
                .goals
                .get(marker.index)
                .is_some_and(|at| covered(&game.state, *at));
            // A death puts the markers out, which is the room saying what has
            // happened without a word of interface.
            let target = if game.dying > 0.0 {
                0.0
            } else if filled {
                1.0
            } else {
                0.0
            };
            marker.glow += (target - marker.glow) * (1.0 - (-delta * GLOW_RATE).exp());
            (marker.base, marker.glow)
        };
        let pulse = (game.elapsed * 2.4).sin() * 0.04 * (1.0 - glow);
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y + glow * 0.012, base.z);
            let width = 0.98 + pulse + glow * 0.08;
            transform.scale = Vec3::new(width, 0.07, width);
        }
    }
}

/// A crate lost to water is floating rather than gone, so it rides the surface
/// the way a buoy does: up and down on the swell, and moored where it went in
/// rather than drifting about the pond.
fn bob_floaters(game: &SokobanResources, world: &mut World) {
    for index in 0..game.state.crates.len() {
        let entry = game.state.crates[index];
        if !entry.sunk || map_tile(&game.map, entry.at) != Tile::Water {
            continue;
        }
        let Some(body) = game.entities.crates.get(index).copied() else {
            continue;
        };
        if is_moving(world, body) {
            continue;
        }
        // Each one keeps its own time, so a raft of them never moves as a
        // block.
        let phase = (entry.at.cell.0 * 5 + entry.at.cell.1 * 3) as f32;
        let rest = world_position(entry.at, CRATE_SUNK_Y);
        // A buoy rides the surface up and down and stays where it is moored.
        // Two waves of different lengths rather than one, so the rise never
        // settles into a metronome.
        let swell = ((game.elapsed * 1.15) + phase).sin() * 0.1;
        let chop = ((game.elapsed * 2.3) + phase * 1.7).sin() * 0.025;
        if let Some(transform) = world.get_mut::<LocalTransform>(body) {
            transform.translation = rest + Vec3::new(0.0, swell + chop, 0.0);
        }
    }
}

/// Outlines whoever the controls are pointed at, using the engine's own
/// selection outline, the same one the editor draws round a selected entity,
/// seeded with the member and expanded by the render sync to everything under
/// it. With a party on the board there is otherwise no telling which one a key
/// is about to move.
fn outline_active(game: &SokobanResources, world: &mut World) {
    let party = game.map.party_size() > 1;
    let body = game.active_body();
    let parts = game
        .entities
        .member_parts
        .get(game.state.active)
        .cloned()
        .unwrap_or_default();

    let selection = world.res_mut::<Selection>();
    selection.outline_enabled = party;
    selection.active_entity = party.then_some(body);
    selection.entities = if party { parts } else { Vec::new() };
}

/// The call sign over the player's head. It only appears when the square they
/// stand on actually goes somewhere, so the prompt is the rule made visible
/// rather than a caption on the interface.
fn update_prompt(game: &SokobanResources, world: &mut World) {
    let entity = game.entities.prompt;
    if entity == Entity::default() {
        return;
    }
    let (down, up) = elevator_options(&game.map, &game.state);
    let label = match (up, down) {
        (true, true) => "Q  ▲   ▼  E",
        (true, false) => "Q  ▲",
        (false, true) => "▼  E",
        (false, false) => "",
    };

    let showing = !label.is_empty();
    let bob = (game.elapsed * 3.0).sin() * 0.05;
    let anchor = world_position(game.state.player(), PROMPT_Y + bob);
    if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
        transform.translation = anchor;
    }
    if world
        .get::<Visibility>(entity)
        .is_some_and(|visibility| visibility.visible != showing)
    {
        world.set(entity, Visibility { visible: showing });
    }
    if !showing {
        return;
    }

    let slot = world
        .get::<nightshade::ecs::text::components::Text>(entity)
        .map(|text| text.text_index);
    let Some(slot) = slot else {
        return;
    };
    let current = world
        .res::<nightshade::ecs::text::resources::TextState>()
        .cache
        .get_text(slot)
        .map(str::to_string);
    if current.as_deref() == Some(label) {
        return;
    }
    world
        .res_mut::<nightshade::ecs::text::resources::TextState>()
        .cache
        .set_text(slot, label);
    if let Some(text) = world.get_mut::<nightshade::ecs::text::components::Text>(entity) {
        text.dirty = true;
    }
}
