//! Shove to shove, with the walking priced in. On a board where walking
//! changes nothing but where the body stands, everywhere the player can get to
//! without touching a crate is one place as far as the puzzle is concerned, and
//! the only moves worth putting in a search are the shoves. So a position here
//! is the crates plus the square the last shove left the player on, an edge is
//! one shove, and what that edge costs is the walk to the shoving square plus
//! the shove itself.
//!
//! Counting the walk is what keeps the answer meaning what it meant before.
//! A search that counted shoves would find the fewest shoves, which is a
//! different number from par, so this is a shortest path by move count with the
//! walking worked out on demand rather than searched.
//!
//! The bound that guides it is the cheapest way to give every marker a crate of
//! its own, measured in shoves on a board with nothing else on it. Every shove
//! costs at least the one move that makes it and an empty board is never
//! further than a crowded one, so the bound is never an overstatement, which is
//! the only property that matters, since an overstatement would quietly
//! shorten par.

use super::board::{Board, OUTSIDE, UNREACHABLE};
use super::packed::Packed;
use super::{Progress, Straight};
use crate::rules::{
    Detail, Direction, GATE_GROUPS, MapState, Step, attempt_move_with, beam_field, deadlocked,
    gate_flags, initial_state, lethal, map_solved,
};
use crate::schema::{Abilities, Map, Tile, WinCondition};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, VecDeque};

/// Beyond any real cost, and far enough below the top of the range that adding
/// to it never wraps.
const FAR: i64 = 1 << 40;

/// How many markers and crates the bound will match up. Past this the matching
/// costs more than it saves, and no bound at all is still a correct one.
const MOST_MATCHED: usize = 16;

struct PushNode {
    state: MapState,
    parent: u32,
    /// The square the player shoved from, which is what the walk has to reach.
    landing: u32,
    direction: Direction,
    key: u128,
}

pub struct PushSpace {
    nodes: Vec<PushNode>,
    /// Ordered by the walk so far plus the bound on what is left, then by the
    /// walk alone, then by the order they were found in, so two runs over one
    /// board expand the same positions in the same order.
    heap: BinaryHeap<Reverse<(u32, u32, u32)>>,
    best: HashMap<u128, u32, Straight>,
    /// Everywhere the player can walk to from the position being expanded, and
    /// how it got there. Stamped rather than cleared, because clearing a board
    /// sized buffer once per position costs more than the walk does.
    walk_stamp: Vec<u32>,
    walk_distance: Vec<u32>,
    walk_from: Vec<(u32, Direction)>,
    /// Squares the light is covering right now, and squares a crate is standing
    /// on, stamped the same way.
    lit: Vec<u32>,
    occupied: Vec<u32>,
    generation: u32,
    region: usize,
    /// What the gates are doing with nobody standing anywhere, which is
    /// everything but the plate the player happens to be on.
    base: [bool; GATE_GROUPS],
    queue: VecDeque<u32>,
    live: Vec<u32>,
    scratch: Vec<u64>,
    /// What the one body on this board can do. Push space is only handed
    /// boards with a party of one, and this is where that is relied on.
    powers: Abilities,
    /// How much of a move to work out, which on these boards is never more than
    /// the board that came out.
    detail: Detail,
    explored: usize,
    budget: usize,
    finished: bool,
}

impl PushSpace {
    pub fn new(map: &Map, board: &Board, budget: usize) -> Self {
        let squares = board.neighbours.len();
        let start = initial_state(map);
        let finished = map_solved(map, &start);
        let mut scratch = Vec::new();
        let key = Packed::read(board, &start).key(board, &mut scratch);
        let mut best = HashMap::default();
        best.insert(key, 0);
        let mut heap = BinaryHeap::new();
        if !finished {
            heap.push(Reverse((0, 0, 0)));
        }
        Self {
            nodes: vec![PushNode {
                state: start,
                parent: 0,
                landing: OUTSIDE,
                direction: Direction::Up,
                key,
            }],
            heap,
            best,
            walk_stamp: vec![0; squares],
            walk_distance: vec![0; squares],
            walk_from: vec![(OUTSIDE, Direction::Up); squares],
            lit: vec![0; squares],
            occupied: vec![0; squares],
            generation: 0,
            region: 0,
            base: [false; GATE_GROUPS],
            queue: VecDeque::new(),
            live: Vec::new(),
            scratch,
            powers: map.member_character(0).abilities(),
            detail: Detail::Position {
                paths: board.reads_paths,
            },
            explored: 0,
            budget,
            finished,
        }
    }

