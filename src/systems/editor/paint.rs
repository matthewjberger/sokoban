//! One edit. The brush decides which part of the map value changes, then the
//! map is relinked so it stays consistent and the world is rebuilt from it.

use crate::ecs::{Brush, SokobanResources};
use crate::schema::{
    Direction, Gem, GemColor, Map, Position, Slant, Tile, map_floor_index, map_relink,
    map_set_tile, map_slot_for, map_teleport_exit, map_tile,
};

/// One edit: the brush decides which part of the map value changes, then the
/// map is relinked so it stays consistent and the world is rebuilt from it.
pub fn paint(game: &mut SokobanResources, at: Position, brush: Brush) {
    let group = game.editor.group;
    if map_floor_index(&game.editor.map, map_slot_for(&game.editor.map, at).0).is_none() {
        game.editor.status = "no floor here: add one first".to_string();
        return;
    }
    let map = &mut game.editor.map;
    let under_player = map.player == at;
    let mut turn = None;
    let mut game_status = None;

    // Selecting reads the board rather than writing to it, so it answers before
    // anything below can decide the map needs rebuilding.
    if brush == Brush::Select {
        game.editor.selected = Some(at);
        game.editor.status = describe_square(&game.editor.map, at);
        return;
    }

    match brush {
        Brush::Wall if under_player => return,
        Brush::Wall => map_set_tile(map, at, Tile::Wall),
        Brush::Floor => map_set_tile(map, at, Tile::Floor),
        Brush::Ice => map_set_tile(map, at, Tile::Ice),
        Brush::Pit => {
            map_set_tile(map, at, Tile::Pit);
            map.crates.retain(|crate_at| *crate_at != at);
        }
        Brush::Plate => map_set_tile(map, at, Tile::Plate(group)),
        Brush::Gate => map_set_tile(map, at, Tile::Gate(group)),
        Brush::Switch => map_set_tile(map, at, Tile::Switch(group)),
        Brush::Tower => map_set_tile(map, at, Tile::Receiver(group)),
        Brush::Sensor => map_set_tile(map, at, Tile::Sensor(group)),
        Brush::Lamp => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            if let Some(index) = map.lamps.iter().position(|lamp| *lamp == at) {
                map.lamps.remove(index);
            } else {
                map.lamps.push(at);
            }
        }
        // Painting the same square again turns the source, the same way an
        // arrow turns.
        Brush::Emitter => {
            let way = match map_tile(map, at) {
                Tile::Emitter(way) => way.next(),
                _ => Direction::Up,
            };
            map_set_tile(map, at, Tile::Emitter(way));
            turn = Some(way);
        }
        // A mirror only has two ways to lean, so painting it over flips it.
        Brush::Mirror => {
            let slant = match map_tile(map, at) {
                Tile::Mirror(slant) => slant.other(),
                _ => Slant::Forward,
            };
            map_set_tile(map, at, Tile::Mirror(slant));
            game_status = Some(format!("MIRROR leans {}", slant.label()));
        }
        Brush::Orb => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            map.crates.retain(|crate_at| *crate_at != at);
            if let Some(index) = map.orbs.iter().position(|orb| *orb == at) {
                map.orbs.remove(index);
            } else {
                map.orbs.push(at);
            }
        }
        Brush::Fragile => map_set_tile(map, at, Tile::Fragile),
        // Painting the same square again turns the plinth, the same way an
        // arrow turns, because which way it will throw its light is the only
        // thing about a socket there is to choose.
        Brush::Socket => {
            let way = match map_tile(map, at) {
                Tile::Socket(way) => way.next(),
                _ => Direction::Up,
            };
            map_set_tile(map, at, Tile::Socket(way));
            turn = Some(way);
        }
        Brush::Glass if under_player => return,
        Brush::Glass => {
            map_set_tile(map, at, Tile::Glass);
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.goals.retain(|goal| *goal != at);
        }
        Brush::Prism if under_player => return,
        // A lens has four colours to be, so painting it over stains it the next
        // one along.
        Brush::Prism => {
            let color = match map_tile(map, at) {
                Tile::Prism(color) => color.next(),
                _ => GemColor::default(),
            };
            map_set_tile(map, at, Tile::Prism(color));
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.goals.retain(|goal| *goal != at);
            game_status = Some(format!("PRISM stains {}", color.label()));
        }
        Brush::Splitter if under_player => return,
        Brush::Splitter => {
            map_set_tile(map, at, Tile::Splitter);
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.goals.retain(|goal| *goal != at);
        }
        Brush::Spike => map_set_tile(map, at, Tile::Spike(group)),
        Brush::Shutter => map_set_tile(map, at, Tile::Shutter(group)),
        // A lock has four colours to be, so painting it over turns the keyhole
        // to the next one along.
        Brush::Lock => {
            let colour = match map_tile(map, at) {
                Tile::Lock(colour) => colour.next(),
                _ => GemColor::default(),
            };
            map_set_tile(map, at, Tile::Lock(colour));
            game_status = Some(format!("LOCK takes {}", colour.label()));
        }
        Brush::Watcher => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.mirrors.retain(|(mirror, _)| *mirror != at);
            if let Some(index) = map.watchers.iter().position(|watcher| *watcher == at) {
                map.watchers.remove(index);
            } else {
                map.watchers.push(at);
            }
        }
        // Painting a pallet over itself flips the mirror standing on it, and
        // painting it over a pallet that has already been flipped takes it away.
        Brush::PalletMirror => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.watchers.retain(|watcher| *watcher != at);
            match map.mirrors.iter().position(|(mirror, _)| *mirror == at) {
                Some(index) if map.mirrors[index].1 == Slant::Back => {
                    map.mirrors.remove(index);
                }
                Some(index) => {
                    map.mirrors[index].1 = Slant::Back;
                    game_status = Some("PALLET MIRROR leans BACK".to_string());
                }
                None => map.mirrors.push((at, Slant::Forward)),
            }
        }
        Brush::Incinerator => {
            map_set_tile(map, at, Tile::Incinerator);
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
        }
        // Painting a gem over a gem recolours it, and painting it somewhere it
        // already is with every colour spent takes it away again.
        Brush::Gem => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            match map.gems.iter().position(|gem| gem.at == at) {
                Some(index) if map.gems[index].color == GemColor::Azure => {
                    map.gems.remove(index);
                }
                Some(index) => {
                    map.gems[index].color = map.gems[index].color.next();
                    game_status = Some(format!("GEM is {}", map.gems[index].color.label()));
                }
                None => map.gems.push(Gem {
                    at,
                    color: GemColor::default(),
                }),
            }
        }
        Brush::Stone => {
            if map_tile(map, at).blocks_walking() || map_tile(map, at) == Tile::Pit {
                map_set_tile(map, at, Tile::Floor);
            }
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            if let Some(index) = map.stones.iter().position(|stone| *stone == at) {
                map.stones.remove(index);
            } else {
                map.stones.push(at);
            }
        }
        Brush::Select => return,
        Brush::Water if under_player => return,
        Brush::Water => {
            map_set_tile(map, at, Tile::Water);
            map.crates.retain(|crate_at| *crate_at != at);
            map.goals.retain(|goal| *goal != at);
        }
        Brush::Brittle if under_player => return,
        Brush::Brittle => {
            map_set_tile(map, at, Tile::Brittle);
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.goals.retain(|goal| *goal != at);
        }
        // Painting the same square again turns the arrow, which is how a
        // direction gets chosen without a second control to choose it with.
        Brush::OneWay => {
            let way = turned(map_tile(map, at), Brush::OneWay);
            map_set_tile(map, at, Tile::OneWay(way));
            turn = Some(way);
        }
        Brush::Conveyor => {
            let way = turned(map_tile(map, at), Brush::Conveyor);
            map_set_tile(map, at, Tile::Conveyor(way));
            turn = Some(way);
        }
        Brush::Portal => {
            if map_tile(map, at) == Tile::Portal {
                map_set_tile(map, at, Tile::Floor);
            } else {
                map_set_tile(map, at, Tile::Portal);
            }
        }
        Brush::Elevator => {
            if map_tile(map, at) == Tile::Elevator {
                map_set_tile(map, at, Tile::Floor);
            } else {
                map_set_tile(map, at, Tile::Elevator);
            }
        }
        Brush::Goal => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            if let Some(index) = map.goals.iter().position(|goal| *goal == at) {
                map.goals.remove(index);
            } else {
                map.goals.push(at);
            }
        }
        Brush::Crate => {
            if map_tile(map, at).blocks_walking() || map_tile(map, at) == Tile::Pit {
                map_set_tile(map, at, Tile::Floor);
            }
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            if let Some(index) = map.crates.iter().position(|crate_at| *crate_at == at) {
                map.crates.remove(index);
            } else {
                map.crates.push(at);
            }
        }
        Brush::Player => {
            if map_tile(map, at).blocks_walking() {
                map_set_tile(map, at, Tile::Floor);
            }
            map.player = at;
        }
        Brush::Erase if under_player => return,
        Brush::Erase => {
            map_set_tile(map, at, Tile::Void);
            map.crates.retain(|crate_at| *crate_at != at);
            map.orbs.retain(|orb| *orb != at);
            map.lamps.retain(|lamp| *lamp != at);
            map.stones.retain(|stone| *stone != at);
            map.mirrors.retain(|(mirror, _)| *mirror != at);
            map.watchers.retain(|watcher| *watcher != at);
            map.gems.retain(|gem| gem.at != at);
            map.goals.retain(|goal| *goal != at);
        }
    }

    map_relink(map);
    if let Some(way) = turn {
        game.editor.status = format!("{} now points {}", brush.label(), way.label());
    }
    if let Some(message) = game_status {
        game.editor.status = message;
    }
    game.editor.needs_rebuild = true;
}

