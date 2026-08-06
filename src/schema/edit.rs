//! Changing a map: painting squares, adding and removing floors from the
//! lattice, and keeping the value consistent afterwards.

use crate::schema::{
    Floor, MAX_EXTENT, MIN_EXTENT, Map, Position, Slot, Tile, map_floor_index, map_positions,
    map_slot_for, map_tile,
};
use std::collections::HashSet;

pub fn map_set_tile(map: &mut Map, position: Position, tile: Tile) {
    let (slot, index) = map_slot_for(map, position);
    if let Some(floor) = map_floor_index(map, slot) {
        map.floors[floor].tiles[index] = tile;
    }
}

/// A walled room on one lattice slot: the starting point for the editor and
/// for a generator.
pub fn map_blank(width: i32, height: i32) -> Map {
    let width = width.clamp(MIN_EXTENT, MAX_EXTENT);
    let height = height.clamp(MIN_EXTENT, MAX_EXTENT);
    let mut map = Map {
        name: "Untitled".to_string(),
        floor_width: width,
        floor_height: height,
        player: Position::new(0, (1, 1)),
        ..Default::default()
    };
    map_add_floor(&mut map, Slot::default());
    map
}

/// Adds a floor to a lattice slot, walled on the edges that have no neighbour
/// so a new floor is playable the moment it appears.
pub fn map_add_floor(map: &mut Map, slot: Slot) {
    if map_floor_index(map, slot).is_some() {
        return;
    }
    map.floors.push(Floor {
        slot,
        tiles: vec![Tile::Floor; (map.floor_width * map.floor_height) as usize],
        skin: None,
    });
    seal_borders(map, slot);
    for neighbour in neighbour_slots(slot) {
        if map_floor_index(map, neighbour).is_some() {
            seal_borders(map, neighbour);
        }
    }
}

pub fn map_remove_floor(map: &mut Map, slot: Slot) {
    let Some(index) = map_floor_index(map, slot) else {
        return;
    };
    map.floors.remove(index);
    for neighbour in neighbour_slots(slot) {
        if map_floor_index(map, neighbour).is_some() {
            seal_borders(map, neighbour);
        }
    }
    map_relink(map);
}

fn neighbour_slots(slot: Slot) -> [Slot; 4] {
    [
        Slot {
            column: slot.column + 1,
            ..slot
        },
        Slot {
            column: slot.column - 1,
            ..slot
        },
        Slot {
            row: slot.row + 1,
            ..slot
        },
        Slot {
            row: slot.row - 1,
            ..slot
        },
    ]
}

/// Walls the rim of a floor where it faces empty lattice, and opens the seam
/// where it faces a neighbour, which is what lets a player walk from one floor
/// straight onto the next.
fn seal_borders(map: &mut Map, slot: Slot) {
    let Some(index) = map_floor_index(map, slot) else {
        return;
    };
    let has_left = map_floor_index(
        map,
        Slot {
            column: slot.column - 1,
            ..slot
        },
    )
    .is_some();
    let has_right = map_floor_index(
        map,
        Slot {
            column: slot.column + 1,
            ..slot
        },
    )
    .is_some();
    let has_up = map_floor_index(
        map,
        Slot {
            row: slot.row - 1,
            ..slot
        },
    )
    .is_some();
    let has_down = map_floor_index(
        map,
        Slot {
            row: slot.row + 1,
            ..slot
        },
    )
    .is_some();

    for local_y in 0..map.floor_height {
        for local_x in 0..map.floor_width {
            let sides = [
                (local_x == 0, has_left),
                (local_x == map.floor_width - 1, has_right),
                (local_y == 0, has_up),
                (local_y == map.floor_height - 1, has_down),
            ];
            let rims: Vec<bool> = sides
                .iter()
                .filter(|(on_rim, _)| *on_rim)
                .map(|(_, joined)| *joined)
                .collect();
            if rims.is_empty() {
                continue;
            }
            let tile_index = (local_y * map.floor_width + local_x) as usize;
            if rims.iter().all(|joined| *joined) {
                // Every rim this square sits on faces a neighbour, so it is a
                // seam rather than an edge, so open it back up and the two
                // floors read as one storey to walk across.
                if map.floors[index].tiles[tile_index] == Tile::Wall {
                    map.floors[index].tiles[tile_index] = Tile::Floor;
                }
            } else {
                map.floors[index].tiles[tile_index] = Tile::Wall;
            }
        }
    }
}

/// Drops every entity that no longer sits on a walkable square and pairs the
/// portal pads in scan order, so a map in hand is always self consistent.
pub fn map_relink(map: &mut Map) {
    let solid: HashSet<Position> = map_positions(map)
        .into_iter()
        .filter(|position| map_tile(map, *position).blocks_walking())
        .collect();
    let known: HashSet<Position> = map_positions(map).into_iter().collect();
    map.crates
        .retain(|position| known.contains(position) && !solid.contains(position));
    map.orbs
        .retain(|position| known.contains(position) && !solid.contains(position));
    map.lamps
        .retain(|position| known.contains(position) && !solid.contains(position));
    map.stones
        .retain(|position| known.contains(position) && !solid.contains(position));
    map.gems
        .retain(|gem| known.contains(&gem.at) && !solid.contains(&gem.at));
    map.watchers
        .retain(|position| known.contains(position) && !solid.contains(position));
    map.mirrors
        .retain(|(position, _)| known.contains(position) && !solid.contains(position));

    // Where the light comes from, found once here so the search never has to
    // hunt the board for it.
    map.emitters = map_positions(map)
        .into_iter()
        .filter_map(|at| match map_tile(map, at) {
            Tile::Emitter(way) => Some((at, way)),
            _ => None,
        })
        .collect();
    map.sockets = map_positions(map)
        .into_iter()
        .filter_map(|at| match map_tile(map, at) {
            Tile::Socket(way) => Some((at, way)),
            _ => None,
        })
        .collect();
    map.spikes = map_positions(map)
        .into_iter()
        .filter_map(|at| match map_tile(map, at) {
            Tile::Spike(group) => Some((at, group)),
            _ => None,
        })
        .collect();
    map.prisms = map_positions(map)
        .into_iter()
        .filter(|at| matches!(map_tile(map, *at), Tile::Prism(_)))
        .collect();
    map.goals
        .retain(|position| known.contains(position) && !solid.contains(position));

    // A pairing the author already made outlives an edit as long as both ends
    // are still pads, and only the leftovers get paired up in scan order.
    let pads: HashSet<Position> = map_positions(map)
        .into_iter()
        .filter(|position| map_tile(map, *position) == Tile::Portal)
        .collect();
    let mut linked: HashSet<Position> = HashSet::new();
    let mut portals: Vec<(Position, Position)> = Vec::new();
    for (first, second) in &map.portals {
        if pads.contains(first)
            && pads.contains(second)
            && linked.insert(*first)
            && linked.insert(*second)
        {
            portals.push((*first, *second));
        }
    }

    let mut loose: Vec<Position> = pads
        .into_iter()
        .filter(|position| !linked.contains(position))
        .collect();
    loose.sort_by_key(|position| (position.layer, position.cell.1, position.cell.0));
    portals.extend(
        loose
            .chunks(2)
            .filter(|pair| pair.len() == 2)
            .map(|pair| (pair[0], pair[1])),
    );
    map.portals = portals;
}
