//! The fixed half of a map made solid: plinths, floor squares, walls, and the
//! props that sit on a square because of what its tile is.

use crate::ecs::{
    BrittleVisual, FragileVisual, GateVisual, PlateVisual, SensorVisual, SocketVisual,
    SokobanResources, SpikeVisual, Spinner, SwitchVisual, TowerVisual,
};
use crate::palette::{
    GLASS_BODY, GLASS_TINT, INCINERATOR_FLAME, INCINERATOR_GLOW, INCINERATOR_LEAF, MIRROR_FACE,
    Palette, SOCKET_BODY, SPIKE_BODY, gem_body, gem_light, palette_for,
};
use crate::schema::{
    Direction, GemColor, Map, Position, Slant, Slot, Tile, map_floor_index, map_floor_skin,
    map_positions, map_tile,
};
use crate::systems::world::build::{
    GATE_CLOSED_Y, block, chrome, frozen, glazed, glowing, layer_height, machined, solid,
    track_entity, world_position,
};
use nightshade::prelude::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

pub fn build_terrain(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    // A plinth per floor rather than per storey, so a lattice of floors reads
    // as rooms standing side by side and each can be made of its own thing.
    for slot in game
        .map
        .floors
        .iter()
        .map(|floor| floor.slot)
        .collect::<Vec<Slot>>()
    {
        build_plinth(game, world, slot);
    }
    for at in map_positions(&game.map) {
        let skin = map_floor_skin(&game.map, at);
        let local = palette_for(skin);
        build_square(
            game,
            if skin == game.map.skin {
                palette
            } else {
                &local
            },
            world,
            at,
        );
    }
    build_water_runs(game, palette, world);
}

/// Lays one water surface over each unbroken run of flooded squares. A surface
/// is a wave pattern rather than a tile, so a run of ten wants one of them
/// stretched across it, so the waves carry along the whole stretch instead of
/// restarting at every seam, and a flooded floor costs the renderer a handful of
/// bodies rather than one per square.
fn build_water_runs(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    let mut flooded: Vec<Position> = map_positions(&game.map)
        .into_iter()
        .filter(|at| map_tile(&game.map, *at) == Tile::Water)
        .collect();
    // Along each row in turn, so squares that touch end up next to each other
    // and a run is a stretch of this list rather than something to search for.
    flooded.sort_unstable_by_key(|at| (at.layer, at.cell.1, at.cell.0));

    let mut run: Vec<Position> = Vec::new();
    for at in flooded {
        let joins = run.last().is_some_and(|last| {
            last.layer == at.layer && last.cell.1 == at.cell.1 && last.cell.0 + 1 == at.cell.0
        });
        if !joins && !run.is_empty() {
            build_water(game, palette, world, &run);
            run.clear();
        }
        run.push(at);
    }
    if !run.is_empty() {
        build_water(game, palette, world, &run);
    }
}

/// How far a plinth stands out past the floor it carries, and how far the rim
/// stands out past that.
const PLINTH_LIP: f32 = 0.3;
const RIM_LIP: f32 = 0.8;

fn build_plinth(game: &mut SokobanResources, world: &mut World, slot: Slot) {
    let minimum = (
        slot.column * game.map.floor_width,
        slot.row * game.map.floor_height,
    );
    let palette = &palette_for(map_floor_skin(
        &game.map,
        Position::new(slot.layer, minimum),
    ));
    let layer = slot.layer;

    let (center, extent) = slab(&game.map, slot, PLINTH_LIP);
    let plinth = block(
        world,
        "Cube",
        Vec3::new(center.x, layer_height(layer) - 0.8, center.y),
        Vec3::new(extent.x, 0.6, extent.y),
        "sokoban_plinth",
        solid(palette.plinth, 0.85, 0.0),
    );
    track_entity(game, world, plinth, layer);

    let (center, extent) = slab(&game.map, slot, RIM_LIP);
    let rim = block(
        world,
        "Cube",
        Vec3::new(center.x, layer_height(layer) - 1.2, center.y),
        Vec3::new(extent.x, 0.3, extent.y),
        "sokoban_plinth_rim",
        solid(palette.plinth_rim, 0.9, 0.0),
    );
    track_entity(game, world, rim, layer);
}

