//! The map schema: the value that fully describes a puzzle. Everything the
//! game plays, shipped or authored or generated, is one of these, and it round
//! trips through JSON unchanged.

mod connectivity;
mod edit;
mod mechanics;
mod query;
mod summary;
mod validate;

pub use connectivity::*;
pub use edit::*;
pub use mechanics::*;
pub use query::*;
pub use summary::*;
pub use validate::*;

use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, PI};

pub type Cell = (i32, i32);

pub const MIN_EXTENT: i32 = 5;
/// How wide or tall one floor may be. Space is cheap for the search, which
/// spends its budget on crates rather than on empty squares, so a big board is
/// only expensive when it is also a crowded one.
pub const MAX_EXTENT: i32 = 40;

/// A way to face. Board data as much as movement data, because an arrow and a
/// belt each carry one, which is why it lives with the schema rather than with
/// the rules that read it.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Direction {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];

    pub fn delta(self) -> Cell {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    pub fn yaw(self) -> f32 {
        match self {
            Self::Up => 0.0,
            Self::Left => FRAC_PI_2,
            Self::Down => PI,
            Self::Right => -FRAC_PI_2,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }
}

/// Which way a mirror leans. Two of them cover every 45 degree reflector: one
/// leaning like a forward slash and one like a backslash.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Slant {
    /// Leaning like `/`.
    #[default]
    Forward,
    /// Leaning like `\`.
    Back,
}

impl Slant {
    /// Where a beam arriving on this heading leaves. A mirror at forty five
    /// degrees turns a beam a quarter turn, and which quarter is the whole
    /// difference between the two mirrors.
    pub fn deflect(self, heading: Direction) -> Direction {
        match (self, heading) {
            (Self::Forward, Direction::Right) => Direction::Up,
            (Self::Forward, Direction::Up) => Direction::Right,
            (Self::Forward, Direction::Left) => Direction::Down,
            (Self::Forward, Direction::Down) => Direction::Left,
            (Self::Back, Direction::Right) => Direction::Down,
            (Self::Back, Direction::Down) => Direction::Right,
            (Self::Back, Direction::Left) => Direction::Up,
            (Self::Back, Direction::Up) => Direction::Left,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Forward => Self::Back,
            Self::Back => Self::Forward,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "FORWARD",
            Self::Back => "BACK",
        }
    }
}

/// What a gem is. A gem is one colour and nothing else, because the colour is
/// the whole of what it does: seated in a socket it throws light of its own
/// colour, and standing in that light lends whoever stands there the one thing
/// the colour is for.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum GemColor {
    #[default]
    Ruby,
    Amber,
    Jade,
    Azure,
}

impl GemColor {
    pub const ALL: [GemColor; 4] = [
        GemColor::Ruby,
        GemColor::Amber,
        GemColor::Jade,
        GemColor::Azure,
    ];

    /// What standing in this colour lends. Every one of them is an ability some
    /// character is born with, so a shover in ruby light is a phaser for as long
    /// as they stand in it, and the rules that read the two cannot drift apart.
    pub fn grants(self) -> Abilities {
        match self {
            Self::Ruby => Abilities {
                phasing: true,
                ..Abilities::NONE
            },
            Self::Amber => Abilities {
                smashes: true,
                ..Abilities::NONE
            },
            Self::Jade => Abilities {
                wades: true,
                ..Abilities::NONE
            },
            Self::Azure => Abilities {
                blinks: true,
                ..Abilities::NONE
            },
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ruby => "RUBY",
            Self::Amber => "AMBER",
            Self::Jade => "JADE",
            Self::Azure => "AZURE",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::Ruby => "step through one wall",
            Self::Amber => "break a boulder",
            Self::Jade => "walk on water",
            Self::Azure => "cross a gap",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Ruby => Self::Amber,
            Self::Amber => Self::Jade,
            Self::Jade => Self::Azure,
            Self::Azure => Self::Ruby,
        }
    }
}

/// One gem and where it is lying when the board is laid out. A gem is carried
/// rather than shoved, so where it is at any moment is part of the position
/// rather than part of the board, and this is only the start of it.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Gem {
    pub at: Position,
    pub color: GemColor,
}

impl Gem {
    /// Whether this is the colour a door is asking for.
    pub fn colour_matches(self, colour: GemColor) -> bool {
        self.color == colour
    }
}

