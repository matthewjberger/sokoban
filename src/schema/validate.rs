//! The static pass over a map. Everything here is decidable without searching
//! the move tree, which makes it cheap enough to run on every edit in the
//! editor and on every candidate a generator produces.

use crate::schema::{
    Character, Map, Position, Tile, map_elevator_target, map_positions, map_reachable,
    map_teleport_exit, map_tile,
};
use std::collections::HashSet;

/// Everything a static pass can say about a map without playing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapIssue {
    NoFloors,
    PlayerOnSolid(Position),
    NoGoals,
    NotEnoughCrates {
        crates: usize,
        goals: usize,
    },
    CrateOnSolid(Position),
    CrateOnPit(Position),
    GoalOnSolid(Position),
    DuplicateCrate(Position),
    UnpairedPortal(Position),
    PortalPairOffPad(Position),
    LonelyElevator(Position),
    GateWithoutPlate(u8),
    PlateWithoutGate(u8),
    SwitchWithoutGate(u8),
    TowerWithoutGate(u8),
    UnreachableGoal(Position),
    UnreachableCrate(Position),
    DuplicateCharacter(Character),
    /// A socket with no gem anywhere that could ever be carried to it.
    SocketWithoutGem,
    /// More sockets than gems on a board that has to fill all of them.
    NotEnoughGems {
        gems: usize,
        sockets: usize,
    },
    /// A board won by seating gems with nowhere to seat one.
    NoSockets,
    GemOnSolid(Position),
    DuplicateGem(Position),
    UnreachableGem(Position),
    /// A gem lying somewhere no gem can be put down again, which is a gem the
    /// board loses the moment it is picked up.
    GemOnBadGround(Position),
    /// A boulder on a board where nobody can break one, which is a wall the
    /// author probably did not mean to build.
    UnbreakableStone(Position),
    SpikesWithoutTrigger(u8),
    ShutterWithoutTrigger(u8),
    /// A lock with no gem of its colour anywhere, which is a wall with a
    /// keyhole nobody was given a key for.
    LockWithoutKey,
    WatcherOnSolid(Position),
    /// Somebody starts within reach of a watcher, so the board is over before
    /// the first move.
    WatchedAtTheStart(Position),
}

impl MapIssue {
    pub fn describe(&self) -> String {
        match self {
            Self::NoFloors => "the map has no floors".to_string(),
            Self::PlayerOnSolid(at) => format!("player stands on a wall at {}", show(*at)),
            Self::NoGoals => "no goals to fill".to_string(),
            Self::NotEnoughCrates { crates, goals } => {
                format!("{crates} crates cannot fill {goals} goals")
            }
            Self::CrateOnSolid(at) => format!("crate inside a wall at {}", show(*at)),
            Self::CrateOnPit(at) => format!("crate starts in a pit at {}", show(*at)),
            Self::GoalOnSolid(at) => format!("goal inside a wall at {}", show(*at)),
            Self::DuplicateCrate(at) => format!("two crates share {}", show(*at)),
            Self::UnpairedPortal(at) => format!("portal at {} has no twin", show(*at)),
            Self::PortalPairOffPad(at) => {
                format!("portal link points at {}, which is not a pad", show(*at))
            }
            Self::LonelyElevator(at) => {
                format!("elevator at {} reaches no other storey", show(*at))
            }
            Self::GateWithoutPlate(group) => {
                format!("gate {} has no plate or switch", group + 1)
            }
            Self::PlateWithoutGate(group) => format!("plate {} opens nothing", group + 1),
            Self::SwitchWithoutGate(group) => format!("switch {} opens nothing", group + 1),
            Self::TowerWithoutGate(group) => format!("tower {} powers nothing", group + 1),
            Self::UnreachableGoal(at) => format!("goal at {} is walled off", show(*at)),
            Self::UnreachableCrate(at) => format!("crate at {} is walled off", show(*at)),
            Self::DuplicateCharacter(character) => {
                format!("two of the party are {}", character.label())
            }
            Self::SocketWithoutGem => "sockets with no gem to seat in them".to_string(),
            Self::NotEnoughGems { gems, sockets } => {
                format!("{gems} gems cannot fill {sockets} sockets")
            }
            Self::NoSockets => "no sockets to seat a gem in".to_string(),
            Self::GemOnSolid(at) => format!("gem inside a wall at {}", show(*at)),
            Self::DuplicateGem(at) => format!("two gems share {}", show(*at)),
            Self::UnreachableGem(at) => format!("gem at {} is walled off", show(*at)),
            Self::GemOnBadGround(at) => {
                format!(
                    "gem at {} is on ground it could never be put back on",
                    show(*at)
                )
            }
            Self::UnbreakableStone(at) => {
                format!("boulder at {} and nobody who can break one", show(*at))
            }
            Self::SpikesWithoutTrigger(group) => {
                format!("spikes {} answer to nothing", group + 1)
            }
            Self::ShutterWithoutTrigger(group) => {
                format!("shutter {} answers to nothing", group + 1)
            }
            Self::LockWithoutKey => "a lock with no gem of its colour".to_string(),
            Self::WatcherOnSolid(at) => format!("watcher inside a wall at {}", show(*at)),
            Self::WatchedAtTheStart(at) => {
                format!("somebody starts beside a watcher at {}", show(*at))
            }
        }
    }
}

