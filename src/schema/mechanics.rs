//! What a board is made of, read off the board. A map's mechanics are not
//! listed anywhere. They are implied by the tiles it paints, the rules it turns
//! on, and who is walking it. Reading them back is what lets the campaign be
//! checked for teaching one thing at a time and for ever combining them, rather
//! than that being a claim in a comment somewhere.

use crate::schema::{Abilities, Character, Map, Tile};

/// One thing a board can be about.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mechanic {
    Push,
    Pairs,
    Drag,
    Magnet,
    Swap,
    Phase,
    Wade,
    Ward,
    Blink,
    Party,
    Ice,
    Pit,
    Water,
    Fragile,
    Brittle,
    OneWay,
    Conveyor,
    Portal,
    Elevator,
    Plate,
    Gate,
    Switch,
    Beam,
    Mirror,
    Receiver,
    Sensor,
    Lamp,
    Orb,
    Floors,
    Storeys,
    Break,
    Gem,
    Socket,
    Glass,
    Prism,
    Splitter,
    Spike,
    Incinerator,
    Stone,
    Watcher,
    Shutter,
    Lock,
}

impl Mechanic {
    /// Every mechanic the schema can express. A campaign is checked against this
    /// so a rule that ships without a board to teach it gets noticed.
    pub const ALL: [Self; 42] = [
        Self::Push,
        Self::Pairs,
        Self::Drag,
        Self::Magnet,
        Self::Swap,
        Self::Phase,
        Self::Wade,
        Self::Ward,
        Self::Blink,
        Self::Party,
        Self::Ice,
        Self::Pit,
        Self::Water,
        Self::Fragile,
        Self::Brittle,
        Self::OneWay,
        Self::Conveyor,
        Self::Portal,
        Self::Elevator,
        Self::Plate,
        Self::Gate,
        Self::Switch,
        Self::Beam,
        Self::Mirror,
        Self::Receiver,
        Self::Sensor,
        Self::Lamp,
        Self::Orb,
        Self::Floors,
        Self::Storeys,
        Self::Break,
        Self::Gem,
        Self::Socket,
        Self::Glass,
        Self::Prism,
        Self::Splitter,
        Self::Spike,
        Self::Incinerator,
        Self::Stone,
        Self::Watcher,
        Self::Shutter,
        Self::Lock,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Pairs => "pairs",
            Self::Drag => "drag",
            Self::Magnet => "magnet",
            Self::Swap => "swap",
            Self::Phase => "phase",
            Self::Wade => "wade",
            Self::Ward => "ward",
            Self::Blink => "blink",
            Self::Party => "party",
            Self::Ice => "ice",
            Self::Pit => "pit",
            Self::Water => "water",
            Self::Fragile => "fragile",
            Self::Brittle => "brittle",
            Self::OneWay => "one way",
            Self::Conveyor => "belt",
            Self::Portal => "portal",
            Self::Elevator => "lift",
            Self::Plate => "plate",
            Self::Gate => "gate",
            Self::Switch => "switch",
            Self::Beam => "beam",
            Self::Mirror => "mirror",
            Self::Receiver => "tower",
            Self::Sensor => "sensor",
            Self::Lamp => "lamp",
            Self::Orb => "orb",
            Self::Floors => "floors",
            Self::Storeys => "storeys",
            Self::Break => "break",
            Self::Gem => "gem",
            Self::Socket => "socket",
            Self::Glass => "glass",
            Self::Prism => "prism",
            Self::Splitter => "splitter",
            Self::Spike => "spikes",
            Self::Incinerator => "burner",
            Self::Stone => "boulder",
            Self::Watcher => "watcher",
            Self::Shutter => "shutter",
            Self::Lock => "lock",
        }
    }
}

/// Adds a character's own abilities to what a board is about. A board walked by
/// somebody who can drag is a board about dragging whatever else is painted on
/// it.
fn abilities_of(character: Character, found: &mut Vec<Mechanic>) {
    abilities_of_powers(character.abilities(), found);
}

/// The same reading taken off a set of powers rather than off a class, because
/// a board can lend a power as well as start with one.
fn abilities_of_powers(abilities: Abilities, found: &mut Vec<Mechanic>) {
    let mut mark = |present: bool, mechanic: Mechanic| {
        if present && !found.contains(&mechanic) {
            found.push(mechanic);
        }
    };
    mark(abilities.push, Mechanic::Push);
    mark(abilities.pull && !abilities.magnetic, Mechanic::Drag);
    mark(abilities.magnetic, Mechanic::Magnet);
    mark(abilities.swap, Mechanic::Swap);
    mark(abilities.phasing, Mechanic::Phase);
    mark(abilities.wades, Mechanic::Wade);
    mark(abilities.warded, Mechanic::Ward);
    mark(abilities.blinks, Mechanic::Blink);
    mark(abilities.smashes, Mechanic::Break);
}