/// Who is pushing the crates. A map picks one, and what it can do is a
/// property of the character rather than of the board, so the same layout is a
/// different puzzle depending on who walks into it.
///
/// What a character can do with a crate. Naming the qualities rather than the
/// characters is what keeps two of them from quietly becoming the same one:
/// each character is a different set of these, and no two sets match.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Abilities {
    /// Walking into a crate moves it.
    pub push: bool,
    /// Stepping away from a crate can take it along, when asked.
    pub pull: bool,
    /// The pull is not a choice. Stepping away from a crate always takes it,
    /// so this one can never put a crate down and walk off.
    pub magnetic: bool,
    /// Moving trades places with the first crate along that line rather than
    /// touching it, which reaches crates nothing else can and moves them
    /// backwards to where the reach came from.
    pub swap: bool,
    /// Water carries this one, so what stops everything else is a road.
    pub wades: bool,
    /// A single wall is one step rather than a stop, when there is somewhere to
    /// arrive on the far side of it.
    pub phasing: bool,
    /// Light does not harm this one, so a beam is scenery rather than a wall
    /// made of death.
    pub warded: bool,
    /// A gap is something to cross rather than an edge to stop at. Moving into
    /// open air carries this one over it to the first ground on the far side,
    /// which is what makes a board of islands a board rather than a set of
    /// separate ones.
    pub blinks: bool,
    /// A boulder gives way to bare hands. Walking into one breaks it and the
    /// square it stood on is clear afterwards, which is the only thing on the
    /// board that answers to a boulder at all.
    pub smashes: bool,
}

/// Who is pushing the crates, as a set of qualities rather than a special case.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Character {
    /// Shoves, and only shoves. The classic.
    #[default]
    Pusher,
    /// Drags only. Walking into a crate does nothing at all, so where a shover
    /// stands to move a crate is exactly where this one cannot.
    Dragger,
    /// Drags only, and never by choice. Step away from a crate and it comes, so
    /// every move is a decision about what is behind as well as ahead.
    Magnet,
    /// Touches nothing. Moving trades places with the first crate down that
    /// line, however far off it is, which is the only way to move a crate
    /// towards yourself.
    Swapper,
    /// Shoves, and walks on water, so the map it is given is a different map.
    Wader,
    /// Shoves, and steps through a single wall when there is floor behind it.
    Phaser,
    /// Shoves, and stands in a beam unharmed.
    Warden,
    /// Shoves, and crosses open air to the next ground along, so a gap is a
    /// road to this one and a wall to everything else.
    Blinker,
    /// Shoves, and breaks a boulder by walking into it, so the one thing on the
    /// board that no shove moves is the one thing this is for.
    Breaker,
}

impl Abilities {
    /// Everything either of them can do. A party is as able as its ablest
    /// member for anything about where a body can get, because pointing the
    /// controls at that member is a move like any other.
    pub fn union(self, other: Self) -> Self {
        Self {
            push: self.push || other.push,
            pull: self.pull || other.pull,
            magnetic: self.magnetic || other.magnetic,
            swap: self.swap || other.swap,
            wades: self.wades || other.wades,
            phasing: self.phasing || other.phasing,
            warded: self.warded || other.warded,
            blinks: self.blinks || other.blinks,
            smashes: self.smashes || other.smashes,
        }
    }

    /// A character that can do nothing to a crate at all, which every other set
    /// is written as a departure from.
    pub const NONE: Abilities = Abilities {
        push: false,
        pull: false,
        magnetic: false,
        swap: false,
        wades: false,
        phasing: false,
        warded: false,
        blinks: false,
        smashes: false,
    };
}

impl Character {
    pub fn abilities(self) -> Abilities {
        match self {
            Self::Pusher => Abilities {
                push: true,
                ..Abilities::NONE
            },
            Self::Dragger => Abilities {
                pull: true,
                ..Abilities::NONE
            },
            Self::Magnet => Abilities {
                pull: true,
                magnetic: true,
                ..Abilities::NONE
            },
            Self::Swapper => Abilities {
                swap: true,
                ..Abilities::NONE
            },
            Self::Wader => Abilities {
                push: true,
                wades: true,
                ..Abilities::NONE
            },
            Self::Phaser => Abilities {
                push: true,
                phasing: true,
                ..Abilities::NONE
            },
            Self::Warden => Abilities {
                push: true,
                warded: true,
                ..Abilities::NONE
            },
            Self::Blinker => Abilities {
                push: true,
                blinks: true,
                ..Abilities::NONE
            },
            Self::Breaker => Abilities {
                push: true,
                smashes: true,
                ..Abilities::NONE
            },
        }
    }

    /// Whether a drag is available at all. What a body can do in the middle of
    /// a move is asked of its powers rather than of its class, because the
    /// light can lend one, and this is the one question asked before a move
    /// starts, where there is nothing to lend yet.
    pub fn can_pull(self) -> bool {
        self.abilities().pull
    }