/// Where a floor's plinth begins and ends, given how far it is meant to stand
/// out past the squares it carries.
///
/// A side with a floor against it gets no lip at all. Two floors laid side by
/// side each standing out over the seam are two slabs sharing a volume with
/// their faces at one height, which is a whole board's worth of surfaces
/// fighting for the same pixels. The depot is six floors in a lattice, so that
/// is what it looked like everywhere at once.
fn slab(map: &Map, slot: Slot, lip: f32) -> (Vec2, Vec2) {
    let minimum = (slot.column * map.floor_width, slot.row * map.floor_height);
    let maximum = (
        minimum.0 + map.floor_width - 1,
        minimum.1 + map.floor_height - 1,
    );
    let against = |column: i32, row: i32| {
        map_floor_index(
            map,
            Slot {
                column: slot.column + column,
                row: slot.row + row,
                layer: slot.layer,
            },
        )
        .is_some()
    };
    // A side gives up its lip to the floor beside it, and to the one across the
    // corner as well: two floors meeting only at a corner would otherwise still
    // lay a lip square of one over a lip square of the other, which is the same
    // fight over a smaller patch.
    let west = against(-1, 0) || against(-1, -1) || against(-1, 1);
    let east = against(1, 0) || against(1, -1) || against(1, 1);
    let north = against(0, -1) || against(-1, -1) || against(1, -1);
    let south = against(0, 1) || against(-1, 1) || against(1, 1);
    let out = |neighbour: bool| if neighbour { 0.0 } else { lip };
    // A square is a unit wide centred on its cell, so a floor's own edge is
    // half a unit past the outermost one.
    let low = Vec2::new(
        minimum.0 as f32 - 0.5 - out(west),
        minimum.1 as f32 - 0.5 - out(north),
    );
    let high = Vec2::new(
        maximum.0 as f32 + 0.5 + out(east),
        maximum.1 as f32 + 0.5 + out(south),
    );
    ((low + high) * 0.5, high - low)
}

fn build_square(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let tile = map_tile(&game.map, at);
    if tile == Tile::Void {
        return;
    }

    if tile == Tile::Pit {
        let shaft = block(
            world,
            "Cube",
            world_position(at, -0.3),
            Vec3::new(0.98, 0.42, 0.98),
            "sokoban_pit",
            solid(palette.pit, 1.0, 0.0),
        );
        track_entity(game, world, shaft, at.layer);
        return;
    }

    // Ice and water are both a surface over a bed rather than a coloured floor
    // tile. The bed is what the surface above it has to refract, so without one
    // there is nothing for either to be transparent about.
    if tile == Tile::Ice || tile == Tile::Water {
        let bed = block(
            world,
            "Cube",
            world_position(at, -0.4),
            Vec3::new(0.98, 0.2, 0.98),
            "sokoban_bed",
            solid(palette.bed, 0.65, 0.0),
        );
        track_entity(game, world, bed, at.layer);

        if tile == Tile::Ice {
            let sheet = block(
                world,
                "Cube",
                world_position(at, -0.14),
                Vec3::new(1.0, 0.32, 1.0),
                "sokoban_ice",
                frozen(palette.ice, palette.ice_tint),
            );
            track_entity(game, world, sheet, at.layer);
        }
        // The surface over the water is not built here. A square of it is a
        // wave pattern one square wide, and a flooded floor wants one stretch
        // of water rather than sixty of them side by side, so the surfaces are
        // laid over whole runs once every square has its bed.
        return;
    }

    // A burner is a fire with steel over it, so like the other holes it is
    // built downwards rather than laid on a floor tile that would fill it in.
    if tile == Tile::Incinerator {
        build_incinerator(game, palette, world, at);
        return;
    }

    // A fragile square is a hole with a lid on it. Building the hole now means
    // the collapse is the lid dropping rather than the board being rebuilt.
    if tile == Tile::Fragile {
        let shaft = block(
            world,
            "Cube",
            world_position(at, -0.3),
            Vec3::new(0.98, 0.42, 0.98),
            "sokoban_pit",
            solid(palette.pit, 1.0, 0.0),
        );
        track_entity(game, world, shaft, at.layer);

        let base = world_position(at, -0.22);
        let lid = block(
            world,
            "Cube",
            base,
            Vec3::new(0.9, 0.44, 0.9),
            "sokoban_fragile",
            glowing(palette.fragile, [0.06, 0.05, 0.05], 0.75),
        );
        world.set(
            lid,
            FragileVisual {
                at,
                base,
                fall: 0.0,
            },
        );
        track_entity(game, world, lid, at.layer);
        return;
    }

    let (name, material) = match tile {
        Tile::Conveyor(_) => ("sokoban_belt", solid(palette.belt, 0.55, 0.05)),
        _ if (at.cell.0 + at.cell.1).rem_euclid(2) == 0 => {
            ("sokoban_floor_light", solid(palette.floor_light, 0.9, 0.0))
        }
        _ => ("sokoban_floor_dark", solid(palette.floor_dark, 0.9, 0.0)),
    };
    let floor = block(
        world,
        "Cube",
        world_position(at, -0.25),
        Vec3::new(1.0, 0.5, 1.0),
        name,
        material,
    );
    track_entity(game, world, floor, at.layer);

    match tile {
        Tile::Wall => build_wall(game, palette, world, at),
        Tile::Brittle => build_brittle(game, palette, world, at),
        Tile::Plate(group) => build_plate(game, palette, world, at, group as usize),
        Tile::Gate(group) => build_gate(game, palette, world, at, group as usize),
        Tile::Portal => build_portal(game, palette, world, at),
        Tile::Elevator => build_elevator(game, palette, world, at),
        Tile::OneWay(way) => build_one_way(game, palette, world, at, way),
        Tile::Conveyor(way) => build_belt(game, palette, world, at, way),
        Tile::Switch(group) => build_switch(game, palette, world, at, group as usize),
        Tile::Gateway(_) => build_gateway(game, palette, world, at),
        Tile::Emitter(way) => build_emitter(game, palette, world, at, way),
        Tile::Mirror(slant) => build_mirror(game, palette, world, at, slant),
        Tile::Receiver(group) => build_receiver(game, palette, world, at, group as usize),
        Tile::Sensor(group) => build_sensor(game, palette, world, at, group as usize),
        Tile::Socket(way) => build_socket(game, palette, world, at, way),
        Tile::Glass => build_glass(game, palette, world, at),
        Tile::Prism(color) => build_prism(game, palette, world, at, color),
        Tile::Splitter => build_splitter(game, palette, world, at),
        Tile::Spike(group) => build_spikes(game, world, at, group as usize),
        _ => {}
    }
}

