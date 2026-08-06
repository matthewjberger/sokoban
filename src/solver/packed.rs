//! A position, small enough that a search can hold tens of millions of them.
//!
//! The board itself never changes, so most of what [`crate::rules::MapState`]
//! carries is either fixed by the board or implied by the rest. What is left is
//! a run of squares, two bytes of switch and gate state, and one bit per square
//! that can be filled, dropped or broken. That is two allocations rather than
//! five, and about a fifth of the memory, which is the whole of why the budget
//! can be what it is.

use super::board::Board;
use crate::rules::{CrateState, Direction, GATE_GROUPS, GemSpot, MapState};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// The top bit of a crate's square, set once it has been spent.
const SUNK: u32 = 1 << 31;

/// A gem in somebody's hands rather than on a square, with which pair of hands
/// in the bits below.
const HELD: u32 = 1 << 31;

/// A gem seated in the socket on the square below.
const SEATED: u32 = 1 << 30;

/// Where the second reading of a position starts from. Any number nobody else
/// uses will do, and this one is the golden ratio, which is the usual choice.
const SECOND_READING: u64 = 0x9E37_79B9_7F4A_7C15;

/// Which plane of the spent bits is which.
const FILLED: usize = 0;
const DROPPED: usize = 1;
const BROKEN: usize = 2;

#[derive(Clone)]
pub struct Packed {
    /// The party's squares followed by the crates', in one run. Which is which
    /// comes from the board, so nothing here has to say.
    entities: Box<[u32]>,
    /// Where each gem is: a square, a square with the seated bit, or a pair of
    /// hands. Gems are never interchangeable, because each is a colour, so this
    /// stays in the map's order and is never sorted the way the crates are.
    gems: Box<[u32]>,
    /// Where each watcher is standing. They are interchangeable with each other
    /// and with nothing else, so the run is sorted the way the crates are.
    watchers: Box<[u32]>,
    active: u8,
    latched: u8,
    /// Three planes over the squares that can be filled, dropped or broken.
    /// Most boards have none of those and this is empty.
    planes: Box<[u64]>,
}

impl Packed {
    pub fn read(board: &Board, state: &MapState) -> Self {
        let mut entities = Vec::with_capacity(state.members.len() + state.crates.len());
        entities.extend(state.members.iter().map(|at| board.index(*at)));
        entities.extend(
            state
                .crates
                .iter()
                .map(|entry| board.index(entry.at) | if entry.sunk { SUNK } else { 0 }),
        );

        let words = board.spendable_words();
        let mut planes = vec![0u64; words * 3];
        for (plane, squares) in [
            (FILLED, &state.pits_filled),
            (DROPPED, &state.collapsed),
            (BROKEN, &state.broken),
        ] {
            for at in squares.iter() {
                if let Some(slot) = board.spendable_slot(board.index(*at)) {
                    planes[plane * words + (slot / 64) as usize] |= 1 << (slot % 64);
                }
            }
        }

        let mut latched = 0u8;
        for (group, on) in state.latched.iter().enumerate() {
            if *on {
                latched |= 1 << group;
            }
        }

        let gems: Vec<u32> = state
            .gems
            .iter()
            .map(|spot| match spot {
                GemSpot::Loose(at) => board.index(*at),
                GemSpot::Seated(at) => board.index(*at) | SEATED,
                GemSpot::Held(member) => HELD | *member as u32,
            })
            .collect();

        let mut watchers: Vec<u32> = state.watchers.iter().map(|at| board.index(*at)).collect();
        watchers.sort_unstable();

        Self {
            entities: entities.into_boxed_slice(),
            gems: gems.into_boxed_slice(),
            watchers: watchers.into_boxed_slice(),
            active: state.active as u8,
            latched,
            planes: planes.into_boxed_slice(),
        }
    }