    pub const ALL: [Character; 9] = [
        Character::Pusher,
        Character::Dragger,
        Character::Magnet,
        Character::Swapper,
        Character::Wader,
        Character::Phaser,
        Character::Warden,
        Character::Blinker,
        Character::Breaker,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pusher => "PUSHER",
            Self::Dragger => "DRAGGER",
            Self::Magnet => "MAGNET",
            Self::Swapper => "SWAPPER",
            Self::Wader => "WADER",
            Self::Phaser => "PHASER",
            Self::Warden => "WARDEN",
            Self::Blinker => "BLINKER",
            Self::Breaker => "BREAKER",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::Pusher => "pushes only",
            Self::Dragger => "drags only, on request",
            Self::Magnet => "drags only, and never by choice",
            Self::Swapper => "trades places with the first crate in line",
            Self::Wader => "shoves, and walks on water",
            Self::Phaser => "shoves, and steps through one wall",
            Self::Warden => "shoves, and beams do not harm it",
            Self::Blinker => "shoves, and crosses gaps to the next ground",
            Self::Breaker => "shoves, and breaks a boulder bare handed",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Pusher => Self::Dragger,
            Self::Dragger => Self::Magnet,
            Self::Magnet => Self::Swapper,
            Self::Swapper => Self::Wader,
            Self::Wader => Self::Phaser,
            Self::Phaser => Self::Warden,
            Self::Warden => Self::Blinker,
            Self::Blinker => Self::Breaker,
            Self::Breaker => Self::Pusher,
        }
    }
}

/// One grid square. Everything fixed to the board lives here. The things that
/// move (player, crates) and the things that score (goals) are separate lists
/// on [`Map`].
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tile {
    #[default]
    Void,
    Floor,
    Wall,
    Ice,
    Pit,
    Plate(u8),
    Gate(u8),
    Portal,
    Elevator,
    /// Passable only by something already travelling the way it points.
    OneWay(Direction),
    /// Carries whatever lands on it one square its own way, which can turn a
    /// traveller off the line it arrived on.
    Conveyor(Direction),
    /// Floor that holds once. Step off it, or slide over it, and it drops away
    /// into a hole behind you.
    Fragile,
    /// Flips its gate group and leaves it flipped, unlike a plate that has to
    /// be held down.
    Switch(u8),
    /// A wall until a crate is spent breaking it, after which it is floor.
    Brittle,
    /// Open water. A crate pushed in is gone and the water is no shallower for
    /// it, which is what separates it from a pit worth spending a crate on.
    Water,
    /// Throws a beam along its heading, always, from the moment the board is
    /// laid out.
    Emitter(Direction),
    /// Turns a beam a quarter turn and is otherwise ordinary floor.
    Mirror(Slant),
    /// A tower that drinks a beam and powers its gate group for as long as one
    /// reaches it.
    Receiver(u8),
    /// A pad that powers its gate group while it is lit. What lights it is
    /// worked out from where the lamps are and what is in the way, not from
    /// what the picture happens to look like.
    Sensor(u8),
    /// A door to another puzzle, naming the one behind it. Ordinary floor to
    /// walk on and to the rules, so a board with these on it is a board like
    /// any other, and standing on one is what the game outside the rules makes
    /// something of.
    Gateway(u8),
    /// A plinth that holds one gem, pointing the way whatever it holds will
    /// throw its light. Ordinary floor until a gem is carried to it, which is
    /// the whole of what makes it a machine somebody has to feed rather than a
    /// machine that was always running.
    Socket(Direction),
    /// A pane. Solid to a body and clear to light, so it is the one wall a beam
    /// crosses and the one window nobody climbs through.
    Glass,
    /// A lens that stains whatever light crosses it. The beam that leaves is
    /// the colour of the lens whatever colour arrived, so a board with one on it
    /// has a colour it can make without a gem of that colour on it at all.
    Prism(GemColor),
    /// A wedge that cuts a beam in two and throws the halves out either side of
    /// the line it arrived on, so one source answers two questions.
    Splitter,
    /// A bed of spikes, up while its group is powered and down otherwise. It
    /// does nothing whatever to a crate, so it is a door that only bars bodies.
    Spike(u8),
    /// A shutter: shut while its group is powered and open while it is not,
    /// which is a gate read backwards. A board with both on one group has two
    /// doors that are never open at the same time.
    Shutter(u8),
    /// A door that answers to what is in your hands rather than to anything on
    /// the board. It stands open for as long as somebody in the party is
    /// carrying a gem of its colour, so a key is a gem with somewhere to be.
    Lock(GemColor),
    /// A burner set into the floor: steel leaves over a fire, held shut by
    /// anything as light as a body and opened by anything as heavy as a crate.
    /// What goes in does not come back, so unlike a hole it never fills and
    /// never becomes a way across, and unlike water it is ground to walk on.
    Incinerator,
}