/// A plinth with a claw on top: somewhere for a gem to sit, pointed the way the
/// gem will throw its light. The gem itself is not built here, because a gem is
/// carried and belongs with the things that move.
fn build_socket(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    way: Direction,
) {
    let ring = block(
        world,
        "Cylinder",
        world_position(at, 0.06),
        Vec3::new(0.9, 0.12, 0.9),
        "sokoban_socket_ring",
        machined(palette.wall_cap, [0.05, 0.05, 0.06]),
    );
    track_entity(game, world, ring, at.layer);

    let base = world_position(at, 0.3);
    let column = block(
        world,
        "Cylinder",
        base,
        Vec3::new(0.38, 0.36, 0.38),
        "sokoban_socket",
        machined(SOCKET_BODY, [0.08, 0.07, 0.05]),
    );
    world.set(column, SocketVisual { at, base, lit: 0.0 });
    track_entity(game, world, column, at.layer);

    // A muzzle on the side it fires from, so an empty socket still says which
    // way it would throw its light if it had any.
    let delta = way.delta();
    let muzzle = block(
        world,
        "Cube",
        world_position(at, 0.34) + Vec3::new(delta.0 as f32 * 0.3, 0.0, delta.1 as f32 * 0.3),
        Vec3::new(0.22, 0.16, 0.22),
        "sokoban_socket_muzzle",
        machined(palette.wall_cap, [0.05, 0.05, 0.06]),
    );
    track_entity(game, world, muzzle, at.layer);
}

/// A pane: a real transmissive surface rather than a pale block, because what
/// makes glass read as glass is seeing through it. It is set in a frame and
/// stands a little proud of neither wall, so the eye reads a window in a run of
/// wall rather than a wall somebody painted lighter.
fn build_glass(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    // Two thin sheets crossed rather than one solid block. A block of
    // transmissive material a whole square thick reads as a coloured lump,
    // because what says glass is a thin sheet with a highlight running along it
    // and the room showing through, and a sheet has to face somewhere.
    for turn in [0.0_f32, FRAC_PI_2] {
        let pane = block(
            world,
            "Cube",
            world_position(at, 0.48),
            Vec3::new(0.94, 0.94, 0.06),
            "sokoban_glass",
            glazed(GLASS_BODY, GLASS_TINT),
        );
        if let Some(transform) = world.get_mut::<LocalTransform>(pane) {
            transform.rotation = nalgebra_glm::quat_angle_axis(turn, &Vec3::new(0.0, 1.0, 0.0));
        }
        track_entity(game, world, pane, at.layer);
    }

    // The frame it is set in: a sill, a head, and a post at each corner, so the
    // sheet reads as glazing in an opening rather than as a pale wall.
    for height in [0.03_f32, 0.97] {
        let rail = block(
            world,
            "Cube",
            world_position(at, height),
            Vec3::new(1.0, 0.1, 1.0),
            "sokoban_glass_frame",
            machined(palette.wall_cap, [0.04, 0.04, 0.05]),
        );
        track_entity(game, world, rail, at.layer);
    }
    for corner in [
        (-0.46_f32, -0.46_f32),
        (0.46, -0.46),
        (-0.46, 0.46),
        (0.46, 0.46),
    ] {
        let post = block(
            world,
            "Cube",
            world_position(at, 0.5) + Vec3::new(corner.0, 0.0, corner.1),
            Vec3::new(0.1, 0.94, 0.1),
            "sokoban_glass_frame",
            machined(palette.wall_cap, [0.04, 0.04, 0.05]),
        );
        track_entity(game, world, post, at.layer);
    }
}

