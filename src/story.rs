//! The story. The overworld is a map like any other, built from the same schema
//! the puzzles are built from and played by the same rules, so walking around it
//! is walking around a board. Some of its squares are doors, which is the only
//! thing that makes it an overworld rather than a puzzle with nothing to solve.

use crate::maps::map_count;
use crate::schema::{
    Character, Direction, GemColor, Map, Position, Skin, Slot, Tile, map_add_floor, map_blank,
    map_relink, map_set_tile,
};

pub use crate::campaign::Area;

/// The areas, read out of the campaign file beside the boards they hold. A
/// board belongs to the area whose doors it falls among, which is a count that
/// travels with the boards rather than arithmetic over how many there are.
pub fn areas() -> &'static [Area] {
    &crate::campaign::campaign().areas
}

pub const FLOOR_WIDTH: i32 = 15;
pub const FLOOR_HEIGHT: i32 = 11;

/// Which area a puzzle's door stands in.
pub fn area_of(level: usize) -> usize {
    let mut last = 0;
    for (area, region) in areas().iter().enumerate() {
        last += region.doors;
        if level < last {
            return area;
        }
    }
    areas().len().saturating_sub(1)
}

/// The first door an area holds, which is where its doors start being counted
/// along the floor.
fn first_of_area(area: usize) -> usize {
    (0..map_count())
        .find(|level| area_of(*level) == area)
        .unwrap_or(0)
}

/// A door opens when everything it names has been finished. A door with nothing
/// to name is open from the start, and there is exactly one of those.
///
/// A level the table does not reach would be open from the start by accident,
/// so the table is checked against the campaign rather than trusted to have kept
/// up with it.
pub fn level_unlocked(level: usize, cleared: &[bool]) -> bool {
    crate::campaign::campaign()
        .levels
        .get(level)
        .map(|entry| entry.requires.as_slice())
        .unwrap_or(&[])
        .iter()
        .all(|needed| cleared.get(*needed).copied().unwrap_or(false))
}

/// An area is open once anything in it is, which is what the signs and the
/// scenes are asking about.
pub fn area_unlocked(area: usize, cleared: &[bool]) -> bool {
    (0..map_count())
        .filter(|level| area_of(*level) == area)
        .any(|level| level_unlocked(level, cleared))
}

/// How many doors stand in one row of an area before the next row starts.
const DOORS_PER_ROW: usize = 6;

/// Where in its area a door stands. They run in rows across the middle of the
/// floor with a gap between them, which leaves the edges of the room for the
/// theming.
pub fn door_position(level: usize) -> Position {
    let area = area_of(level);
    let index = level - first_of_area(area);
    let (column, row) = areas()[area].slot;
    Position::new(
        0,
        (
            column * FLOOR_WIDTH + 2 + (index % DOORS_PER_ROW) as i32 * 2,
            row * FLOOR_HEIGHT + 3 + (index / DOORS_PER_ROW) as i32 * 3,
        ),
    )
}

/// The area a square belongs to, for anything that has to say where the player
/// is standing.
pub fn area_at(at: Position) -> usize {
    let column = at.cell.0.div_euclid(FLOOR_WIDTH);
    let row = at.cell.1.div_euclid(FLOOR_HEIGHT);
    areas()
        .iter()
        .position(|area| area.slot == (column, row))
        .unwrap_or(0)
}

fn paint(map: &mut Map, tile: Tile, squares: &[Position]) {
    for square in squares {
        map_set_tile(map, *square, tile);
    }
}

/// A square in an area, given in that area's own coordinates.
fn local(area: usize, x: i32, y: i32) -> Position {
    let (column, row) = areas()[area].slot;
    Position::new(0, (column * FLOOR_WIDTH + x, row * FLOOR_HEIGHT + y))
}