impl Tile {
    /// Whether a body is stopped by this square. A mirror, an emitter and a
    /// tower are all machines standing on their square rather than markings on
    /// it, so nothing shares one, and a beam cannot be walked round by stepping
    /// onto the thing that threw it.
    pub fn blocks_walking(self) -> bool {
        matches!(
            self,
            Self::Void
                | Self::Wall
                | Self::Brittle
                | Self::Water
                | Self::Mirror(_)
                | Self::Emitter(_)
                | Self::Receiver(_)
                | Self::Glass
                | Self::Prism(_)
                | Self::Splitter
        )
    }

    /// Solid with no way through it, ever. A brittle wall blocks walking but
    /// does not block forever, which is the difference that decides whether a
    /// crate beside one is really out of moves. The machines are the other way
    /// about, because nothing ever shares one and nothing ever clears one.
    pub fn blocks_forever(self) -> bool {
        matches!(
            self,
            Self::Void
                | Self::Wall
                | Self::Water
                | Self::Mirror(_)
                | Self::Emitter(_)
                | Self::Receiver(_)
                | Self::Glass
                | Self::Prism(_)
                | Self::Splitter
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Void => "VOID",
            Self::Floor => "FLOOR",
            Self::Wall => "WALL",
            Self::Ice => "ICE",
            Self::Pit => "PIT",
            Self::Plate(_) => "PLATE",
            Self::Gate(_) => "GATE",
            Self::Portal => "PORTAL",
            Self::Elevator => "ELEVATOR",
            Self::OneWay(_) => "ONE WAY",
            Self::Conveyor(_) => "BELT",
            Self::Fragile => "FRAGILE",
            Self::Switch(_) => "SWITCH",
            Self::Brittle => "BRITTLE",
            Self::Water => "WATER",
            Self::Gateway(_) => "GATEWAY",
            Self::Emitter(_) => "EMITTER",
            Self::Mirror(_) => "MIRROR",
            Self::Receiver(_) => "TOWER",
            Self::Sensor(_) => "SENSOR",
            Self::Socket(_) => "SOCKET",
            Self::Glass => "GLASS",
            Self::Prism(_) => "PRISM",
            Self::Splitter => "SPLITTER",
            Self::Spike(_) => "SPIKES",
            Self::Shutter(_) => "SHUTTER",
            Self::Lock(_) => "LOCK",
            Self::Incinerator => "INCINERATOR",
        }
    }

    /// Whether a beam stops here. Light is stopped by what light is stopped by:
    /// a wall it cannot pass, a tower that drinks it, and the housing of
    /// another emitter. A gate is a grille and a beam goes straight through,
    /// which is also what keeps a gate opened by a tower from deciding whether
    /// the beam that opened it arrives.
    pub fn stops_light(self) -> bool {
        matches!(
            self,
            Self::Void | Self::Wall | Self::Brittle | Self::Receiver(_) | Self::Emitter(_)
        )
    }

    /// The way an arrow or a belt points, for anything that has to draw it or
    /// cycle it.
    pub fn heading(self) -> Option<Direction> {
        match self {
            Self::OneWay(way) | Self::Conveyor(way) => Some(way),
            _ => None,
        }
    }

    /// Whether a body could ever be on this square. A door might open, a hole
    /// might fill and a cracked wall might come down, so none of those shuts a
    /// square out for good. Water does not either, since somebody who wades
    /// walks on it and a trade reaches straight across it.
    pub fn open_to_bodies(self) -> bool {
        !self.blocks_forever() || self == Self::Water
    }

    /// Whether a crate spent on this square leaves a mark the board carries
    /// afterwards. A hole fills, a fragile square drops and a cracked wall
    /// comes down, and each of those changes what the square is. Water keeps no
    /// record, because it is exactly as wet as it was before.
    ///
    /// This is the whole of what a position has to remember about the board
    /// itself, so a square that records anything has to be named here or the
    /// record has nowhere to live.
    pub fn records_spending(self) -> bool {
        match self {
            Self::Pit | Self::Fragile | Self::Brittle => true,
            Self::Void
            | Self::Floor
            | Self::Wall
            | Self::Ice
            | Self::Plate(_)
            | Self::Gate(_)
            | Self::Portal
            | Self::Elevator
            | Self::OneWay(_)
            | Self::Conveyor(_)
            | Self::Switch(_)
            | Self::Water
            | Self::Emitter(_)
            | Self::Mirror(_)
            | Self::Receiver(_)
            | Self::Sensor(_)
            | Self::Gateway(_)
            | Self::Socket(_)
            | Self::Glass
            | Self::Prism(_)
            | Self::Splitter
            | Self::Spike(_)
            | Self::Shutter(_)
            | Self::Lock(_)
            | Self::Incinerator => false,
        }
    }