/// A lens on a mount, in the colour it stains the light. It is turned on its
/// corner so it reads as cut rather than moulded.
fn build_prism(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    color: GemColor,
) {
    let mount = block(
        world,
        "Cylinder",
        world_position(at, 0.07),
        Vec3::new(0.62, 0.14, 0.62),
        "sokoban_prism_mount",
        machined(palette.wall_cap, [0.03, 0.03, 0.04]),
    );
    track_entity(game, world, mount, at.layer);

    let body = gem_body(color);
    let lens = block(
        world,
        "Cube",
        world_position(at, 0.46),
        Vec3::new(0.66, 0.66, 0.66),
        format!("sokoban_prism_{}", color.label()),
        glowing(body, gem_light(color), 0.12),
    );
    if let Some(transform) = world.get_mut::<LocalTransform>(lens) {
        transform.rotation = nalgebra_glm::quat_angle_axis(FRAC_PI_4, &Vec3::new(0.0, 1.0, 0.0));
    }
    track_entity(game, world, lens, at.layer);
}

/// A wedge, drawn as the two mirrors it behaves like: one leaning each way, so
/// what happens to a beam that meets it is on the square in front of you.
fn build_splitter(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let mount = block(
        world,
        "Cylinder",
        world_position(at, 0.07),
        Vec3::new(0.62, 0.14, 0.62),
        "sokoban_splitter_mount",
        machined(palette.wall_cap, [0.03, 0.03, 0.04]),
    );
    track_entity(game, world, mount, at.layer);

    for turn in [-FRAC_PI_4, FRAC_PI_4] {
        let face = block(
            world,
            "Cube",
            world_position(at, 0.42),
            Vec3::new(1.16, 0.74, 0.08),
            "sokoban_splitter",
            chrome(MIRROR_FACE),
        );
        if let Some(transform) = world.get_mut::<LocalTransform>(face) {
            transform.rotation = nalgebra_glm::quat_angle_axis(turn, &Vec3::new(0.0, 1.0, 0.0));
        }
        track_entity(game, world, face, at.layer);
    }
}

/// A bed of spikes: a grid of points that stand up while their group is
/// powered. They are built down and raised by the props pass, so the board
/// never has to be rebuilt for a gate opening.
fn build_spikes(game: &mut SokobanResources, world: &mut World, at: Position, group: usize) {
    for offset in [
        (-0.22_f32, -0.22_f32),
        (0.22, -0.22),
        (-0.22, 0.22),
        (0.22, 0.22),
    ] {
        let base = world_position(at, -0.16) + Vec3::new(offset.0, 0.0, offset.1);
        let spike = block(
            world,
            "Cone",
            base,
            Vec3::new(0.26, 0.5, 0.26),
            format!("sokoban_spike_{group}"),
            machined(SPIKE_BODY, [0.06, 0.06, 0.07]),
        );
        world.set(
            spike,
            SpikeVisual {
                group,
                base,
                raised: 0.0,
            },
        );
        track_entity(game, world, spike, at.layer);
    }
}

