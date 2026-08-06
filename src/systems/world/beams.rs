//! The light, drawn. Where the beams go is decided by the rules, and this only
//! turns that answer into something on screen, and does it again whenever the
//! answer changes.

use crate::ecs::{LayerTag, LightPool, SokobanResources, TOWER_VISUAL, TowerVisual};
use crate::palette::LAMP_BODY;
use crate::rules::{BeamSegment, beam_field, lit_squares};
use crate::schema::{Position, Tile, map_tile};
use crate::systems::world::build::world_position;
use nightshade::prelude::*;

/// How high off the floor a beam runs, which is roughly the height of the thing
/// it is dangerous to.
const BEAM_Y: f32 = 0.45;
/// How thick a beam is drawn. It kills whoever touches it, so it is not a
/// hairline.
const BEAM_THICKNESS: f32 = 0.3;
/// How far back from the middle of a square a beam stops. A run is measured
/// between the middles of the squares it starts and ends on, and the thing at
/// either end stands on its own square, so a beam drawn the whole way buries
/// half a square of itself in the emitter that threw it and the wall that
/// stopped it.
const BEAM_INSET: f32 = 0.5;
/// How much brighter than its colour a beam is drawn. Past one the bloom pass
/// has something to find, which is what turns a bright bar into a glow.
const BEAM_GLOW: f32 = 3.2;
const TOWER_RATE: f32 = 9.0;
/// How high a pool of light lies over the floor, and how thick and wide it is
/// drawn. It is a slab rather than a decal because the floor is a slab, and a
/// coplanar face would fight it.
///
/// It rides above everything else laid flat on a square, because a marker, a
/// plate or a pad is a slab of its own and a pool drawn inside one is a pool
/// nobody can see. Nothing that stands up starts this low, so it passes through
/// nothing that matters.
const POOL_Y: f32 = 0.13;
const POOL_THICKNESS: f32 = 0.03;
const POOL_EXTENT: f32 = 0.92;
/// How much brighter than its colour a pool is drawn. Enough for the bloom pass
/// to find it, so a lit square reads as lit under the room's own lighting rather
/// than as a square somebody painted a different colour.
const POOL_GLOW: f32 = 1.35;

/// Rebuilds the beams when the board has moved under them, and lights the
/// towers they are reaching. Tracing is cheap and only maps with emitters pay
/// for it, so this asks every frame rather than trying to guess when to.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    pools(game, world);
    if game.map.emitters.is_empty() && game.map.gems.is_empty() {
        if !game.beams.is_empty() {
            clear(game, world);
        }
        return;
    }

    let field = beam_field(&game.map, &game.state);
    if field.segments != game.beam_shape {
        clear(game, world);
        for segment in &field.segments {
            spawn_beam(game, world, *segment);
        }
        game.beam_shape = field.segments.clone();
    }

    let delta = world.res::<Time>().delta_time;
    let entities: Vec<Entity> = world.ecs.worlds[GAME]
        .query_entities(TOWER_VISUAL)
        .collect();
    for entity in entities {
        let (base, lit) = {
            let Some(tower) = world.get_mut::<TowerVisual>(entity) else {
                continue;
            };
            let powered = field.powered.get(tower.group).copied().unwrap_or(false);
            let target = if powered { 1.0 } else { 0.0 };
            tower.lit += (target - tower.lit) * (1.0 - (-delta * TOWER_RATE).exp());
            (tower.base, tower.lit)
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = Vec3::new(base.x, base.y + lit * 0.08, base.z);
        }
        // The arc is built with the tower and shown when it is drinking, which
        // is steadier than spawning lightning and throwing it away again.
        let sparks = world.get::<TowerVisual>(entity).map(|tower| tower.sparks);
        if let Some(sparks) = sparks.filter(|sparks| world.is_alive(*sparks))
            && let Some(emitter) = world.get_mut::<ParticleEmitter>(sparks)
        {
            emitter.enabled = lit > 0.5;
        }
        let arc = world.get::<TowerVisual>(entity).map(|tower| tower.arc);
        if let Some(arc) = arc.filter(|arc| world.is_alive(*arc)) {
            let showing = lit > 0.5;
            if world
                .get::<Visibility>(arc)
                .is_some_and(|visibility| visibility.visible != showing)
            {
                world.set(arc, Visibility { visible: showing });
            }
        }
    }
}