/// Everything a board is about, in the order the schema declares it.
pub fn mechanics(map: &Map) -> Vec<Mechanic> {
    let mut found = Vec::new();
    let mark = |present: bool, mechanic: Mechanic, found: &mut Vec<Mechanic>| {
        if present && !found.contains(&mechanic) {
            found.push(mechanic);
        }
    };

    abilities_of(map.character, &mut found);
    for member in &map.followers {
        abilities_of(member.character, &mut found);
    }
    mark(!map.followers.is_empty(), Mechanic::Party, &mut found);
    mark(map.rules.crates_push_in_pairs, Mechanic::Pairs, &mut found);
    mark(!map.orbs.is_empty(), Mechanic::Orb, &mut found);
    mark(!map.lamps.is_empty(), Mechanic::Lamp, &mut found);
    mark(!map.stones.is_empty(), Mechanic::Stone, &mut found);
    mark(!map.gems.is_empty(), Mechanic::Gem, &mut found);
    mark(!map.watchers.is_empty(), Mechanic::Watcher, &mut found);
    mark(!map.mirrors.is_empty(), Mechanic::Mirror, &mut found);
    // A gem is a power to stand in as much as a thing to carry, and what the
    // colours on this board lend is exactly what its light is about. A board
    // that has turned that rule off has gems to carry and nothing to stand in,
    // so it is not about what the colours would have lent.
    if map.rules.gem_light_grants_powers {
        for gem in &map.gems {
            abilities_of_powers(gem.color.grants(), &mut found);
        }
    }
    mark(!map.emitters.is_empty(), Mechanic::Beam, &mut found);
    mark(map.floors.len() > 1, Mechanic::Floors, &mut found);
    mark(
        map.floors.iter().any(|floor| floor.slot.layer != 0),
        Mechanic::Storeys,
        &mut found,
    );

    for floor in &map.floors {
        for tile in &floor.tiles {
            let mechanic = match tile {
                Tile::Ice => Mechanic::Ice,
                Tile::Pit => Mechanic::Pit,
                Tile::Water => Mechanic::Water,
                Tile::Fragile => Mechanic::Fragile,
                Tile::Brittle => Mechanic::Brittle,
                Tile::OneWay(_) => Mechanic::OneWay,
                Tile::Conveyor(_) => Mechanic::Conveyor,
                Tile::Portal => Mechanic::Portal,
                Tile::Elevator => Mechanic::Elevator,
                Tile::Plate(_) => Mechanic::Plate,
                Tile::Gate(_) => Mechanic::Gate,
                Tile::Switch(_) => Mechanic::Switch,
                Tile::Emitter(_) => Mechanic::Beam,
                Tile::Mirror(_) => Mechanic::Mirror,
                Tile::Receiver(_) => Mechanic::Receiver,
                Tile::Sensor(_) => Mechanic::Sensor,
                Tile::Socket(_) => Mechanic::Socket,
                Tile::Glass => Mechanic::Glass,
                Tile::Prism(_) => Mechanic::Prism,
                Tile::Splitter => Mechanic::Splitter,
                Tile::Spike(_) => Mechanic::Spike,
                Tile::Incinerator => Mechanic::Incinerator,
                Tile::Shutter(_) => Mechanic::Shutter,
                Tile::Lock(_) => Mechanic::Lock,
                _ => continue,
            };
            if !found.contains(&mechanic) {
                found.push(mechanic);
            }
        }
    }

    found
}

/// How much a board asks of whoever plays it, read off the board rather than
/// searched for. The crates and the markers are the work, the mechanics laid
/// over them are what makes the work hard to see, a second pair of hands is
/// another way to attempt all of it, and the room it stands in is how far the
/// answer has to be carried.
///
/// Nothing here plays the board. A reading that costs a search is no use to a
/// generator deciding what to build next, and a map is plain data, so what it
/// is asking can be read straight off the value.
pub fn complexity(map: &Map) -> u32 {
    // Shoving is what the game is. Every board has it and no board is harder
    // for having it, so it is the one mechanic that counts for nothing.
    let mechanical = mechanics(map)
        .into_iter()
        .filter(|mechanic| *mechanic != Mechanic::Push)
        .count() as u32;
    let pushable = (map.crates.len()
        + map.orbs.len()
        + map.lamps.len()
        + map.stones.len()
        + map.mirrors.len()) as u32;
    // A gem is a second thing to route, and unlike a crate it has to be carried
    // there by the same body that has to be somewhere else afterwards.
    let carried = map.gems.len() as u32;
    let standable: u32 = map
        .floors
        .iter()
        .map(|floor| {
            floor
                .tiles
                .iter()
                .filter(|tile| !tile.blocks_forever())
                .count() as u32
        })
        .sum();

    mechanical * 5
        + pushable * 6
        + carried * 8
        + map.goals.len() as u32 * 4
        + (map.party_size() as u32 - 1) * 8
        + (map.floors.len() as u32 - 1) * 6
        + standable / 12
}

/// The mechanics of a board written out, for a report that has to say what a
/// board is about in one line.
pub fn describe(mechanics: &[Mechanic]) -> String {
    mechanics
        .iter()
        .map(|mechanic| mechanic.name())
        .collect::<Vec<_>>()
        .join(" ")
}