fn show(position: Position) -> String {
    format!(
        "layer {} cell {},{}",
        position.layer, position.cell.0, position.cell.1
    )
}

/// The static pass. Everything here is decidable without searching the move
/// tree, which makes it cheap enough to run on every edit in the editor and on
/// every candidate a generator produces.
pub fn validate(map: &Map) -> Vec<MapIssue> {
    let mut issues = Vec::new();
    if map.floors.is_empty() || map.floor_width <= 0 || map.floor_height <= 0 {
        issues.push(MapIssue::NoFloors);
        return issues;
    }

    if map_tile(map, map.player).blocks_walking() {
        issues.push(MapIssue::PlayerOnSolid(map.player));
    }
    // What a board owes depends on what it is asking for. One won by seating
    // gems owes nothing to its markers and everything to its sockets.
    if map.rules.win.wants_crates() {
        if map.goals.is_empty() {
            issues.push(MapIssue::NoGoals);
        }
        let pushables = map.crates.len() + map.orbs.len();
        if pushables < map.goals.len() {
            issues.push(MapIssue::NotEnoughCrates {
                crates: pushables,
                goals: map.goals.len(),
            });
        }
    } else {
        if map.sockets.is_empty() {
            issues.push(MapIssue::NoSockets);
        }
        if map.gems.len() < map.sockets.len() {
            issues.push(MapIssue::NotEnoughGems {
                gems: map.gems.len(),
                sockets: map.sockets.len(),
            });
        }
    }
    if !map.sockets.is_empty() && map.gems.is_empty() {
        issues.push(MapIssue::SocketWithoutGem);
    }
    if !map.stones.is_empty() && !map.latent_abilities().smashes {
        for at in &map.stones {
            issues.push(MapIssue::UnbreakableStone(*at));
        }
    }

    // One of each, ever. A class is what a member is, because its abilities, its
    // colour and the key that selects it all come from it, so two members of one class
    // are two of the same person and there is no telling them apart on screen.
    let mut party = Vec::new();
    for index in 0..map.party_size() {
        let character = map.member_character(index);
        if party.contains(&character) {
            issues.push(MapIssue::DuplicateCharacter(character));
        } else {
            party.push(character);
        }
    }

    for (index, position) in map.crates.iter().enumerate() {
        if map_tile(map, *position).blocks_walking() {
            issues.push(MapIssue::CrateOnSolid(*position));
        }
        if map_tile(map, *position) == Tile::Pit {
            issues.push(MapIssue::CrateOnPit(*position));
        }
        if map.crates[..index].contains(position) {
            issues.push(MapIssue::DuplicateCrate(*position));
        }
    }
    for position in &map.goals {
        if map_tile(map, *position).blocks_walking() {
            issues.push(MapIssue::GoalOnSolid(*position));
        }
    }
    for (index, gem) in map.gems.iter().enumerate() {
        if map_tile(map, gem.at).blocks_walking() {
            issues.push(MapIssue::GemOnSolid(gem.at));
        }
        if matches!(
            map_tile(map, gem.at),
            Tile::Water | Tile::Incinerator | Tile::Pit
        ) {
            issues.push(MapIssue::GemOnBadGround(gem.at));
        }
        if map.gems[..index].iter().any(|other| other.at == gem.at) {
            issues.push(MapIssue::DuplicateGem(gem.at));
        }
    }

    for position in map_positions(map) {
        match map_tile(map, position) {
            Tile::Portal if map_teleport_exit(map, position).is_none() => {
                issues.push(MapIssue::UnpairedPortal(position));
            }
            Tile::Elevator
                if map_elevator_target(map, position, 1).is_none()
                    && map_elevator_target(map, position, -1).is_none() =>
            {
                issues.push(MapIssue::LonelyElevator(position));
            }
            _ => {}
        }
    }
    for (first, second) in &map.portals {
        for endpoint in [first, second] {
            if map_tile(map, *endpoint) != Tile::Portal {
                issues.push(MapIssue::PortalPairOffPad(*endpoint));
            }
        }
    }

    let mut plates = HashSet::new();
    let mut gates = HashSet::new();
    let mut switches = HashSet::new();
    let mut towers = HashSet::new();
    let mut spikes = HashSet::new();
    let mut shutters = HashSet::new();
    for floor in &map.floors {
        for tile in &floor.tiles {
            match tile {
                Tile::Plate(group) => {
                    plates.insert(*group);
                }
                Tile::Gate(group) => {
                    gates.insert(*group);
                }
                Tile::Switch(group) => {
                    switches.insert(*group);
                }
                Tile::Receiver(group) => {
                    towers.insert(*group);
                }
                Tile::Sensor(group) => {
                    towers.insert(*group);
                }
                Tile::Spike(group) => {
                    spikes.insert(*group);
                }
                Tile::Shutter(group) => {
                    shutters.insert(*group);
                }
                _ => {}
            }
        }
    }
    // Any kind of trigger answers for a gate, so a gate is only orphaned when
    // none of them names its group.
    let openers: HashSet<u8> = plates
        .union(&switches)
        .copied()
        .chain(towers.iter().copied())
        .collect();
    let mut orphan_gates: Vec<u8> = gates.difference(&openers).copied().collect();
    orphan_gates.sort_unstable();
    for group in orphan_gates {
        issues.push(MapIssue::GateWithoutPlate(group));
    }
    // A door and a bed of spikes are the same wiring answered two ways, so
    // either of them is something for a trigger to be for.
    let driven: HashSet<u8> = gates
        .union(&spikes)
        .chain(shutters.iter())
        .copied()
        .collect();
    let mut orphan_plates: Vec<u8> = plates.difference(&driven).copied().collect();
    orphan_plates.sort_unstable();
    for group in orphan_plates {
        issues.push(MapIssue::PlateWithoutGate(group));
    }
    let mut orphan_switches: Vec<u8> = switches.difference(&driven).copied().collect();
    orphan_switches.sort_unstable();
    for group in orphan_switches {
        issues.push(MapIssue::SwitchWithoutGate(group));
    }
    let mut orphan_towers: Vec<u8> = towers.difference(&driven).copied().collect();
    orphan_towers.sort_unstable();
    for group in orphan_towers {
        issues.push(MapIssue::TowerWithoutGate(group));
    }
    // Spikes and shutters are wired the way a door is, so one that nothing
    // names never moves and might as well have been painted on.
    let mut idle_spikes: Vec<u8> = spikes.difference(&openers).copied().collect();
    idle_spikes.sort_unstable();
    for group in idle_spikes {
        issues.push(MapIssue::SpikesWithoutTrigger(group));
    }
    let mut idle_shutters: Vec<u8> = shutters.difference(&openers).copied().collect();
    idle_shutters.sort_unstable();
    for group in idle_shutters {
        issues.push(MapIssue::ShutterWithoutTrigger(group));
    }

    for at in &map.watchers {
        if map_tile(map, *at).blocks_walking() {
            issues.push(MapIssue::WatcherOnSolid(*at));
        }
    }
    for index in 0..map.party_size() {
        let start = map.member_start(index);
        if map.watchers.iter().any(|watcher| {
            watcher.layer == start.layer
                && (watcher.cell.0 - start.cell.0).abs() + (watcher.cell.1 - start.cell.1).abs()
                    == 1
        }) {
            issues.push(MapIssue::WatchedAtTheStart(start));
        }
    }
    for floor in &map.floors {
        for tile in &floor.tiles {
            if let Tile::Lock(colour) = tile
                && !map.gems.iter().any(|gem| gem.color == *colour)
            {
                issues.push(MapIssue::LockWithoutKey);
                break;
            }
        }
    }

    let reachable = map_reachable(map);
    for position in &map.goals {
        if !reachable.contains(position) {
            issues.push(MapIssue::UnreachableGoal(*position));
        }
    }
    for position in &map.crates {
        if !reachable.contains(position) {
            issues.push(MapIssue::UnreachableCrate(*position));
        }
    }
    for gem in &map.gems {
        if !reachable.contains(&gem.at) {
            issues.push(MapIssue::UnreachableGem(gem.at));
        }
    }

    issues
}