/// The overworld, as one map value. Nothing about it is special to the code
/// that draws or walks it.
pub fn overworld() -> Map {
    let mut map = map_blank(FLOOR_WIDTH, FLOOR_HEIGHT);
    map.name = "THE DEPOT".to_string();
    map.hint = "Walk to a door and press ENTER".to_string();
    map.skin = Skin::Warehouse;
    map.character = Character::Pusher;

    for area in areas().iter().skip(1) {
        map_add_floor(
            &mut map,
            Slot {
                column: area.slot.0,
                row: area.slot.1,
                layer: 0,
            },
        );
    }

    // Each area is made of its own thing, so the depot reads as four places
    // rather than one floor with different furniture on it.
    for index in 0..areas().len() {
        let skin = areas()[index].skin;
        let (column, row) = areas()[index].slot;
        if let Some(floor) = map.floors.iter_mut().find(|floor| {
            floor.slot.column == column && floor.slot.row == row && floor.slot.layer == 0
        }) {
            floor.skin = Some(skin);
        }
    }

    theme_yard(&mut map);
    theme_freezer(&mut map);
    theme_quarry(&mut map);
    theme_vault(&mut map);
    theme_gantry(&mut map);
    theme_lamp_room(&mut map);
    seal_boundaries(&mut map);

    for level in 0..map_count() {
        map_set_tile(&mut map, door_position(level), Tile::Gateway(level as u8));
    }

    map.player = local(0, 6, 8);
    map_relink(&mut map);
    map
}

/// The way out of an area is one gate in an otherwise solid wall, and the gate
/// is held open by a crate standing on a plate. Getting through the depot means
/// solving the depot, not only the rooms off it.
fn seal_boundaries(map: &mut Map) {
    // Every seam in the lattice, not only the ones on the way to somewhere new.
    // A boundary left open is a boundary walked round, and the vault sits next
    // to three areas rather than one.
    boundary_wall(map, 0, 1, 0);
    boundary_wall(map, 0, 2, 1);
    boundary_wall(map, 2, 3, 2);
    boundary_wall(map, 1, 3, 3);
    boundary_wall(map, 3, 4, 4);
    boundary_wall(map, 1, 5, 5);

    // A crate stands directly above its plate, because a crate is shoved in
    // straight lines and a plate set diagonally from one can never be reached.
    map.crates = vec![
        local(0, 3, 7),
        local(0, 9, 7),
        local(2, 4, 7),
        local(1, 8, 7),
        local(3, 4, 3),
        local(1, 6, 7),
    ];
    paint(map, Tile::Plate(0), &[local(0, 3, 8)]);
    paint(map, Tile::Plate(1), &[local(0, 9, 8)]);
    paint(map, Tile::Plate(2), &[local(2, 4, 8)]);
    paint(map, Tile::Plate(3), &[local(1, 8, 8)]);
    paint(map, Tile::Plate(4), &[local(3, 4, 4)]);
    paint(map, Tile::Plate(5), &[local(1, 6, 8)]);
}

/// Walls the seam between two areas but for one square, and puts a gate in it.
fn boundary_wall(map: &mut Map, from: usize, to: usize, group: u8) {
    let (from_column, from_row) = areas()[from].slot;
    let (to_column, to_row) = areas()[to].slot;
    let horizontal = from_row == to_row;
    let squares: Vec<Position> = if horizontal {
        let x = from_column.max(to_column) * FLOOR_WIDTH;
        (0..FLOOR_HEIGHT)
            .map(|y| Position::new(0, (x, from_row * FLOOR_HEIGHT + y)))
            .collect()
    } else {
        let y = from_row.max(to_row) * FLOOR_HEIGHT;
        (0..FLOOR_WIDTH)
            .map(|x| Position::new(0, (from_column * FLOOR_WIDTH + x, y)))
            .collect()
    };

    let middle = squares.len() / 2;
    for (index, at) in squares.into_iter().enumerate() {
        if index == middle {
            map_set_tile(map, at, Tile::Gate(group));
        } else {
            map_set_tile(map, at, Tile::Wall);
        }
    }
}

/// The yard: the goods door and the sorting line. Stacks of pallets to walk
/// around, and the belt that fed the whole depot still running along the top of
/// it with nothing left to carry.
fn theme_yard(map: &mut Map) {
    paint(
        map,
        Tile::Wall,
        &[
            local(0, 2, 2),
            local(0, 3, 2),
            local(0, 2, 3),
            local(0, 9, 2),
            local(0, 10, 2),
            local(0, 10, 3),
            local(0, 2, 8),
            local(0, 10, 8),
            local(0, 6, 9),
        ],
    );
    paint(
        map,
        Tile::Conveyor(Direction::Right),
        &[
            local(0, 4, 1),
            local(0, 5, 1),
            local(0, 6, 1),
            local(0, 7, 1),
            local(0, 8, 1),
        ],
    );
}