    pub fn explored(&self) -> usize {
        self.explored
    }

    pub fn advance(&mut self, map: &Map, board: &Board, slice: usize) -> Progress {
        if self.finished {
            return Progress::Solved(Vec::new());
        }
        let mut worked = 0usize;
        while worked < slice {
            let Some(Reverse((_, cost, index))) = self.heap.pop() else {
                return Progress::Unsolvable;
            };
            // A position found again by a cheaper route leaves the dearer entry
            // in the heap, and this is where it is thrown away.
            if self.best.get(&self.nodes[index as usize].key) != Some(&cost) {
                worked += 1;
                continue;
            }
            let state = self.nodes[index as usize].state.clone();
            if map_solved(map, &state) {
                return Progress::Solved(self.route(map, board, index));
            }
            self.explored += 1;
            if self.explored > self.budget {
                return Progress::Exhausted;
            }
            self.survey(map, board, &state);
            worked += 1 + self.region;
            self.explored += self.region;
            self.shove(map, board, &state, index, cost);
        }
        Progress::Running
    }

    /// Every shove available from here, each one priced at the walk it takes to
    /// get behind the crate.
    fn shove(&mut self, map: &Map, board: &Board, state: &MapState, index: u32, cost: u32) {
        let detail = self.detail;
        let mut probe = state.clone();
        for entry in state.crates.iter().filter(|entry| !entry.sunk) {
            for direction in Direction::ALL {
                let delta = direction.delta();
                let standing = entry.at.offset((-delta.0, -delta.1));
                let square = board.index(standing);
                if square == OUTSIDE || self.walk_stamp[square as usize] != self.generation {
                    continue;
                }
                probe.members[0] = standing;
                let flags = self.flags_at(map, board, square);
                let Some(outcome) =
                    attempt_move_with(map, &probe, direction, &flags, self.powers, detail)
                else {
                    continue;
                };
                if outcome.state.crates == probe.crates {
                    continue;
                }
                if lethal(map, &outcome.state) || deadlocked(map, &outcome.state) {
                    continue;
                }
                self.live.clear();
                self.live.extend(
                    outcome
                        .state
                        .crates
                        .iter()
                        .filter(|entry| !entry.sunk)
                        .map(|entry| board.index(entry.at)),
                );
                if board.stuck(&self.live) {
                    continue;
                }
                let Some(bound) = estimate(board, &self.live) else {
                    continue;
                };
                let reached = cost + self.walk_distance[square as usize] + 1;
                let key = Packed::read(board, &outcome.state).key(board, &mut self.scratch);
                if self.best.get(&key).is_some_and(|held| *held <= reached) {
                    continue;
                }
                self.best.insert(key, reached);
                self.nodes.push(PushNode {
                    state: outcome.state,
                    parent: index,
                    landing: square,
                    direction,
                    key,
                });
                self.heap.push(Reverse((
                    reached.saturating_add(bound),
                    reached,
                    (self.nodes.len() - 1) as u32,
                )));
            }
        }
    }

