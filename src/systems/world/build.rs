//! Turning one map value into a world. Every storey is built once, and which
//! one is on show is a visibility question rather than a rebuilding one.

mod actors;
mod terrain;

use crate::ecs::{LayerTag, MapEntities, MapRequest, Playback, SokobanResources};
use crate::palette::{Palette, palette_for};
use crate::rules::initial_state;
use crate::schema::{Position, map_layer_bounds};
pub(crate) use actors::spawn_emitter;
use actors::{
    build_crates, build_gems, build_goals, build_lights, build_name, build_player, build_prompt,
    build_splash, build_watchers,
};
use nightshade::prelude::*;
use nightshade::render::config::FogMode;
use terrain::build_terrain;

pub const LAYER_HEIGHT: f32 = 4.0;
pub const CRATE_Y: f32 = 0.42;
pub const CRATE_SUNK_Y: f32 = -0.44;
pub const PLAYER_Y: f32 = 0.29;
pub const GATE_CLOSED_Y: f32 = 0.5;
pub const GATE_OPEN_Y: f32 = -0.52;
pub const PROMPT_Y: f32 = 1.35;
/// How high the map's name stands off its storey.
pub const NAME_Y: f32 = 1.1;
/// How high a gem lying on the floor floats, which is a little, because it is
/// a thing on the ground rather than a thing on a stand.
pub const GEM_Y: f32 = 0.34;
/// How high a gem sits once it is seated in a socket, which is on top of the
/// plinth that is holding it.
pub const GEM_SEATED_Y: f32 = 0.72;
/// How high a gem rides in the hands of whoever is carrying it. Above the crest
/// its carrier wears, because two things over one head that pass through each
/// other read as neither.
pub const GEM_CARRIED_Y: f32 = 1.06;
/// How high a watcher stands, which is taller than a crate, because the one
/// thing on the board that kills by standing there should be the thing the eye
/// lands on first.
pub const WATCHER_Y: f32 = 0.4;

pub fn layer_height(layer: i32) -> f32 {
    layer as f32 * LAYER_HEIGHT
}

pub fn world_position(at: Position, height: f32) -> Vec3 {
    Vec3::new(
        at.cell.0 as f32,
        layer_height(at.layer) + height,
        at.cell.1 as f32,
    )
}

/// Turns one map value into a world: tiles become meshes, entities become
/// movable actors, and the skin becomes the palette and the sky. Every storey
/// is built once, and which one is on show is a visibility question rather than
/// a rebuilding one.
pub fn start_map(game: &mut SokobanResources, world: &mut World, request: MapRequest) {
    // A board arriving answers whatever was being generated, because whoever
    // asked for one is now looking at one. Without this, picking a campaign
    // level while a random board is still being rolled means the roll finishes
    // a moment later and takes the level away again. The run that hands a board
    // over has already taken its own work out of here, so it never cancels
    // itself.
    if matches!(
        game.work,
        Some(crate::systems::world::work::Work::Making(..))
    ) {
        game.work = None;
    }
    clear(game, world);

    game.origin = request.origin;
    game.map = request.map;
    // Relinking is what finds the emitters and pairs the pads. Doing it on the
    // way in means no builder has to remember to, and doing it twice is free
    // because it preserves what is already right.
    crate::schema::map_relink(&mut game.map);
    game.state = initial_state(&game.map);
    game.objectives = crate::objectives::objectives(&game.map);
    game.undo_stack.clear();
    game.solved_announced = false;
    game.solved_delay = 0.0;
    game.notice.clear();
    game.playback = Playback::default();
    // A death that was still playing out belongs to the board it happened on.
    // Carried over, it holds the controls of a board nobody died on.
    game.dying = 0.0;

    let palette = palette_for(game.map.skin);
    apply_skin(&palette, world);
    build_terrain(game, &palette, world);
    build_goals(game, &palette, world);
    build_crates(game, &palette, world);
    build_gems(game, world);
    build_watchers(game, world);
    build_player(game, &palette, world);
    build_prompt(game, world);
    build_splash(game, world);
    build_name(game, world);
    build_lights(game, &palette, world);

    focus_layer(game, game.state.player().layer);
    game.camera.settled = false;
}

/// Points the camera at a storey, framed to that storey's own footprint, and
/// records it as the one on show.
pub fn focus_layer(game: &mut SokobanResources, layer: i32) {
    game.entities.layer = layer;
    let Some((minimum, maximum)) = map_layer_bounds(&game.map, layer) else {
        // An empty storey has no footprint to frame, which happens in the
        // editor on the way to adding a floor there. Keep the framing and just
        // ride up to it.
        game.camera.focus.y = layer_height(layer);
        return;
    };
    game.camera.focus = Vec3::new(
        (minimum.0 + maximum.0) as f32 * 0.5,
        layer_height(layer),
        (minimum.1 + maximum.1) as f32 * 0.5,
    );
    game.camera.extent = Vec2::new(
        (maximum.0 - minimum.0 + 1) as f32,
        (maximum.1 - minimum.1 + 1) as f32,
    );
}