/// A burner set into the floor: a fire under two steel leaves that meet down
/// the middle. The seam glows, which is the whole of what tells you this square
/// is not a floor tile, and the leaves are what a body walks over and a crate
/// goes through. What it burns is gone, so nothing about it ever fills.
fn build_incinerator(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
) {
    // The fire, sunk far enough that what is seen of it is the light it throws
    // up through the seam rather than a bright square on the floor.
    let fire = block(
        world,
        "Cube",
        world_position(at, -0.4),
        Vec3::new(0.86, 0.3, 0.86),
        "sokoban_burner_fire",
        glowing(INCINERATOR_GLOW, INCINERATOR_FLAME, 0.9),
    );
    track_entity(game, world, fire, at.layer);

    let hearth = spawn_light_entity(world, world_position(at, 0.12), "SokobanBurner");
    world.set(
        hearth,
        Light {
            light_type: LightType::Point,
            color: Vec3::new(1.0, 0.42, 0.12),
            intensity: 9.0,
            range: 2.4,
            cast_shadows: false,
            ..Default::default()
        },
    );
    track_entity(game, world, hearth, at.layer);

    // Two leaves with the seam between them, and the rim they are hung in.
    for side in [-0.255_f32, 0.255] {
        let leaf = block(
            world,
            "Cube",
            world_position(at, -0.1) + Vec3::new(side, 0.0, 0.0),
            Vec3::new(0.46, 0.2, 0.96),
            "sokoban_burner",
            machined(INCINERATOR_LEAF, [0.06, 0.02, 0.0]),
        );
        track_entity(game, world, leaf, at.layer);
    }
    for (offset, size) in [
        (Vec3::new(0.0, 0.0, -0.46), Vec3::new(1.0, 0.22, 0.08)),
        (Vec3::new(0.0, 0.0, 0.46), Vec3::new(1.0, 0.22, 0.08)),
    ] {
        let rim = block(
            world,
            "Cube",
            world_position(at, -0.09) + offset,
            size,
            "sokoban_burner_rim",
            machined(palette.wall_cap, [0.03, 0.03, 0.04]),
        );
        track_entity(game, world, rim, at.layer);
    }

    // The heat coming off it, which is the one thing on the board that says
    // this square is working rather than merely built.
    let heat = crate::systems::world::build::spawn_emitter(
        world,
        world_position(at, 0.06),
        ParticleEmitter {
            shape: EmitterShape::Box {
                half_extents: Vec3::new(0.04, 0.01, 0.42),
            },
            spawn_rate: 18.0,
            particle_lifetime_min: 0.5,
            particle_lifetime_max: 1.1,
            initial_velocity_min: 0.25,
            initial_velocity_max: 0.7,
            velocity_spread: 0.35,
            gravity: Vec3::new(0.0, 0.9, 0.0),
            drag: 0.7,
            size_start: 0.05,
            size_end: 0.0,
            color_gradient: ColorGradient::fire(),
            emissive_strength: 2.6,
            enabled: true,
            ..Default::default()
        },
    );
    track_entity(game, world, heat, at.layer);
}

/// A real water surface rather than a blue tile: the engine's own waves, depth
/// fade and edge foam, sized to the one square it floods.
fn build_water(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    run: &[Position],
) {
    let Some(head) = run.first().copied() else {
        return;
    };
    let length = run.len() as f32;
    // The run is laid along the row, so its middle is half a run east of the
    // square it starts on.
    let centre = world_position(head, -0.12) + Vec3::new((length - 1.0) * 0.5, 0.0, 0.0);
    let surface = world.spawn_with((
        LocalTransform {
            translation: centre,
            ..Default::default()
        },
        GlobalTransform::default(),
        Water {
            enabled: true,
            half_extents: Vec2::new(length * 0.5, 0.5),
            tessellation: 24,
            wave_amplitude: 0.02,
            wave_steepness: 0.3,
            wave_length: 1.2,
            wave_speed: 0.7,
            shallow_color: palette.water_shallow,
            deep_color: palette.water_deep,
            depth_fade_distance: 0.7,
            edge_foam_distance: 0.16,
            foam_amount: 0.45,
            roughness: 0.04,
            ..Default::default()
        },
    ));
    track_entity(game, world, surface, head.layer);
}

/// One arrowhead on the floor: where it sits relative to the square's centre,
/// how big it is, and what it is made of.
struct Chevron<'a> {
    offset: Vec3,
    scale: Vec3,
    name: &'a str,
    material: Material,
}

/// A cone laid flat so its point reads as a direction on the floor. The cone
/// stands along its own up axis, so it is tipped forward once and then yawed
/// the same way anything else on this board is yawed.
fn chevron(
    game: &mut SokobanResources,
    world: &mut World,
    at: Position,
    way: Direction,
    spec: Chevron,
) {
    let cone = block(
        world,
        "Cone",
        world_position(at, 0.0) + spec.offset,
        spec.scale,
        spec.name,
        spec.material,
    );
    let rotation = nalgebra_glm::quat_angle_axis(way.yaw(), &Vec3::new(0.0, 1.0, 0.0))
        * nalgebra_glm::quat_angle_axis(-FRAC_PI_2, &Vec3::new(1.0, 0.0, 0.0));
    if let Some(transform) = world.get_mut::<LocalTransform>(cone) {
        transform.rotation = rotation;
    }
    track_entity(game, world, cone, at.layer);
}

fn build_one_way(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    way: Direction,
) {
    chevron(
        game,
        world,
        at,
        way,
        Chevron {
            offset: Vec3::new(0.0, 0.06, 0.0),
            scale: Vec3::new(0.5, 0.62, 0.16),
            name: "sokoban_one_way",
            material: glowing(palette.one_way, [0.5, 0.4, 0.08], 0.35),
        },
    );
}