    /// Everywhere the player can walk to without touching a crate, and how many
    /// steps each of them takes. The light is traced first, because a square a
    /// beam covers is a square nobody unwarded walks out of, and on these
    /// boards the light answers to the crates alone.
    fn survey(&mut self, map: &Map, board: &Board, state: &MapState) {
        self.generation += 1;
        let mut ghost = state.clone();
        ghost.members.clear();
        self.base = gate_flags(map, &ghost);
        if !map.emitters.is_empty() {
            for at in beam_field(map, &ghost).covered {
                let square = board.index(at);
                if square != OUTSIDE {
                    self.lit[square as usize] = self.generation;
                }
            }
        }

        for entry in state.crates.iter().filter(|entry| !entry.sunk) {
            let square = board.index(entry.at);
            if square != OUTSIDE {
                self.occupied[square as usize] = self.generation;
            }
        }

        let detail = self.detail;
        // Stepping through a wall or over a gap lands somewhere other than the
        // square ahead, so only a body without either can be told in advance
        // where its step ends.
        let lands_ahead = !self.powers.phasing && !self.powers.blinks;
        let start = board.index(state.player());
        self.region = 0;
        self.queue.clear();
        if start == OUTSIDE {
            return;
        }
        self.walk_stamp[start as usize] = self.generation;
        self.walk_distance[start as usize] = 0;
        self.region = 1;
        self.queue.push_back(start);
        let mut probe = state.clone();
        while let Some(square) = self.queue.pop_front() {
            let at = board.position(square);
            probe.members[0] = at;
            let flags = self.flags_at(map, board, square);
            for (way, direction) in Direction::ALL.into_iter().enumerate() {
                // A crate ahead is a shove, which is an edge of the search
                // rather than part of the walk, and a square already reached is
                // one this would only reach again. Working either of them out
                // is the expensive part.
                let ahead = board.neighbours[square as usize][way];
                if self.occupied[ahead as usize] == self.generation
                    || (lands_ahead && self.walk_stamp[ahead as usize] == self.generation)
                {
                    continue;
                }
                let Some(outcome) =
                    attempt_move_with(map, &probe, direction, &flags, self.powers, detail)
                else {
                    continue;
                };
                // Anything that moved a crate was a shove, and shoves are the
                // edges of the search rather than part of the walk.
                if outcome.state.crates != probe.crates {
                    continue;
                }
                let landing = board.index(outcome.state.player());
                if landing == OUTSIDE || self.walk_stamp[landing as usize] == self.generation {
                    continue;
                }
                if self.deadly(board, landing) {
                    continue;
                }
                self.walk_stamp[landing as usize] = self.generation;
                self.walk_distance[landing as usize] = self.walk_distance[square as usize] + 1;
                self.walk_from[landing as usize] = (square, direction);
                self.region += 1;
                self.queue.push_back(landing);
            }
        }
    }

    /// What the gates are doing with the player standing here. A plate under
    /// their feet is the one thing that changes with where they are, and a gate
    /// is answered on the way in against what was true before the step, so
    /// stepping plate then gate then beyond is a walk the board allows.
    fn flags_at(&self, map: &Map, board: &Board, square: u32) -> [bool; GATE_GROUPS] {
        let mut flags = self.base;
        if map.rules.plates_sense_player
            && let Tile::Plate(group) = board.tile(square)
            && let Some(held) = flags.get_mut(group as usize)
        {
            *held = true;
        }
        flags
    }

    /// Whether standing here kills whoever is playing. The party is one, so
    /// nobody can be shielded and nobody else has to be asked about.
    fn deadly(&self, board: &Board, square: u32) -> bool {
        if !self.powers.wades && board.tile(square) == Tile::Water {
            return true;
        }
        !self.powers.warded && self.lit[square as usize] == self.generation
    }

    /// The route back out, as the steps a player would make. Each shove is
    /// preceded by the walk to the square it was made from, worked out again
    /// from the position it was made in rather than remembered, because
    /// remembering it would cost every position in the search what only the
    /// answer needs.
    fn route(&mut self, map: &Map, board: &Board, found: u32) -> Vec<Step> {
        // A shove is always found after the position it was made from, so the
        // chain of parents runs downwards and ends. So does the walk back
        // through the region, a step nearer the start each time.
        let mut chain = Vec::new();
        let mut cursor = found;
        while cursor != 0 {
            let node = &self.nodes[cursor as usize];
            chain.push((node.parent, node.landing, node.direction));
            cursor = node.parent;
        }
        chain.reverse();

        let mut steps = Vec::new();
        for (parent, landing, direction) in chain {
            let state = self.nodes[parent as usize].state.clone();
            self.survey(map, board, &state);
            let start = board.index(state.player());
            let mut back = Vec::new();
            let mut square = landing;
            while square != start {
                let (previous, way) = self.walk_from[square as usize];
                back.push(Step::Go(way));
                square = previous;
            }
            back.reverse();
            steps.extend(back);
            steps.push(Step::Go(direction));
        }
        steps
    }
}