pub fn clear(game: &mut SokobanResources, world: &mut World) {
    // The light is owned by the pass that draws it, because it is laid down and
    // taken up again every time the board moves under it rather than once with
    // the map, so it is taken down here by name.
    for entity in game
        .beams
        .drain(..)
        .chain(game.pools.drain(..))
        .chain(game.reach.drain(..))
        .chain(game.footprints.drain(..))
        .chain(game.signposts.drain(..))
        .collect::<Vec<Entity>>()
    {
        if world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    game.beam_shape.clear();
    game.pool_shape.clear();
    game.reach_shape.clear();
    game.footprint_shape.clear();
    game.signpost_shape.clear();

    for entity in std::mem::take(&mut game.entities.spawned) {
        despawn_recursive_immediate(world, entity);
    }
    game.entities = MapEntities::default();
}

/// Records an entity as part of the current map so the next build tears it
/// down, and tags the storey it belongs to so the visibility pass can find it.
pub fn track_entity(game: &mut SokobanResources, world: &mut World, entity: Entity, layer: i32) {
    game.entities.spawned.push(entity);
    world.set(entity, LayerTag { layer });
}

fn solid(color: [f32; 4], roughness: f32, metallic: f32) -> Material {
    Material {
        base_color: color,
        roughness,
        metallic,
        ..Default::default()
    }
}

/// Painted or lacquered rather than raw, meaning a thin polished layer over the
/// colour underneath, which is what gives a crate or a helmet a highlight that slides
/// across it instead of a flat sheen.
fn lacquered(color: [f32; 4], roughness: f32, coat: f32) -> Material {
    Material {
        base_color: color,
        roughness,
        metallic: 0.0,
        clearcoat_factor: coat,
        clearcoat_roughness_factor: 0.12,
        specular_factor: 1.0,
        ..Default::default()
    }
}

/// Machined metal, for the parts of a board that are meant to read as built
/// rather than poured.
pub(crate) fn machined(color: [f32; 4], emissive: [f32; 3]) -> Material {
    Material {
        base_color: color,
        emissive_factor: emissive,
        roughness: 0.28,
        metallic: 0.85,
        ..Default::default()
    }
}

fn glowing(color: [f32; 4], emissive: [f32; 3], roughness: f32) -> Material {
    Material {
        base_color: color,
        emissive_factor: emissive,
        roughness,
        ..Default::default()
    }
}

/// Glass, as the material it is rather than a pale block. Nearly everything
/// that meets it goes through, what does go through bends the way glass bends
/// it rather than the way ice does, and the little it holds onto is the colour
/// of the pane. It is a window, so the surface is polished and the sheet is
/// thin enough not to stain what is behind it.
fn glazed(color: [f32; 4], tint: [f32; 3]) -> Material {
    Material {
        base_color: color,
        roughness: 0.02,
        metallic: 0.0,
        transmission_factor: 0.97,
        thickness: 0.25,
        attenuation_color: tint,
        attenuation_distance: 4.0,
        // Window glass sits at one and a half, well above ice, which is why a
        // pane bends what is behind it further than a frozen surface does.
        ior: 1.52,
        specular_factor: 1.0,
        ..Default::default()
    }
}

/// A mirror finish: no roughness worth the name and nothing but metal, so what
/// it shows is entirely what is around it.
fn chrome(color: [f32; 4]) -> Material {
    Material {
        base_color: color,
        roughness: 0.03,
        metallic: 1.0,
        specular_factor: 1.0,
        ..Default::default()
    }
}

/// Ice, as a material rather than as a pale floor tile. It refracts what is
/// under it, tints by how far light travels inside it, and carries a polished
/// coat on top, which between them are what tell an eye that a surface is
/// frozen rather than merely light coloured.
fn frozen(color: [f32; 4], tint: [f32; 3]) -> Material {
    Material {
        base_color: color,
        roughness: 0.07,
        metallic: 0.0,
        transmission_factor: 0.92,
        thickness: 0.55,
        attenuation_color: tint,
        attenuation_distance: 1.6,
        // Ice sits near 1.31, well below glass, which is why a frozen surface
        // bends what is under it far less than a window would.
        ior: 1.31,
        specular_factor: 1.0,
        clearcoat_factor: 0.7,
        clearcoat_roughness_factor: 0.05,
        ..Default::default()
    }
}

pub(crate) fn block(
    world: &mut World,
    mesh: &str,
    position: Vec3,
    scale: Vec3,
    material_name: impl Into<String>,
    material: Material,
) -> Entity {
    let entity = spawn_mesh_at(world, mesh, position, scale);
    set_material(world, entity, material_name, material);
    entity
}

pub fn apply_skin(palette: &Palette, world: &mut World) {
    let settings = world.res_mut::<RenderSettings>();
    settings.atmosphere = palette.atmosphere;
    settings.fog = Some(Fog {
        color: palette.fog,
        start: 16.0,
        end: 54.0,
        mode: FogMode::Exponential,
    });
}
