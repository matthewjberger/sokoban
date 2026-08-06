//! The map editor. It edits a map value and rebuilds the world from it after
//! every change, so what the author sees while building is exactly what the
//! game renders while playing.

mod controls;
mod floors;
mod paint;

use crate::ecs::{MapOrigin, MapRequest, Screen, SokobanResources};
use crate::schema::{region_count, validate};
use crate::storage;
use crate::systems::world::build;
use crate::theme::ACCENT;
use controls::{
    handle_buttons, handle_keys, handle_pad, handle_pointer, read_rule_switches, read_text_fields,
};
use nightshade::prelude::*;
use nightshade::ui::widgets::world_state::ui_text_input_set_value;

pub fn on_enter(game: &mut SokobanResources, world: &mut World) {
    game.editor.slots = storage::list();
    game.editor.slot_index = game
        .editor
        .slot_index
        .min(game.editor.slots.len().saturating_sub(1));
    rebuild(game, world);
    revalidate(game);
}

pub fn on_exit(game: &mut SokobanResources, world: &mut World) {
    for entity in [game.editor.cursor_entity, game.editor.marker_entity] {
        if entity != Entity::default() && world.is_alive(entity) {
            despawn_recursive_immediate(world, entity);
        }
    }
    game.editor.cursor_entity = Entity::default();
    game.editor.marker_entity = Entity::default();
    game.editor.painting = false;
}

/// The editor edits a map value and rebuilds the world from it, so what the
/// author sees while building is exactly what the game renders while playing.
pub fn rebuild(game: &mut SokobanResources, world: &mut World) {
    let map = game.editor.map.clone();
    build::start_map(
        game,
        world,
        MapRequest {
            map,
            origin: MapOrigin::Authored,
        },
    );
    let layer = game.editor.cursor.layer;
    build::focus_layer(game, layer);
    spawn_cursor(game, world);
    spawn_marker(game, world);
    game.editor.needs_rebuild = false;
}

fn spawn_cursor(game: &mut SokobanResources, world: &mut World) {
    let entity = spawn_mesh_at(
        world,
        "Cube",
        build::world_position(game.editor.cursor, 0.06),
        Vec3::new(1.02, 0.06, 1.02),
    );
    set_material(
        world,
        entity,
        "sokoban_editor_cursor",
        Material {
            base_color: [ACCENT.x, ACCENT.y, ACCENT.z, 0.55],
            emissive_factor: [ACCENT.x * 0.6, ACCENT.y * 0.6, ACCENT.z * 0.6],
            alpha_mode: AlphaMode::Blend,
            roughness: 0.4,
            ..Default::default()
        },
    );
    game.entities.spawned.push(entity);
    game.editor.cursor_entity = entity;
}

/// The ring the select brush leaves behind, so the square the status line is
/// talking about is the square being looked at.
fn spawn_marker(game: &mut SokobanResources, world: &mut World) {
    let entity = spawn_mesh_at(
        world,
        "Torus",
        build::world_position(game.editor.cursor, 0.1),
        Vec3::new(0.44, 0.44, 0.44),
    );
    set_material(
        world,
        entity,
        "sokoban_editor_marker",
        Material {
            base_color: [1.0, 0.96, 0.86, 1.0],
            emissive_factor: [0.9, 0.7, 0.25],
            roughness: 0.3,
            ..Default::default()
        },
    );
    world.set(entity, Visibility { visible: false });
    game.entities.spawned.push(entity);
    game.editor.marker_entity = entity;
}

pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;

    read_text_fields(game, world);
    read_rule_switches(game, world);
    handle_buttons(game, world);
    handle_keys(game, world);
    handle_pointer(game, world);
    handle_pad(game, world);

    if game.editor.needs_rebuild {
        rebuild(game, world);
        // Painting reruns the cheap structural pass only. Searching the move
        // tree is a deliberate act, not something that happens under the brush.
        revalidate(game);
    }

    let cursor = build::world_position(game.editor.cursor, 0.06);
    let entity = game.editor.cursor_entity;
    if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
        transform.translation = cursor;
    }

    let marker = game.editor.marker_entity;
    let selected = game
        .editor
        .selected
        .filter(|at| at.layer == game.editor.cursor.layer);
    if let Some(at) = selected
        && let Some(transform) = world.get_mut::<LocalTransform>(marker)
    {
        transform.translation = build::world_position(at, 0.16);
        transform.rotation =
            nalgebra_glm::quat_angle_axis(std::f32::consts::FRAC_PI_2, &Vec3::new(1.0, 0.0, 0.0));
    }
    let showing = selected.is_some();
    if world
        .get::<Visibility>(marker)
        .is_some_and(|visibility| visibility.visible != showing)
    {
        world.set(marker, Visibility { visible: showing });
    }
}

/// Throws the map away for a generated one. The editor asks first, because the
/// board on screen may be an hour of somebody's work.
pub fn randomize(game: &mut SokobanResources, world: &mut World) {
    let Some(map) = crate::generator::generate(&game.recipe) else {
        game.editor.status = "no solvable map at the current recipe".to_string();
        return;
    };
    game.editor.map = map;
    game.editor.cursor = game.editor.map.player;
    game.editor.selected = None;
    game.editor.needs_rebuild = true;
    game.editor.status = format!("generated a new map, par {}", game.editor.map.par);
    push_text_fields(game, world);
}

/// The structural pass. Cheap enough to run on every edit, and it is what the
/// issue line under the board reports.
pub fn revalidate(game: &mut SokobanResources) -> bool {
    let issues = validate(&game.editor.map);
    if issues.is_empty() {
        let regions = region_count(&game.editor.map);
        game.editor.issues = if regions > 1 {
            format!("no structural problems, but the board is in {regions} separate pieces")
        } else {
            "no structural problems".to_string()
        };
        return true;
    }
    let listed: Vec<String> = issues
        .iter()
        .take(3)
        .map(|issue| issue.describe())
        .collect();
    let extra = issues.len().saturating_sub(listed.len());
    game.editor.issues = if extra > 0 {
        format!("{}  (+{extra} more)", listed.join("   ·   "))
    } else {
        listed.join("   ·   ")
    };
    game.editor.status = format!("{} issues", issues.len());
    false
}

/// The structural pass plus an exhaustive search. A map that survives both is
/// safe to ship, and the route the search hands back gives the move count to
/// use as par and says whether the board can be short circuited. The search
/// runs a slice a frame, so the editor keeps drawing while it works.
pub fn analyze(game: &mut SokobanResources) {
    if !revalidate(game) {
        return;
    }
    crate::systems::world::work::analyse(game);
}

pub fn test_play(game: &mut SokobanResources, world: &mut World) {
    let map = game.editor.map.clone();
    game.pending = Some(MapRequest {
        map,
        origin: MapOrigin::Authored,
    });
    next_state(world, Screen::InGame);
}

pub fn save(game: &mut SokobanResources) {
    game.editor.status = match storage::save(&game.editor.map) {
        Ok(path) => {
            game.editor.slots = storage::list();
            format!("saved {path}")
        }
        Err(message) => message,
    };
}

pub fn load(game: &mut SokobanResources, world: &mut World) {
    let Some(slot) = game.editor.slots.get(game.editor.slot_index).cloned() else {
        game.editor.status = "no saved maps".to_string();
        return;
    };
    match storage::load(&slot) {
        Ok(map) => {
            game.editor.map = map;
            game.editor.cursor = game.editor.map.player;
            game.editor.needs_rebuild = true;
            game.editor.status = format!("loaded {slot}");
            push_text_fields(game, world);
        }
        Err(message) => game.editor.status = message,
    }
}

/// Writes the map's own name and hint back into the text fields, for the paths
/// that replace the map wholesale rather than typing into it.
pub fn push_text_fields(game: &SokobanResources, world: &mut World) {
    ui_text_input_set_value(world, game.ui.editor.name_input, &game.editor.map.name);
    ui_text_input_set_value(world, game.ui.editor.hint_input, &game.editor.map.hint);
}