/// The freezer: a rink from wall to wall, so crossing it is a slide and the
/// meltwater at the low end of the room has nowhere to go.
fn theme_freezer(map: &mut Map) {
    for x in 2..=11 {
        for y in 1..=3 {
            map_set_tile(map, local(1, x, y), Tile::Ice);
        }
    }
    paint(
        map,
        Tile::Ice,
        &[
            local(1, 3, 8),
            local(1, 4, 8),
            local(1, 5, 8),
            local(1, 3, 9),
            local(1, 4, 9),
        ],
    );
    paint(
        map,
        Tile::Water,
        &[local(1, 10, 9), local(1, 11, 9), local(1, 11, 8)],
    );
    paint(map, Tile::Wall, &[local(1, 1, 4), local(1, 11, 4)]);
}

/// The quarry: they dug under the floor and never filled it in. Holes, cracked
/// walls, and water standing where the digging went too deep.
fn theme_quarry(map: &mut Map) {
    paint(
        map,
        Tile::Pit,
        &[
            local(2, 3, 2),
            local(2, 4, 2),
            local(2, 3, 1),
            local(2, 9, 8),
            local(2, 10, 9),
        ],
    );
    paint(
        map,
        Tile::Water,
        &[
            local(2, 8, 1),
            local(2, 8, 2),
            local(2, 9, 2),
            local(2, 8, 3),
            local(2, 9, 3),
            local(2, 9, 1),
        ],
    );
    paint(
        map,
        Tile::Brittle,
        &[
            local(2, 6, 8),
            local(2, 7, 8),
            local(2, 2, 3),
            local(2, 5, 9),
        ],
    );
    paint(map, Tile::Fragile, &[local(2, 6, 1), local(2, 6, 2)]);
}

/// The vault: pads that move you without walking, and the strongroom in the
/// corner whose door answers to a switch across the floor rather than to
/// anybody standing at it.
fn theme_vault(map: &mut Map) {
    paint(map, Tile::Portal, &[local(3, 2, 2), local(3, 10, 2)]);
    paint(map, Tile::Switch(0), &[local(3, 3, 8)]);
    // The last door of the depot is walled in but for one gate, and the switch
    // that opens it is across the room. A gate with nothing behind it would be
    // scenery, and this one is the reason the vault is called the vault.
    paint(map, Tile::Gate(0), &[local(3, 9, 5)]);
    paint(
        map,
        Tile::Wall,
        &[
            local(3, 10, 4),
            local(3, 10, 6),
            local(3, 11, 5),
            local(3, 11, 4),
            local(3, 11, 6),
        ],
    );
    paint(
        map,
        Tile::Fragile,
        &[
            local(3, 6, 7),
            local(3, 6, 8),
            local(3, 7, 8),
            local(3, 7, 9),
        ],
    );
    paint(map, Tile::Sensor(1), &[local(3, 2, 5)]);
}

/// The lamp room: where the depot kept its light. Cases of glass along the top
/// wall with a lens still standing in one of them, an empty plinth nobody ever
/// came back for, and the burner in the floor they fed the broken ones to.
fn theme_lamp_room(map: &mut Map) {
    paint(
        map,
        Tile::Glass,
        &[
            local(5, 3, 1),
            local(5, 4, 1),
            local(5, 5, 1),
            local(5, 9, 1),
            local(5, 10, 1),
            local(5, 11, 1),
        ],
    );
    paint(map, Tile::Prism(GemColor::Jade), &[local(5, 7, 1)]);
    paint(map, Tile::Socket(Direction::Left), &[local(5, 12, 8)]);
    paint(map, Tile::Incinerator, &[local(5, 4, 8), local(5, 5, 8)]);
    paint(
        map,
        Tile::Wall,
        &[local(5, 1, 1), local(5, 13, 1), local(5, 8, 9)],
    );
}

/// The gantry: the way out. Walkways over open air with the ground gone from
/// under them, which is what the far end of a depot looks like once everything
/// under it has been carried away.
fn theme_gantry(map: &mut Map) {
    for x in 1..=11 {
        for y in 1..=9 {
            // Two runs across and one down the middle joining them. Everything
            // else has been taken up.
            let walkway = matches!(y, 2 | 5 | 8) || x == 6 || (x == 11 && y <= 5);
            if !walkway {
                map_set_tile(map, local(4, x, y), Tile::Void);
            }
        }
    }
    paint(
        map,
        Tile::Fragile,
        &[local(4, 3, 5), local(4, 9, 5), local(4, 6, 6)],
    );
    paint(map, Tile::Emitter(Direction::Down), &[local(4, 6, 0)]);
    paint(map, Tile::Receiver(1), &[local(4, 6, 9)]);
}