    /// Whether crossing this square carries a body somewhere it did not ask to
    /// go, or leaves the board different once it has gone. Ice carries, a belt
    /// turns, a pad throws, a lift moves, a switch flips its gate and stays
    /// flipped, and fragile floor drops away behind. A board with none of these
    /// is a board where walking only ever changes where a body stands, which is
    /// what lets a search work in shoves rather than in steps.
    ///
    /// A plate is not one of them. It holds its gate only while something
    /// stands on it, so what it does is read off where the bodies are rather
    /// than carried from one position to the next.
    ///
    /// Every tile is named rather than left to a catch-all, so a new one has to
    /// answer before it compiles.
    pub fn stirs_the_board(self, rules: &Rules) -> bool {
        match self {
            Self::Ice => rules.ice_slides_player,
            Self::Conveyor(_) => rules.conveyors_carry_player,
            Self::Portal => rules.portals_carry_player,
            Self::Elevator => rules.elevators_move_player,
            Self::Switch(_) => rules.switches_toggle_gates,
            Self::Fragile => rules.fragile_floor_collapses,
            Self::Void
            | Self::Floor
            | Self::Wall
            | Self::Pit
            | Self::Plate(_)
            | Self::Gate(_)
            | Self::OneWay(_)
            | Self::Brittle
            | Self::Water
            | Self::Emitter(_)
            | Self::Mirror(_)
            | Self::Receiver(_)
            | Self::Sensor(_)
            | Self::Gateway(_)
            | Self::Socket(_)
            | Self::Glass
            | Self::Prism(_)
            | Self::Splitter
            | Self::Spike(_)
            | Self::Shutter(_)
            | Self::Lock(_)
            | Self::Incinerator => false,
        }
    }

    /// Whether the board reads the squares a move crossed rather than only
    /// where it ended. Fragile floor drops behind whatever left it and a switch
    /// answers to anything passing over it, and nothing else looks at the
    /// trail, so a board with neither can be searched without recording where
    /// anything walked.
    ///
    /// Named tile by tile for the same reason as [`Tile::stirs_the_board`].
    /// Dropping a trail the board reads stops both of those rules working.
    pub fn reads_the_trail(self, rules: &Rules) -> bool {
        match self {
            Self::Fragile => rules.fragile_floor_collapses,
            Self::Switch(_) => rules.switches_toggle_gates,
            Self::Void
            | Self::Floor
            | Self::Wall
            | Self::Ice
            | Self::Pit
            | Self::Plate(_)
            | Self::Gate(_)
            | Self::Portal
            | Self::Elevator
            | Self::OneWay(_)
            | Self::Conveyor(_)
            | Self::Brittle
            | Self::Water
            | Self::Emitter(_)
            | Self::Mirror(_)
            | Self::Receiver(_)
            | Self::Sensor(_)
            | Self::Gateway(_)
            | Self::Socket(_)
            | Self::Glass
            | Self::Prism(_)
            | Self::Splitter
            | Self::Spike(_)
            | Self::Shutter(_)
            | Self::Lock(_)
            | Self::Incinerator => false,
        }
    }
}

/// Where a floor sits in the lattice. Columns and rows place it beside its
/// neighbours on the same storey, which is how a player walks from one floor
/// onto the next without anything special happening. Layers stack them.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Slot {
    pub column: i32,
    pub row: i32,
    pub layer: i32,
}

/// A square anywhere in the map: the storey plus the cell in that storey's
/// shared coordinate space. Floors carve that space into tiles, and the cell
/// numbering runs straight across floor boundaries.
#[derive(
    Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default,
)]
pub struct Position {
    pub layer: i32,
    pub cell: Cell,
}

impl Position {
    pub fn new(layer: i32, cell: Cell) -> Self {
        Self { layer, cell }
    }

    pub fn offset(self, delta: Cell) -> Self {
        Self {
            layer: self.layer,
            cell: (self.cell.0 + delta.0, self.cell.1 + delta.1),
        }
    }
}

/// What a pushable thing is. All three shove the same way and differ in what
/// they do to light: a box stops it, an orb stops it, and a lamp is where it
/// comes from.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CrateKind {
    #[default]
    Box,
    /// A mirrored sphere.
    Orb,
    /// A light on the floor, shoved about like anything else.
    Lamp,
    /// A boulder. No shove in the game moves one, so a boulder is a wall that
    /// happens to be standing in the room rather than built into it, and the
    /// only thing that answers it is a pair of hands that can break it.
    Stone,
    /// A mirror on a pallet. It turns a beam the way a mirror set in the floor
    /// does and it shoves the way a crate does, so where the light goes is
    /// something a body can push around the room.
    Mirror(Slant),
}