    /// The position back in the shape the rules read. Facing and the two
    /// counters are left at nothing, because no rule asks about them and a
    /// route that is played back builds them again as it goes.
    pub fn spread(&self, board: &Board) -> MapState {
        let words = board.spendable_words();
        let mut latched = [false; GATE_GROUPS];
        for (group, on) in latched.iter_mut().enumerate() {
            *on = self.latched & (1 << group) != 0;
        }
        let mut state = MapState {
            members: self.entities[..board.party]
                .iter()
                .map(|square| board.position(*square))
                .collect(),
            active: self.active as usize,
            facing: Direction::Down,
            crates: self.entities[board.party..]
                .iter()
                .enumerate()
                .map(|(index, raw)| CrateState {
                    at: board.position(raw & !SUNK),
                    sunk: raw & SUNK != 0,
                    kind: board.kinds[index],
                })
                .collect(),
            gems: self
                .gems
                .iter()
                .map(|raw| match *raw {
                    held if held & HELD != 0 => GemSpot::Held((held & !HELD) as usize),
                    seated if seated & SEATED != 0 => {
                        GemSpot::Seated(board.position(seated & !SEATED))
                    }
                    square => GemSpot::Loose(board.position(square)),
                })
                .collect(),
            watchers: self
                .watchers
                .iter()
                .map(|square| board.position(*square))
                .collect(),
            pits_filled: Vec::new(),
            collapsed: Vec::new(),
            latched,
            broken: Vec::new(),
            moves: 0,
            pushes: 0,
        };
        for (slot, at) in board.spendable().iter().enumerate() {
            let word = slot / 64;
            let bit = 1u64 << (slot % 64);
            if self.planes[FILLED * words + word] & bit != 0 {
                state.pits_filled.push(*at);
            }
            if self.planes[DROPPED * words + word] & bit != 0 {
                state.collapsed.push(*at);
            }
            if self.planes[BROKEN * words + word] & bit != 0 {
                state.broken.push(*at);
            }
        }
        state
    }

    /// A position as one number. Two boards that are the same position have to
    /// be the same number and two that are not have to differ. The run of
    /// squares that decides it is built in a buffer the search owns and then
    /// thrown away, because a search holds tens of millions of these and what it
    /// holds has to be a number rather than a list.
    ///
    /// The width is what makes throwing the list away safe. Two different
    /// positions landing on one number would make a search skip a board it had
    /// never seen, and at a hundred and twenty eight bits that will not happen
    /// in a run anybody will sit through.
    pub fn key(&self, board: &Board, scratch: &mut Vec<u64>) -> u128 {
        scratch.clear();
        // The party is not interchangeable the way crates are. Each member is a
        // different character, so the order is the map's order and it is kept.
        scratch.push(self.active as u64);
        scratch.push(self.latched as u64);
        scratch.extend(self.entities[..board.party].iter().map(|at| *at as u64));
        let crates = scratch.len();
        // Sorting is what makes two crates that changed places one position
        // rather than two. That is true of a box and an orb, which do the same
        // thing to everything, and false of a lamp, which is where the light
        // comes from, so the kind rides above the square and the sort bands
        // them.
        scratch.extend(
            self.entities[board.party..]
                .iter()
                .enumerate()
                .map(|(index, at)| *at as u64 | (board.kinds[index].code() << 34)),
        );
        scratch[crates..].sort_unstable();
        scratch.extend(self.gems.iter().map(|spot| *spot as u64));
        scratch.extend(self.watchers.iter().map(|square| *square as u64));
        scratch.extend(self.planes.iter().copied());
        stamp(scratch)
    }
}

/// The same run down two hashers, one of them started somewhere else, which is
/// two readings of it rather than the same reading twice.
pub fn stamp(run: &[u64]) -> u128 {
    let mut low = DefaultHasher::new();
    let mut high = DefaultHasher::new();
    SECOND_READING.hash(&mut high);
    run.hash(&mut low);
    run.hash(&mut high);
    (u128::from(high.finish()) << 64) | u128::from(low.finish())
}