/// Everything on one square, said in a line. The tile, what is standing on it,
/// and where it leads if it leads anywhere.
fn describe_square(map: &Map, at: Position) -> String {
    let tile = map_tile(map, at);
    let mut parts = vec![format!(
        "{},{} on storey {}",
        at.cell.0, at.cell.1, at.layer
    )];
    parts.push(match tile {
        Tile::Plate(group)
        | Tile::Gate(group)
        | Tile::Switch(group)
        | Tile::Spike(group)
        | Tile::Shutter(group) => {
            format!("{} {}", tile.label(), group + 1)
        }
        Tile::OneWay(way) | Tile::Conveyor(way) | Tile::Socket(way) => {
            format!("{} {}", tile.label(), way.label())
        }
        Tile::Prism(color) | Tile::Lock(color) => format!("{} {}", tile.label(), color.label()),
        _ => tile.label().to_string(),
    });
    if map.player == at {
        parts.push("player".to_string());
    }
    if map.crates.contains(&at) {
        parts.push("crate".to_string());
    }
    if map.stones.contains(&at) {
        parts.push("boulder".to_string());
    }
    if map.watchers.contains(&at) {
        parts.push("watcher".to_string());
    }
    if let Some((_, slant)) = map.mirrors.iter().find(|(mirror, _)| *mirror == at) {
        parts.push(format!(
            "pallet mirror leaning {}",
            slant.label().to_lowercase()
        ));
    }
    if let Some(gem) = map.gems.iter().find(|gem| gem.at == at) {
        parts.push(format!("{} gem", gem.color.label().to_lowercase()));
    }
    if map.goals.contains(&at) {
        parts.push("goal".to_string());
    }
    if let Some(exit) = map_teleport_exit(map, at) {
        parts.push(format!(
            "links to {},{} on storey {}",
            exit.cell.0, exit.cell.1, exit.layer
        ));
    }
    parts.join("   ·   ")
}

/// The way an arrow or a belt should point after being painted over. One turn
/// on from whatever was already there, or up when the square was something
/// else entirely.
fn turned(current: Tile, brush: Brush) -> Direction {
    let same_kind = matches!(
        (current, brush),
        (Tile::OneWay(_), Brush::OneWay) | (Tile::Conveyor(_), Brush::Conveyor)
    );
    match current.heading() {
        Some(way) if same_kind => way.next(),
        _ => Direction::Up,
    }
}