impl CrateKind {
    /// What this is, as a number, for the run of squares a search hashes. Two
    /// crates that changed places are one position and two that are different
    /// things are not, so the kind rides above the square and bands the sort.
    pub fn code(self) -> u64 {
        match self {
            Self::Box => 0,
            Self::Orb => 1,
            Self::Lamp => 2,
            Self::Stone => 3,
            Self::Mirror(Slant::Forward) => 4,
            Self::Mirror(Slant::Back) => 5,
        }
    }

    /// Whether a shove moves it at all. A boulder is the one thing on the board
    /// that no hand and no trade shifts.
    pub fn shoves(self) -> bool {
        !matches!(self, Self::Stone)
    }
}

/// One member of the party: who they are and where they start. A board always
/// has at least one, which is why the first is named on the map itself and only
/// the rest are a list.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Member {
    pub at: Position,
    pub character: Character,
}

/// One rectangle of tiles pinned to a lattice slot. Every floor in a map is
/// the same size, so the lattice stride is exact and neighbouring floors line
/// up cell for cell.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Floor {
    pub slot: Slot,
    pub tiles: Vec<Tile>,
    /// What this floor is made of, when it is not made of what the rest of the
    /// map is made of. A lattice of floors walked as one storey can be a
    /// warehouse in one corner and a freezer in the next, which is how an
    /// overworld gets areas that look like different places.
    pub skin: Option<Skin>,
}

/// How the board behaves. The tiles say what is there, these say what it does,
/// so a map is playable from its own data without the code deciding anything
/// on its behalf. A generator can dial these to make a whole family of variant
/// puzzles out of one layout.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(default)]
pub struct Rules {
    pub ice_slides_player: bool,
    pub ice_slides_crates: bool,
    pub pits_swallow_crates: bool,
    pub filled_pits_are_floor: bool,
    pub portals_carry_player: bool,
    pub portals_carry_crates: bool,
    /// Momentum survives the trip. Leave a pad with ice directly ahead and the
    /// slide picks up again there, which throws a crate somewhere no push
    /// could reach it.
    pub portal_exit_continues_on_ice: bool,
    pub plates_sense_player: bool,
    pub plates_sense_crates: bool,
    pub elevators_move_player: bool,
    /// A crate pushed onto an elevator rides it, down when there is a storey
    /// below and up otherwise, which is what lets a puzzle span floors.
    pub elevators_move_crates: bool,
    pub one_way_stops_player: bool,
    pub one_way_stops_crates: bool,
    pub conveyors_carry_player: bool,
    pub conveyors_carry_crates: bool,
    /// Fragile floor drops away behind whatever leaves it. With this off the
    /// squares are ordinary floor, which is the switch a generator turns to
    /// soften a layout it already trusts.
    pub fragile_floor_collapses: bool,
    /// A switch flips its gate group on the way past and leaves it flipped, so
    /// a gate stands open with nothing holding it.
    pub switches_toggle_gates: bool,
    /// A crate pushed into a brittle wall is spent breaking it, which turns the
    /// wall into floor and gives a spare crate a use besides filling a pit.
    pub crates_break_brittle: bool,
    /// Water takes a crate and keeps it. Turn this off and the water is merely
    /// scenery the crates refuse to enter.
    pub crates_sink_in_water: bool,
    /// How far a lamp throws light, in squares. Nothing beyond this is lit
    /// however clear the line to it.
    pub light_range: u8,
    /// How far a blink carries somebody over open air, in squares. Data for the
    /// same reason the light range is, because it dials how a board plays, and a
    /// board that wants wider gaps should be able to say so.
    pub blink_reach: u8,
    /// Lets one shove move two crates standing in a line, which turns a pair
    /// that would otherwise jam each other into a thing worth arranging.
    pub crates_push_in_pairs: bool,
    /// Standing in the light a seated gem throws lends its power for as long as
    /// the light reaches. Turn this off and a socket is a lamp stand.
    pub gem_light_grants_powers: bool,
    /// A boulder gives way to whoever has the hands for it. Turn this off and a
    /// boulder is simply immovable, which makes it a wall that can be walked
    /// around rather than a wall that can be answered.
    pub stones_break_bare_handed: bool,
    /// Spikes kill whoever stands on them while their group is powered. Turn
    /// this off and they are a pattern on the floor.
    pub spikes_impale: bool,
    /// A crate pushed onto a burner goes through the leaves and is gone. Turn
    /// this off and the leaves hold, which makes it a warm patch of floor.
    pub incinerators_burn_crates: bool,
    pub win: WinCondition,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            ice_slides_player: true,
            ice_slides_crates: true,
            pits_swallow_crates: true,
            filled_pits_are_floor: true,
            portals_carry_player: true,
            portals_carry_crates: true,
            portal_exit_continues_on_ice: true,
            plates_sense_player: true,
            plates_sense_crates: true,
            elevators_move_player: true,
            elevators_move_crates: true,
            one_way_stops_player: true,
            one_way_stops_crates: true,
            conveyors_carry_player: true,
            conveyors_carry_crates: true,
            fragile_floor_collapses: true,
            switches_toggle_gates: true,
            crates_break_brittle: true,
            crates_sink_in_water: true,
            light_range: 4,
            blink_reach: 4,
            crates_push_in_pairs: false,
            gem_light_grants_powers: true,
            stones_break_bare_handed: true,
            spikes_impale: true,
            incinerators_burn_crates: true,
            win: WinCondition::GoalsCovered,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WinCondition {
    /// Every goal square holds a crate. Spare crates may be spent on pits.
    #[default]
    GoalsCovered,
    /// Every surviving crate stands on a goal, the classic strict form.
    CratesOnGoals,
    /// Every socket on the board holds a gem. The crates are whatever the board
    /// makes of them rather than the point of it, which is what lets a board be
    /// about carrying rather than about shoving.
    SocketsFilled,
}

impl WinCondition {
    pub fn label(self) -> &'static str {
        match self {
            Self::GoalsCovered => "FILL THE GOALS",
            Self::CratesOnGoals => "PLACE EVERY CRATE",
            Self::SocketsFilled => "SEAT EVERY GEM",
        }
    }