/// Two chevrons rather than one, so a belt never reads as an arrow that merely
/// forbids the other way.
fn build_belt(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    way: Direction,
) {
    let delta = way.delta();
    let along = Vec3::new(delta.0 as f32, 0.0, delta.1 as f32);
    for step in [-0.22_f32, 0.2] {
        chevron(
            game,
            world,
            at,
            way,
            Chevron {
                offset: along * step + Vec3::new(0.0, 0.05, 0.0),
                scale: Vec3::new(0.46, 0.34, 0.14),
                name: "sokoban_belt_chevron",
                material: glowing(palette.belt, [0.1, 0.34, 0.24], 0.3),
            },
        );
    }
}

/// The source: a squat housing with a lens facing the way it fires.
fn build_emitter(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    way: Direction,
) {
    let housing = block(
        world,
        "Cube",
        world_position(at, 0.3),
        Vec3::new(0.72, 0.6, 0.72),
        "sokoban_emitter",
        machined(palette.wall_cap, [0.05, 0.05, 0.06]),
    );
    track_entity(game, world, housing, at.layer);

    let delta = way.delta();
    let lens = block(
        world,
        "Sphere",
        world_position(at, 0.34) + Vec3::new(delta.0 as f32 * 0.3, 0.0, delta.1 as f32 * 0.3),
        Vec3::new(0.26, 0.26, 0.26),
        "sokoban_emitter_lens",
        glowing(
            [palette.beam[0], palette.beam[1], palette.beam[2], 1.0],
            palette.beam,
            0.1,
        ),
    );
    track_entity(game, world, lens, at.layer);
}

/// A mirror: a polished plate stood on edge at forty five degrees, which is
/// what the beam is bouncing off and what the eye should read it as. The plate
/// is polished on both sides, because light arriving from either turns at it,
/// so what frames it is a dark mount under it rather than a backing behind it.
fn build_mirror(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    slant: Slant,
) {
    let turn = match slant {
        Slant::Forward => -FRAC_PI_4,
        Slant::Back => FRAC_PI_4,
    };

    let mount = block(
        world,
        "Cylinder",
        world_position(at, 0.07),
        Vec3::new(0.62, 0.14, 0.62),
        "sokoban_mirror_mount",
        machined(palette.wall_cap, [0.03, 0.03, 0.04]),
    );
    track_entity(game, world, mount, at.layer);

    let face = block(
        world,
        "Cube",
        world_position(at, 0.42),
        Vec3::new(1.24, 0.82, 0.1),
        "sokoban_mirror",
        chrome(MIRROR_FACE),
    );
    if let Some(transform) = world.get_mut::<LocalTransform>(face) {
        transform.rotation = nalgebra_glm::quat_angle_axis(turn, &Vec3::new(0.0, 1.0, 0.0));
    }
    track_entity(game, world, face, at.layer);
}

/// A tower: a mast that drinks a beam. It lights when one reaches it, which the
/// props pass looks after.
fn build_receiver(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    group: usize,
) {
    let base = block(
        world,
        "Cylinder",
        world_position(at, 0.12),
        Vec3::new(0.86, 0.24, 0.86),
        format!("sokoban_tower_base_{group}"),
        machined(palette.tower, [0.06, 0.05, 0.06]),
    );
    track_entity(game, world, base, at.layer);

    let mast = block(
        world,
        "Cylinder",
        world_position(at, 0.62),
        Vec3::new(0.34, 0.8, 0.34),
        format!("sokoban_tower_{group}"),
        glowing(palette.tower, [0.04, 0.03, 0.04], 0.35),
    );
    let arc = world.spawn_with((
        LightningBolt {
            start: world_position(at, 0.24),
            end: world_position(at, 1.5),
            color: Vec3::new(palette.beam[0], palette.beam[1], palette.beam[2]),
            intensity: 2.0,
            alpha: 0.85,
            segments: 9,
            jaggedness: 0.11,
            branch_count: 2,
            branch_spread: 0.28,
            regen_interval: 0.07,
            timer: 0.0,
            seed: (at.cell.0 * 31 + at.cell.1).unsigned_abs(),
        },
        nightshade::ecs::lines::components::Lines::default(),
        LocalTransform::default(),
        GlobalTransform::default(),
        Visibility { visible: false },
    ));
    track_entity(game, world, arc, at.layer);

    let sparks = crate::systems::world::build::spawn_emitter(
        world,
        world_position(at, 1.1),
        ParticleEmitter {
            shape: EmitterShape::Sphere { radius: 0.16 },
            spawn_rate: 26.0,
            particle_lifetime_min: 0.25,
            particle_lifetime_max: 0.6,
            initial_velocity_min: 0.6,
            initial_velocity_max: 1.8,
            velocity_spread: 2.4,
            gravity: Vec3::new(0.0, -3.0, 0.0),
            drag: 0.3,
            size_start: 0.06,
            size_end: 0.0,
            color_gradient: ColorGradient::sparks(),
            emissive_strength: 3.0,
            // Off until a beam reaches it, which the props pass decides.
            enabled: false,
            ..Default::default()
        },
    );
    track_entity(game, world, sparks, at.layer);

    world.set(
        mast,
        TowerVisual {
            group,
            base: world_position(at, 0.62),
            lit: 0.0,
            arc,
            sparks,
        },
    );
    track_entity(game, world, mast, at.layer);
}

