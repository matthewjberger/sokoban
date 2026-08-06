//! The lattice side of authoring: which storey is being edited, and adding,
//! removing, and resizing the floors that make up a map.

use crate::ecs::SokobanResources;
use crate::schema::{
    MAX_EXTENT, MIN_EXTENT, Map, Position, Slot, map_add_floor, map_blank, map_relink,
    map_remove_floor, map_set_tile, map_slot_for, map_tile,
};

pub fn change_layer(game: &mut SokobanResources, step: i32) {
    game.editor.cursor.layer += step;
    game.editor.slot = map_slot_for(&game.editor.map, game.editor.cursor).0;
    game.editor.needs_rebuild = true;
    game.editor.status = format!("editing storey {}", game.editor.cursor.layer);
}

pub fn add_floor(game: &mut SokobanResources) {
    let slot = map_slot_for(&game.editor.map, game.editor.cursor).0;
    map_add_floor(&mut game.editor.map, slot);
    game.editor.needs_rebuild = true;
    game.editor.status = format!(
        "added a floor at column {} row {} storey {}",
        slot.column, slot.row, slot.layer
    );
}

pub fn remove_floor(game: &mut SokobanResources) {
    let slot = map_slot_for(&game.editor.map, game.editor.cursor).0;
    if game.editor.map.floors.len() <= 1 {
        game.editor.status = "a map needs at least one floor".to_string();
        return;
    }
    map_remove_floor(&mut game.editor.map, slot);
    game.editor.needs_rebuild = true;
    game.editor.status = "removed a floor".to_string();
}

/// Floors all share one size, so a resize rebuilds the lattice at the new
/// extent and carries every square and entity across by its position within
/// its own floor. Anything that no longer fits is dropped, and nothing that
/// still fits moves.
pub fn resize(game: &mut SokobanResources, width_step: i32, height_step: i32) {
    let map = &game.editor.map;
    let width = (map.floor_width + width_step).clamp(MIN_EXTENT, MAX_EXTENT);
    let height = (map.floor_height + height_step).clamp(MIN_EXTENT, MAX_EXTENT);
    if width == map.floor_width && height == map.floor_height {
        return;
    }

    let mut rebuilt = map_blank(width, height);
    rebuilt.name = map.name.clone();
    rebuilt.hint = map.hint.clone();
    rebuilt.par = map.par;
    rebuilt.rules = map.rules;
    rebuilt.skin = map.skin;
    rebuilt.floors.clear();
    for floor in &map.floors {
        map_add_floor(&mut rebuilt, floor.slot);
    }
    for floor in &map.floors {
        for local_y in 0..map.floor_height.min(height) {
            for local_x in 0..map.floor_width.min(width) {
                // A square that was the old outer rim was a wall because the
                // floor ended there, not because anyone drew it. Growing the
                // floor moves the rim, so that wall does not come along and
                // leave a fence through the middle of the room.
                if growing_past_old_rim(map, width, height, (local_x, local_y)) {
                    continue;
                }
                let from = local_position(map, floor.slot, (local_x, local_y));
                let to = local_position(&rebuilt, floor.slot, (local_x, local_y));
                map_set_tile(&mut rebuilt, to, map_tile(map, from));
            }
        }
    }

    rebuilt.player = remap(map, &rebuilt, map.player).unwrap_or(Position::new(0, (1, 1)));
    rebuilt.crates = map
        .crates
        .iter()
        .filter_map(|at| remap(map, &rebuilt, *at))
        .collect();
    rebuilt.goals = map
        .goals
        .iter()
        .filter_map(|at| remap(map, &rebuilt, *at))
        .collect();
    map_relink(&mut rebuilt);

    let cursor = remap(map, &rebuilt, game.editor.cursor)
        .unwrap_or(Position::new(game.editor.cursor.layer, (1, 1)));
    game.editor.map = rebuilt;
    game.editor.cursor = cursor;
    game.editor.needs_rebuild = true;
    game.editor.status = format!("floors are now {width} by {height}");
}

/// Whether a square sat on the old floor's rim and is now interior, which only
/// happens when the floor grew.
fn growing_past_old_rim(map: &Map, width: i32, height: i32, local: (i32, i32)) -> bool {
    let on_old_edge = local.0 == map.floor_width - 1 || local.1 == map.floor_height - 1;
    let now_interior = local.0 < width - 1 && local.1 < height - 1;
    on_old_edge && now_interior
}

/// The global position of a square given its floor and its coordinates within
/// that floor. Resizing changes the lattice stride, so it changes this answer.
fn local_position(map: &Map, slot: Slot, local: (i32, i32)) -> Position {
    Position::new(
        slot.layer,
        (
            slot.column * map.floor_width + local.0,
            slot.row * map.floor_height + local.1,
        ),
    )
}

/// Carries a position from one lattice stride to another, keeping which floor
/// it is on and where it sits inside that floor. `None` when the square falls
/// outside the new floor size.
fn remap(from: &Map, to: &Map, at: Position) -> Option<Position> {
    let slot = map_slot_for(from, at).0;
    let local = (
        at.cell.0 - slot.column * from.floor_width,
        at.cell.1 - slot.row * from.floor_height,
    );
    if local.0 >= to.floor_width || local.1 >= to.floor_height {
        return None;
    }
    Some(local_position(to, slot, local))
}