    /// Whether the markers are what the board is asking for. Everything that
    /// prunes a search by counting crates against markers has to ask this
    /// first, because a board won by seating gems is owed nothing by its crates.
    pub fn wants_crates(self) -> bool {
        match self {
            Self::GoalsCovered | Self::CratesOnGoals => true,
            Self::SocketsFilled => false,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::GoalsCovered => Self::CratesOnGoals,
            Self::CratesOnGoals => Self::SocketsFilled,
            Self::SocketsFilled => Self::GoalsCovered,
        }
    }
}

/// Which look the map wears. The schema names the intent and the world builder
/// owns the palette and atmosphere that intent maps to, so reskinning is a
/// data edit rather than a code edit.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Skin {
    #[default]
    Warehouse,
    Glacier,
    Quarry,
    Vault,
    /// Ground broken into islands with nothing under it, lit by the sky rather
    /// than by a roof.
    Drift,
    /// Overgrown and open to the weather.
    Grove,
}

impl Skin {
    pub fn label(self) -> &'static str {
        match self {
            Self::Warehouse => "WAREHOUSE",
            Self::Glacier => "GLACIER",
            Self::Quarry => "QUARRY",
            Self::Vault => "VAULT",
            Self::Drift => "DRIFT",
            Self::Grove => "GROVE",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Warehouse => Self::Glacier,
            Self::Glacier => Self::Quarry,
            Self::Quarry => Self::Vault,
            Self::Vault => Self::Drift,
            Self::Drift => Self::Grove,
            Self::Grove => Self::Warehouse,
        }
    }
}

/// The whole definition of a puzzle: a lattice of same sized floors, the
/// entities placed on them, the rules that govern those entities, and the look
/// it wears. Every map the game plays, shipped or authored in the editor or
/// produced by a generator, is one of these values, and it round trips through
/// JSON unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Map {
    pub name: String,
    pub hint: String,
    pub par: u32,
    pub floor_width: i32,
    pub floor_height: i32,
    pub floors: Vec<Floor>,
    pub player: Position,
    pub crates: Vec<Position>,
    /// Mirrored spheres. They are crates in every rule that moves them and stop
    /// a beam dead, and they are round and polished, which is the whole of the
    /// difference.
    pub orbs: Vec<Position>,
    /// Lamps, which are pushed about like crates and are what the sensors are
    /// reading.
    pub lamps: Vec<Position>,
    /// Boulders. They are crates to everything that counts crates and to
    /// nothing that moves them, which is the whole of the difference.
    pub stones: Vec<Position>,
    /// The gems lying about, each one a colour and a square to start on. Where
    /// a gem is once the board is being played is part of the position, because
    /// gems are carried.
    pub gems: Vec<Gem>,
    /// Mirrors on pallets, each with the way it leans. They shove like crates
    /// and turn light like the mirrors set into the floor, so which list they
    /// live in is the whole of what makes them one and not the other.
    pub mirrors: Vec<(Position, Slant)>,
    /// The watchers, and where each one stands when the board is laid out. A
    /// watcher kills whatever comes within reach of it and can be traded with
    /// from further off, so where they are ends up being part of the position
    /// rather than part of the board.
    pub watchers: Vec<Position>,
    pub goals: Vec<Position>,
    pub portals: Vec<(Position, Position)>,
    /// Every emitter on the board, found once when the map is relinked. Tracing
    /// beams runs inside the search, and hunting the board for sources on every
    /// call would be the expensive half of it.
    pub emitters: Vec<(Position, Direction)>,
    /// Every socket, found the same way and for the same reason. A socket with
    /// a gem in it is a source of light, so this is the other half of the list
    /// the tracer walks.
    pub sockets: Vec<(Position, Direction)>,
    /// Every bed of spikes and the group that raises it. Whether the board has
    /// killed anybody is asked after every single move, and a board with no
    /// spikes on it should not pay to find that out.
    pub spikes: Vec<(Position, u8)>,
    /// Every lens. A lens can stain an emitter's beam into a colour that lends
    /// a power, so this is the other half of the answer to whether the light on
    /// this board is worth asking about at all.
    pub prisms: Vec<Position>,
    pub rules: Rules,
    pub skin: Skin,
    /// Who the first of the party is. A board with nobody else on it is a board
    /// with one character, which is most of them.
    pub character: Character,
    /// Everyone else, if there is anyone. Swapping between them is a move like
    /// any other, so who is standing where is part of the position rather than
    /// part of the interface.
    pub followers: Vec<Member>,
}