/// The fewest shoves the board still owes, as the cheapest way to give
/// everything that wants one a square of its own. Nothing here is a guess, and
/// an assignment that cannot be made at all is a position that can never be
/// finished, which is worth more than the bound is.
///
/// Filling every marker means every marker wants a crate. Placing every crate
/// means every crate wants a marker, except one with a hole still in reach,
/// which never has to survive to want one.
pub(super) fn estimate(board: &Board, live: &[u32]) -> Option<u32> {
    let goals = board.goals.len();
    if goals == 0 || goals > MOST_MATCHED || live.len() > MOST_MATCHED {
        return Some(0);
    }
    // A crate that can be broken owes no marker anything, and which of them is
    // the boulder is not something a run of squares can say, so a board with a
    // pair of hands on it gets no bound rather than a wrong one.
    if board.breakable {
        return Some(0);
    }
    // Nothing is owed by the crates on a board won by seating gems, and a bound
    // is only ever a bound on what is owed.
    let strict = match board.win {
        WinCondition::CratesOnGoals => true,
        WinCondition::GoalsCovered => false,
        WinCondition::SocketsFilled => return Some(0),
    };
    let mut owed = [0u32; MOST_MATCHED];
    let mut owed_count = 0;
    for square in live {
        if strict && board.sinkable(*square) {
            continue;
        }
        owed[owed_count] = *square;
        owed_count += 1;
    }
    let (rows, columns) = if strict {
        (owed_count, goals)
    } else {
        (goals, owed_count)
    };
    if rows == 0 {
        return Some(0);
    }
    // More to place than there are places for it can never be finished,
    // whichever way round the question was asked.
    if rows > columns {
        return None;
    }
    let mut cost = [FAR; MOST_MATCHED * MOST_MATCHED];
    for row in 0..rows {
        for column in 0..columns {
            let (goal, square) = if strict {
                (column, owed[row])
            } else {
                (row, owed[column])
            };
            let reach = board.goal_distance(goal, square);
            if reach < UNREACHABLE {
                cost[row * columns + column] = i64::from(reach);
            }
        }
    }
    assign(&cost, rows, columns).map(|total| total as u32)
}

/// The cheapest one to one assignment of rows to columns, by the usual method
/// of keeping a price on each and walking augmenting paths at zero reduced
/// cost. Nothing here is large, since a board has a handful of markers and a
/// handful of crates.
fn assign(cost: &[i64], rows: usize, columns: usize) -> Option<i64> {
    if rows == 0 {
        return Some(0);
    }
    let mut row_price = [0i64; MOST_MATCHED + 1];
    let mut column_price = [0i64; MOST_MATCHED + 1];
    let mut taken = [0usize; MOST_MATCHED + 1];
    let mut came_from = [0usize; MOST_MATCHED + 1];
    for row in 1..=rows {
        taken[0] = row;
        let mut column = 0usize;
        let mut cheapest = [FAR; MOST_MATCHED + 1];
        let mut used = [false; MOST_MATCHED + 1];
        loop {
            used[column] = true;
            let holder = taken[column];
            let mut delta = FAR;
            let mut next = 0usize;
            for candidate in 1..=columns {
                if used[candidate] {
                    continue;
                }
                let value = cost[(holder - 1) * columns + candidate - 1]
                    - row_price[holder]
                    - column_price[candidate];
                if value < cheapest[candidate] {
                    cheapest[candidate] = value;
                    came_from[candidate] = column;
                }
                if cheapest[candidate] < delta {
                    delta = cheapest[candidate];
                    next = candidate;
                }
            }
            if next == 0 {
                return None;
            }
            for candidate in 0..=columns {
                if used[candidate] {
                    row_price[taken[candidate]] += delta;
                    column_price[candidate] -= delta;
                } else {
                    cheapest[candidate] -= delta;
                }
            }
            column = next;
            if taken[column] == 0 {
                break;
            }
        }
        loop {
            let previous = came_from[column];
            taken[column] = taken[previous];
            column = previous;
            if column == 0 {
                break;
            }
        }
    }
    let total = -column_price[0];
    (total < FAR / 2).then_some(total)
}
