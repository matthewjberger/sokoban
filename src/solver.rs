//! Playing a map to the end without a player. The answer that comes back is
//! the fewest moves that can reach it, and that count doubles as the map's par,
//! so everything below has to be minimal in moves rather than merely correct.
//!
//! The state space here is implicit. It is generated as it is walked and is far
//! larger than the map itself, so it stays a frontier and a set of keys rather
//! than a graph held in memory. The map's own connectivity, which is small and
//! genuinely an adjacency list, lives in [`crate::schema::connectivity`].
//!
//! Two engines answer the same question. On a board where walking changes
//! nothing but where the body stands, the interesting moves are the shoves and
//! everything between them is a walk that can be worked out on demand, so the
//! search steps from shove to shove and pays the walk as the cost of the edge.
//! On every other board, where the floor drops away behind you or the ice
//! carries you past where you meant to stop, walking is part of the position
//! and the search has to step one move at a time. Which engine a board gets is
//! decided by [`board::Board`] from the board and its rules together.

mod board;
mod movespace;
mod packed;
#[cfg(test)]
mod proofs;
mod pushspace;

use crate::rules::{GemSpot, MapState, Step};
use crate::schema::Map;
use board::Board;
use movespace::MoveSpace;
use pushspace::PushSpace;
use std::hash::{BuildHasher, Hasher};

/// How many states a batch analysis will walk before giving up. High enough to
/// decide every map the campaign ships, at the cost of seconds on the hardest
/// of them, which is the right trade for a command that runs once.
///
/// A browser has a few gigabytes of address space and no more, so it keeps the
/// lower ceiling. A search that runs the machine out of memory is worse than
/// one that says it does not know.
#[cfg(target_arch = "wasm32")]
pub const DEFAULT_STATE_BUDGET: usize = 8_000_000;
#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_STATE_BUDGET: usize = 24_000_000;

/// Marks the root of the tree of moves, which has no move above it.
const ROOT: u32 = u32::MAX;

/// How many positions a search walks between checks when nobody is waiting on
/// the frame. Large enough that the bookkeeping around a slice is noise beside
/// the slice itself.
const BATCH: usize = 8192;

/// What an exhaustive search found. `Unknown` means the search hit its budget
/// before it could prove either answer, which is the honest result for a large
/// map rather than a false negative.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Solvability {
    Solved { moves: usize },
    Unsolvable,
    Unknown { explored: usize },
}

impl Solvability {
    pub fn describe(&self) -> String {
        match self {
            Self::Solved { moves } => format!("solvable in {moves} moves"),
            Self::Unsolvable => "no solution exists".to_string(),
            Self::Unknown { explored } => format!("undecided after {explored} states"),
        }
    }
}

/// Where a search has got to.
pub enum Progress {
    /// Still walking. There is more to do and it has not run out of anything.
    Running,
    Solved(Vec<Step>),
    Unsolvable,
    /// The budget ran out before either answer, which is the honest result on a
    /// board too big rather than a false negative.
    Exhausted,
}

/// A search kept between calls. Everything a walk carries is held here, so the
/// walk can be stopped after any number of positions and picked up exactly
/// where it was. That is what lets a board be proved over as many frames as it
/// takes instead of on the one frame that asked for it.
pub struct Search {
    board: Board,
    engine: Engine,
}

enum Engine {
    Moves(MoveSpace),
    Pushes(PushSpace),
}

impl Search {
    pub fn new(map: &Map, budget: usize) -> Self {
        let board = Board::read(map);
        let engine = if board.quiet {
            Engine::Pushes(PushSpace::new(map, &board, budget))
        } else {
            Engine::Moves(MoveSpace::new(map, &board, budget))
        };
        Self { board, engine }
    }

    /// The same search held to the one move at a time engine, whatever the
    /// board would otherwise have got. Two engines that disagree about a board
    /// are two engines one of which is wrong, and this is how they are asked
    /// the same question.
    #[cfg(test)]
    pub fn in_move_space(map: &Map, budget: usize) -> Self {
        let board = Board::read(map);
        let engine = Engine::Moves(MoveSpace::new(map, &board, budget));
        Self { board, engine }
    }

    /// Walks at most this many positions and says where that left it. A slice
    /// of nothing is a legitimate ask and does nothing.
    pub fn advance(&mut self, map: &Map, slice: usize) -> Progress {
        match &mut self.engine {
            Engine::Moves(inner) => inner.advance(map, &self.board, slice),
            Engine::Pushes(inner) => inner.advance(map, &self.board, slice),
        }
    }

    /// How much has been walked so far, for a screen that has to say something
    /// while it waits and for anything holding the search to a budget.
    ///
    /// It counts work rather than boards. One move at a time, work and boards
    /// are the same thing. Shove to shove, a single position also walks the
    /// region around it to price its shoves, so it is charged for that walk as
    /// well, which is what keeps one budget meaning the same amount of waiting
    /// whichever engine a board was given.
    pub fn explored(&self) -> usize {
        match &self.engine {
            Engine::Moves(inner) => inner.explored(),
            Engine::Pushes(inner) => inner.explored(),
        }
    }