impl Map {
    /// How many are in the party, which is one more than the list of the rest.
    pub fn party_size(&self) -> usize {
        self.followers.len() + 1
    }

    /// Who the member at this index is. Written without building the party so
    /// the search can ask it as often as it likes.
    /// The class after this map's leader that no follower already is. A party
    /// holds one of each, so cycling the leader has to step over whoever is
    /// already standing on the board rather than land on top of them.
    pub fn next_free_character(&self) -> Character {
        let mut candidate = self.character;
        for _ in 0..Character::ALL.len() {
            candidate = candidate.next();
            if !self
                .followers
                .iter()
                .any(|member| member.character == candidate)
            {
                return candidate;
            }
        }
        self.character
    }

    /// Everything anybody on this board can do. Whether a square can be got to
    /// is a question about the party rather than about whoever is being moved
    /// at the time, since swapping to the one who can is itself a move.
    pub fn party_abilities(&self) -> Abilities {
        (0..self.party_size())
            .map(|index| self.member_character(index).abilities())
            .fold(Abilities::NONE, Abilities::union)
    }

    /// Everything anybody on this board could ever do, counting what its gems
    /// and its lenses could lend them. A gem is carried to a socket and the
    /// light it throws is a power somebody can stand in, so a board with one on
    /// it allows more than its party does.
    ///
    /// Every reading that decides what is impossible has to be taken against
    /// this rather than against the party. A pin, a walled off square and a
    /// crate nothing can reach are all claims that a move is not there, and a
    /// claim like that made against the smaller set is a claim that is wrong.
    pub fn latent_abilities(&self) -> Abilities {
        let mut powers = self.party_abilities();
        if !self.rules.gem_light_grants_powers {
            return powers;
        }
        for gem in &self.gems {
            powers = powers.union(gem.color.grants());
        }
        // A lens stains whatever crosses it, so a board with a lens and any
        // source at all can make that colour without owning a gem of it.
        if self.gems.is_empty() && self.emitters.is_empty() {
            return powers;
        }
        for floor in &self.floors {
            for tile in &floor.tiles {
                if let Tile::Prism(color) = tile {
                    powers = powers.union(color.grants());
                }
            }
        }
        powers
    }

    pub fn member_character(&self, index: usize) -> Character {
        match index.checked_sub(1) {
            None => self.character,
            Some(follower) => self
                .followers
                .get(follower)
                .map(|member| member.character)
                .unwrap_or_default(),
        }
    }

    /// Where the member at this index starts.
    pub fn member_start(&self, index: usize) -> Position {
        match index.checked_sub(1) {
            None => self.player,
            Some(follower) => self
                .followers
                .get(follower)
                .map(|member| member.at)
                .unwrap_or_default(),
        }
    }
}

impl Default for Map {
    fn default() -> Self {
        Self {
            name: String::new(),
            hint: String::new(),
            par: 0,
            floor_width: MIN_EXTENT,
            floor_height: MIN_EXTENT,
            floors: Vec::new(),
            player: Position::default(),
            crates: Vec::new(),
            orbs: Vec::new(),
            lamps: Vec::new(),
            stones: Vec::new(),
            gems: Vec::new(),
            mirrors: Vec::new(),
            watchers: Vec::new(),
            goals: Vec::new(),
            portals: Vec::new(),
            emitters: Vec::new(),
            sockets: Vec::new(),
            spikes: Vec::new(),
            prisms: Vec::new(),
            rules: Rules::default(),
            skin: Skin::default(),
            character: Character::default(),
            followers: Vec::new(),
        }
    }
}
