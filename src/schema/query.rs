//! Reading a map: which floor owns a square, what is on it, where its links
//! lead, and how far a player could get.

use crate::schema::{Abilities, Cell, Direction, Floor, Map, Position, Skin, Slot, Tile};

/// Which lattice slot owns a position, and where in that floor's tiles it
/// lands. Cells divide by the floor size, so the answer is arithmetic rather
/// than a search.
pub fn map_slot_for(map: &Map, position: Position) -> (Slot, usize) {
    let column = position.cell.0.div_euclid(map.floor_width);
    let row = position.cell.1.div_euclid(map.floor_height);
    let local_x = position.cell.0.rem_euclid(map.floor_width);
    let local_y = position.cell.1.rem_euclid(map.floor_height);
    (
        Slot {
            column,
            row,
            layer: position.layer,
        },
        (local_y * map.floor_width + local_x) as usize,
    )
}

pub fn map_floor_index(map: &Map, slot: Slot) -> Option<usize> {
    map.floors.iter().position(|floor| floor.slot == slot)
}

pub fn map_tile(map: &Map, position: Position) -> Tile {
    let (slot, index) = map_slot_for(map, position);
    match map_floor_index(map, slot) {
        Some(floor) => map.floors[floor].tiles[index],
        None => Tile::Void,
    }
}

/// What the floor under a square is made of. A floor may wear its own skin;
/// otherwise it wears the map's.
pub fn map_floor_skin(map: &Map, position: Position) -> Skin {
    let (slot, _) = map_slot_for(map, position);
    map_floor_index(map, slot)
        .and_then(|floor| map.floors[floor].skin)
        .unwrap_or(map.skin)
}

pub fn map_teleport_exit(map: &Map, position: Position) -> Option<Position> {
    map.portals.iter().find_map(|(first, second)| {
        if *first == position {
            Some(*second)
        } else if *second == position {
            Some(*first)
        } else {
            None
        }
    })
}

/// The square an elevator at `position` reaches one storey up or down. Both
/// ends have to be elevator tiles, which keeps the link visible on the board
/// instead of hidden in a side table.
pub fn map_elevator_target(map: &Map, position: Position, direction: i32) -> Option<Position> {
    if map_tile(map, position) != Tile::Elevator {
        return None;
    }
    let target = Position::new(position.layer + direction.signum(), position.cell);
    (map_tile(map, target) == Tile::Elevator).then_some(target)
}

/// Where a crate pushed onto an elevator ends up: down if there is a storey
/// below, otherwise up. One deterministic answer keeps the search honest.
pub fn map_elevator_drop(map: &Map, position: Position) -> Option<Position> {
    if !map.rules.elevators_move_crates {
        return None;
    }
    map_elevator_target(map, position, -1).or_else(|| map_elevator_target(map, position, 1))
}

pub fn map_layers(map: &Map) -> Vec<i32> {
    let mut layers: Vec<i32> = map.floors.iter().map(|floor| floor.slot.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    layers
}

pub fn map_floor_positions(map: &Map, floor: &Floor) -> Vec<Position> {
    let origin_x = floor.slot.column * map.floor_width;
    let origin_y = floor.slot.row * map.floor_height;
    (0..map.floor_height)
        .flat_map(|row| (0..map.floor_width).map(move |column| (origin_x + column, origin_y + row)))
        .map(|cell| Position::new(floor.slot.layer, cell))
        .collect()
}

pub fn map_positions(map: &Map) -> Vec<Position> {
    map.floors
        .iter()
        .flat_map(|floor| map_floor_positions(map, floor))
        .collect()
}

/// The cell rectangle a storey occupies, used to frame the camera on whatever
/// the player can currently see.
pub fn map_layer_bounds(map: &Map, layer: i32) -> Option<(Cell, Cell)> {
    let mut bounds: Option<(Cell, Cell)> = None;
    for floor in map.floors.iter().filter(|floor| floor.slot.layer == layer) {
        let minimum = (
            floor.slot.column * map.floor_width,
            floor.slot.row * map.floor_height,
        );
        let maximum = (
            minimum.0 + map.floor_width - 1,
            minimum.1 + map.floor_height - 1,
        );
        bounds = Some(match bounds {
            None => (minimum, maximum),
            Some((low, high)) => (
                (low.0.min(minimum.0), low.1.min(minimum.1)),
                (high.0.max(maximum.0), high.1.max(maximum.1)),
            ),
        });
    }
    bounds
}

/// Where a body with these powers lands stepping this way, if it can go at all.
///
/// This is the shape of the board and nothing else, with no crates, no gates
/// and no board state. Three things can happen and they are all the same question. An
/// ordinary square ahead is stepped onto. A single wall is stepped through by
/// whoever phases, landing beyond it. A stretch of open air is crossed by
/// whoever blinks, landing on the first ground within reach.
///
/// Both the rules and the board graph ask this rather than working it out
/// separately, which is what stops them disagreeing about which squares a party
/// can reach.
pub fn map_step(
    map: &Map,
    from: Position,
    direction: Direction,
    powers: Abilities,
) -> Option<Position> {
    let delta = direction.delta();
    let target = from.offset(delta);
    let standable = |at: Position| -> bool {
        let tile = map_tile(map, at);
        !tile.blocks_forever() || (powers.wades && tile == Tile::Water)
    };
    if standable(target) {
        return Some(target);
    }

    // A wall is one step rather than a stop, when there is somewhere to arrive
    // on the far side of it. Open air is not a wall, because nothing is held
    // up by it.
    if powers.phasing && map_tile(map, target) != Tile::Void {
        let beyond = target.offset(delta);
        if standable(beyond) {
            return Some(beyond);
        }
    }

    if powers.blinks && map_tile(map, target) == Tile::Void {
        let mut at = target;
        for _ in 0..map.rules.blink_reach.max(1) {
            at = at.offset(delta);
            if map_tile(map, at) != Tile::Void {
                return standable(at).then_some(at);
            }
        }
    }

    None
}