/// A sensor: a pad with an eye that opens when a lamp reaches it. What lights
/// it is the rules' answer rather than the picture's, and the props pass keeps
/// the two agreeing.
fn build_sensor(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    group: usize,
) {
    let ring = block(
        world,
        "Cylinder",
        world_position(at, 0.05),
        Vec3::new(0.88, 0.1, 0.88),
        format!("sokoban_sensor_ring_{group}"),
        machined(palette.tower, [0.05, 0.05, 0.06]),
    );
    track_entity(game, world, ring, at.layer);

    let base = world_position(at, 0.18);
    let eye = block(
        world,
        "Sphere",
        base,
        Vec3::new(0.34, 0.34, 0.34),
        format!("sokoban_sensor_{group}"),
        glowing(palette.goal, [0.04, 0.05, 0.06], 0.25),
    );
    world.set(eye, SensorVisual { at, base, lit: 0.0 });
    track_entity(game, world, eye, at.layer);
}

/// A door: a lit pad with an arch over it, so it reads as somewhere to go from
/// across the room rather than as another marked square.
fn build_gateway(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let level = match map_tile(&game.map, at) {
        Tile::Gateway(level) => level as usize,
        _ => 0,
    };
    let cleared = game.story.cleared.get(level).copied().unwrap_or(false);
    let open = crate::story::level_unlocked(level, &game.story.cleared);
    let (color, emissive) = match (open, cleared) {
        (false, _) => (palette.wall, [0.06, 0.05, 0.06]),
        (true, true) => (palette.goal_done, [0.2, 0.6, 0.3]),
        (true, false) => (palette.goal, [0.28, 0.5, 0.7]),
    };

    let pad = block(
        world,
        "Cylinder",
        world_position(at, 0.05),
        Vec3::new(0.92, 0.1, 0.92),
        format!(
            "sokoban_gateway_pad_{}",
            u8::from(open) * 2 + u8::from(cleared)
        ),
        glowing(color, emissive, 0.3),
    );
    track_entity(game, world, pad, at.layer);

    for side in [-0.36_f32, 0.36] {
        let post = block(
            world,
            "Cube",
            world_position(at, 0.55) + Vec3::new(side, 0.0, 0.0),
            Vec3::new(0.16, 1.1, 0.18),
            "sokoban_gateway_post",
            glowing(palette.wall_cap, [0.16, 0.22, 0.3], 0.5),
        );
        track_entity(game, world, post, at.layer);
    }

    let lintel = block(
        world,
        "Cube",
        world_position(at, 1.16),
        Vec3::new(0.9, 0.16, 0.22),
        format!("sokoban_gateway_lintel_{}", u8::from(open)),
        glowing(color, emissive, 0.35),
    );
    track_entity(game, world, lintel, at.layer);
}

fn build_switch(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    group: usize,
) {
    let base = world_position(at, 0.16);
    let lever = block(
        world,
        "Cylinder",
        base,
        Vec3::new(0.3, 0.32, 0.3),
        format!("sokoban_switch_{group}"),
        machined(palette.switch, [0.4, 0.1, 0.38]),
    );
    world.set(
        lever,
        SwitchVisual {
            group,
            base,
            thrown: 0.0,
        },
    );
    track_entity(game, world, lever, at.layer);

    let ring = block(
        world,
        "Cylinder",
        world_position(at, 0.04),
        Vec3::new(0.8, 0.08, 0.8),
        format!("sokoban_switch_ring_{group}"),
        glowing(palette.switch, [0.22, 0.05, 0.2], 0.45),
    );
    track_entity(game, world, ring, at.layer);
}