    /// Which engine answered, so a board quietly dropping out of push space is
    /// something that can be seen rather than guessed at.
    pub fn engine(&self) -> &'static str {
        match &self.engine {
            Engine::Moves(_) => "moves",
            Engine::Pushes(_) => "pushes",
        }
    }
}

/// Whether a board can be finished, for everything that wants the answer rather
/// than the route.
pub fn solve(map: &Map, budget: usize) -> Solvability {
    let mut search = Search::new(map, budget);
    loop {
        match search.advance(map, BATCH) {
            Progress::Running => {}
            Progress::Solved(route) => {
                return Solvability::Solved { moves: route.len() };
            }
            Progress::Unsolvable => return Solvability::Unsolvable,
            Progress::Exhausted => {
                return Solvability::Unknown {
                    explored: search.explored(),
                };
            }
        }
    }
}

/// The same search run to its end, for everything that can afford to wait for
/// the answer.
pub fn solve_path(map: &Map, budget: usize) -> Option<Vec<Step>> {
    let mut search = Search::new(map, budget);
    loop {
        match search.advance(map, BATCH) {
            Progress::Running => {}
            Progress::Solved(route) => return Some(route),
            Progress::Unsolvable | Progress::Exhausted => return None,
        }
    }
}

/// A position as one number, for the readings taken outside the search. The
/// solver's own engines key their packed positions directly, so this is the
/// same idea told to a [`MapState`], and the two never have to agree with each
/// other because nothing compares one with the other.
pub(crate) fn search_key(state: &MapState) -> u128 {
    packed::stamp(&canonical(state))
}

/// The party is listed in the map's order and the crates are sorted, because
/// two crates that change places are the same position and two members that do
/// are not.
///
/// Crates are interchangeable, so the run sorts them. Filled pits are implied
/// by the sunk crates sitting in them and need no entry of their own, but a
/// collapsed square, a broken wall, and a thrown latch are implied by nothing
/// and would make two different boards look like one state if they were left
/// out. The crate run is a fixed length for a given map and the two runs after
/// it vary, so a collapsed square is tagged and a broken one is not, which keeps
/// a dropped floor from reading as a hole punched through a wall.
fn canonical(state: &MapState) -> Vec<u64> {
    let mut key = Vec::with_capacity(state.crates.len() + state.collapsed.len() + 3);
    key.push(state.active as u64);
    key.extend(state.members.iter().map(|at| pack(*at, false)));
    key.push(
        state.latched.iter().enumerate().fold(
            0u64,
            |bits, (group, on)| if *on { bits | (1 << group) } else { bits },
        ),
    );
    let crates_start = key.len();
    key.extend(
        state
            .crates
            .iter()
            .map(|entry| pack(entry.at, entry.sunk) | (entry.kind.code() << 49)),
    );
    key[crates_start..].sort_unstable();
    let collapsed_start = key.len();
    key.extend(state.collapsed.iter().map(|at| pack(*at, true)));
    key[collapsed_start..].sort_unstable();
    let broken_start = key.len();
    key.extend(state.broken.iter().map(|at| pack(*at, false)));
    key[broken_start..].sort_unstable();
    // Gems are never interchangeable, since each of them is a colour, so this
    // run stays in the map's order and is not sorted the way the crates are.
    // Which pair of hands is holding one is part of the position too, because
    // putting it down and the other one picking it up is a real difference.
    let watchers_start = key.len();
    key.extend(state.watchers.iter().map(|at| pack(*at, false)));
    key[watchers_start..].sort_unstable();
    key.extend(state.gems.iter().map(|spot| match spot {
        GemSpot::Loose(at) => pack(*at, false),
        GemSpot::Seated(at) => pack(*at, true),
        GemSpot::Held(member) => (1 << 51) | *member as u64,
    }));
    key
}

/// A square packed into an integer so a state key is a run of plain numbers to
/// hash rather than a structure to walk. Sixteen bits each covers any board
/// worth searching.
fn pack(at: crate::schema::Position, sunk: bool) -> u64 {
    let layer = (i64::from(at.layer) + 32768) as u64;
    let x = (i64::from(at.cell.0) + 32768) as u64;
    let y = (i64::from(at.cell.1) + 32768) as u64;
    (layer << 33) | (x << 17) | (y << 1) | u64::from(sunk)
}

/// The hasher for a set of position keys. A key is already two readings of the
/// position folded together, so hashing it again buys nothing and costs a round
/// of SipHash on every lookup in the hottest loop there is.
#[derive(Default, Clone, Copy)]
pub struct Straight;

impl BuildHasher for Straight {
    type Hasher = Folded;

    fn build_hasher(&self) -> Folded {
        Folded(0)
    }
}

#[derive(Default)]
pub struct Folded(u64);

impl Hasher for Folded {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = self.0.rotate_left(8) ^ u64::from(*byte);
        }
    }

    fn write_u128(&mut self, value: u128) {
        self.0 = (value as u64) ^ ((value >> 64) as u64);
    }
}

/// Walks the parent chain back to the root and turns it the right way round.
fn retrace(nodes: &[(u32, Step)], from: u32) -> Vec<Step> {
    let mut route = Vec::new();
    let mut cursor = from;
    while cursor != ROOT {
        let (parent, step) = nodes[cursor as usize];
        route.push(step);
        cursor = parent;
    }
    route.reverse();
    route
}