/// Lays a pool of light on every square the rules call lit, and takes them up
/// again when the light moves. Which squares a lamp reaches is worked out from
/// where the lamps are and what is in the way, and which squares a colour lends
/// on is worked out from where the beams go, and neither of those is anything a
/// rendered light can be trusted to say: the room has a sun over it and a fill
/// light in each corner, and a lamp standing in that is a warm patch rather than
/// an answer. So the answer is drawn.
fn pools(game: &mut SokobanResources, world: &mut World) {
    let mut wanted: Vec<(Position, LightPool)> = Vec::new();
    if !game.map.lamps.is_empty() {
        wanted.extend(
            lit_squares(&game.map, &game.state)
                .into_iter()
                .map(|at| (at, LightPool::Lamp)),
        );
    }
    if !game.map.gems.is_empty() || !game.map.prisms.is_empty() {
        wanted.extend(
            beam_field(&game.map, &game.state)
                .aura
                .into_iter()
                .map(|(at, color)| (at, LightPool::Aura(color))),
        );
    }
    // Two colours crossing one square each lay their own pool, so the square
    // says both rather than whichever was traced last.
    wanted.sort_unstable_by_key(|(at, pool)| (at.layer, at.cell.1, at.cell.0, key(*pool)));
    wanted.dedup();
    if wanted == game.pool_shape {
        return;
    }

    for entity in std::mem::take(&mut game.pools) {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    for (at, pool) in &wanted {
        let entity = spawn_pool(world, *at, *pool);
        game.pools.push(entity);
    }
    game.pool_shape = wanted;
}

/// An order for the pools on one square, so a square lit two ways is drawn the
/// same way round every time and the shape it is compared against holds still.
fn key(pool: LightPool) -> u8 {
    match pool {
        LightPool::Lamp => 0,
        LightPool::Aura(color) => 1 + color as u8,
    }
}

fn spawn_pool(world: &mut World, at: Position, pool: LightPool) -> Entity {
    let (name, color, emissive) = match pool {
        LightPool::Lamp => (
            "sokoban_pool_lamp".to_string(),
            LAMP_BODY,
            [POOL_GLOW * 0.9, POOL_GLOW * 0.78, POOL_GLOW * 0.5],
        ),
        LightPool::Aura(tint) => {
            let body = crate::palette::gem_body(tint);
            (
                format!("sokoban_pool_{}", tint.label()),
                body,
                [
                    body[0] * POOL_GLOW,
                    body[1] * POOL_GLOW,
                    body[2] * POOL_GLOW,
                ],
            )
        }
    };
    // A second pool on the same square sits a hair above the first, so two
    // colours crossing read as two rather than as one fighting itself.
    let lift = POOL_Y + key(pool) as f32 * 0.012;
    let entity = crate::systems::world::build::block(
        world,
        "Cube",
        world_position(at, lift),
        Vec3::new(POOL_EXTENT, POOL_THICKNESS, POOL_EXTENT),
        name,
        Material {
            base_color: [color[0] * 0.6, color[1] * 0.6, color[2] * 0.6, 1.0],
            emissive_factor: emissive,
            roughness: 0.8,
            ..Default::default()
        },
    );
    // A pool is laid and taken up again every time the light moves, so it is
    // owned by this pass rather than by the map. The storey it belongs to is
    // still stamped on it, because the visibility pass shows one storey at a
    // time and a pool on another one has no business being on screen.
    world.set(entity, LayerTag { layer: at.layer });
    entity
}

/// A beam drawn as a stretched cylinder rather than a line effect, because a
/// mesh has a thickness that is exactly what it is told, which a bundle of strands does
/// not.
fn spawn_beam(game: &mut SokobanResources, world: &mut World, segment: BeamSegment) {
    let palette = crate::palette::palette_for(game.map.skin);
    // The light an emitter throws wears the room's colour and burns. The light
    // a gem throws wears the gem's and lends, so the two are told apart on
    // sight before anybody has to learn which is which.
    let color = match segment.tint {
        None => palette.beam,
        Some(tint) => crate::palette::gem_light(tint),
    };
    let mut start = world_position(segment.from, BEAM_Y);
    let mut end = world_position(segment.to, BEAM_Y);
    let along = end - start;
    let length = along.magnitude();
    if length <= f32::EPSILON {
        return;
    }
    let step = along / length;

    // Light leaves an emitter at its face rather than its middle, and stops at
    // the face of whatever stopped it. A mirror is neither, because the beam
    // turns at the middle of one, so the two runs that meet there have to meet.
    if matches!(
        map_tile(&game.map, segment.from),
        Tile::Emitter(_) | Tile::Socket(_)
    ) {
        start += step * BEAM_INSET;
    }
    if !matches!(
        map_tile(&game.map, segment.to),
        Tile::Mirror(_) | Tile::Prism(_) | Tile::Splitter
    ) {
        end -= step * BEAM_INSET;
    }
    let along = end - start;
    let length = along.magnitude();
    if length <= f32::EPSILON {
        return;
    }

    let entity = crate::systems::world::build::block(
        world,
        "Cylinder",
        start + along * 0.5,
        Vec3::new(BEAM_THICKNESS, length, BEAM_THICKNESS),
        match segment.tint {
            None => "sokoban_beam".to_string(),
            Some(tint) => format!("sokoban_beam_{}", tint.label()),
        },
        Material {
            // The surface itself is dim and the light coming off it is not. A
            // base colour above one only clips to white, which is a bar rather
            // than a beam.
            base_color: [color[0] * 0.18, color[1] * 0.18, color[2] * 0.18, 1.0],
            emissive_factor: [
                color[0] * BEAM_GLOW,
                color[1] * BEAM_GLOW,
                color[2] * BEAM_GLOW,
            ],
            roughness: 0.35,
            ..Default::default()
        },
    );
    // A cylinder stands on its own up axis, so it is turned onto the line it
    // has to lie along.
    let up = Vec3::new(0.0, 1.0, 0.0);
    let direction = along / length;
    let axis = up.cross(&direction);
    if axis.magnitude() > 1.0e-4 {
        let turn = up.dot(&direction).clamp(-1.0, 1.0).acos();
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.rotation = nalgebra_glm::quat_angle_axis(turn, &axis.normalize());
        }
    }

    // Like a pool, a beam is rebuilt whenever the light changes, so this pass
    // owns it and the map's own teardown list never hears about it.
    world.set(
        entity,
        LayerTag {
            layer: segment.from.layer,
        },
    );
    game.beams.push(entity);
}

fn clear(game: &mut SokobanResources, world: &mut World) {
    for entity in std::mem::take(&mut game.beams) {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    game.beam_shape.clear();
}