fn build_wall(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let body = block(
        world,
        "Cube",
        world_position(at, 0.5),
        Vec3::new(1.0, 1.0, 1.0),
        "sokoban_wall",
        solid(palette.wall, 0.8, 0.0),
    );
    track_entity(game, world, body, at.layer);

    let cap = block(
        world,
        "Cube",
        world_position(at, 1.02),
        Vec3::new(0.92, 0.1, 0.92),
        "sokoban_wall_cap",
        solid(palette.wall_cap, 0.7, 0.0),
    );
    track_entity(game, world, cap, at.layer);
}

/// A wall built to be lost. It sits on the same square a floor tile already
/// covers, so when it sinks there is ground underneath rather than a hole.
fn build_brittle(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let base = world_position(at, 0.46);
    let body = block(
        world,
        "Cube",
        base,
        Vec3::new(0.9, 0.92, 0.9),
        "sokoban_brittle",
        glowing(palette.brittle, [0.09, 0.07, 0.05], 0.9),
    );
    world.set(
        body,
        BrittleVisual {
            at,
            base,
            fall: 0.0,
        },
    );
    track_entity(game, world, body, at.layer);
}

fn build_plate(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    group: usize,
) {
    let base = world_position(at, 0.05);
    let pad = block(
        world,
        "Cylinder",
        base,
        Vec3::new(0.86, 0.1, 0.86),
        format!("sokoban_plate_{group}"),
        machined(palette.plate, [0.5, 0.28, 0.05]),
    );
    world.set(
        pad,
        PlateVisual {
            at,
            base,
            pressed: 0.0,
        },
    );
    track_entity(game, world, pad, at.layer);
}

fn build_gate(
    game: &mut SokobanResources,
    palette: &Palette,
    world: &mut World,
    at: Position,
    group: usize,
) {
    let base = world_position(at, GATE_CLOSED_Y);
    let gate = block(
        world,
        "Cube",
        base,
        Vec3::new(0.94, 1.0, 0.94),
        format!("sokoban_gate_{group}"),
        machined(palette.gate, [0.32, 0.04, 0.05]),
    );
    world.set(
        gate,
        GateVisual {
            group,
            at,
            base,
            openness: 0.0,
        },
    );
    track_entity(game, world, gate, at.layer);

    let frame = block(
        world,
        "Cube",
        world_position(at, 1.02),
        Vec3::new(0.98, 0.1, 0.98),
        "sokoban_gate_frame",
        solid(palette.wall_cap, 0.7, 0.0),
    );
    track_entity(game, world, frame, at.layer);
}

fn build_portal(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let pair = game
        .map
        .portals
        .iter()
        .position(|(first, second)| *first == at || *second == at)
        .unwrap_or(0);
    let color = palette.portals[pair % palette.portals.len()];
    let emissive = [color[0] * 0.5, color[1] * 0.5, color[2] * 0.75];

    let pad = block(
        world,
        "Cylinder",
        world_position(at, 0.05),
        Vec3::new(0.94, 0.1, 0.94),
        format!("sokoban_portal_{pair}"),
        glowing(color, emissive, 0.3),
    );
    track_entity(game, world, pad, at.layer);

    let core_base = world_position(at, 0.62);
    let core = block(
        world,
        "Cube",
        core_base,
        Vec3::new(0.26, 0.26, 0.26),
        format!("sokoban_portal_core_{pair}"),
        glowing(color, [color[0], color[1], color[2]], 0.25),
    );
    world.set(
        core,
        Spinner {
            base: core_base,
            spin_speed: 1.6,
            bob_height: 0.09,
            bob_speed: 2.2,
            phase: (at.cell.0 + at.cell.1) as f32 * 0.9,
        },
    );
    track_entity(game, world, core, at.layer);
}

fn build_elevator(game: &mut SokobanResources, palette: &Palette, world: &mut World, at: Position) {
    let pad = block(
        world,
        "Cylinder",
        world_position(at, 0.06),
        Vec3::new(0.9, 0.12, 0.9),
        "sokoban_elevator",
        machined(palette.elevator, [0.18, 0.36, 0.46]),
    );
    track_entity(game, world, pad, at.layer);

    let mast_base = world_position(at, 0.78);
    let mast = block(
        world,
        "Cone",
        mast_base,
        Vec3::new(0.34, 0.36, 0.34),
        "sokoban_elevator_mast",
        glowing(palette.elevator, [0.3, 0.62, 0.78], 0.2),
    );
    world.set(
        mast,
        Spinner {
            base: mast_base,
            spin_speed: 0.9,
            bob_height: 0.12,
            bob_speed: 1.6,
            phase: (at.cell.0 - at.cell.1) as f32 * 0.7,
        },
    );
    track_entity(game, world, mast, at.layer);
}
