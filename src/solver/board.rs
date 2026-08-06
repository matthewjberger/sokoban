//! The board read once and turned into numbers. Everything below this line
//! works in square indices rather than positions, because a search asks the
//! same questions about the same squares tens of millions of times and every
//! one of them should already be answered.
//!
//! The lattice is flattened into one run of tiles covering columns, rows and
//! storeys, with a margin wide enough that asking about the square two beyond
//! a wall never falls off the end. Index zero is the square outside the board,
//! which is solid and belongs to nothing, so a neighbour that leaves the
//! lattice lands there rather than needing a test of its own.

use crate::rules::{Direction, crate_run, initial_state, light_lends};
use crate::schema::{
    Abilities, CrateKind, Map, Position, Tile, WinCondition, map_layers, map_positions, map_tile,
};
use std::collections::VecDeque;

/// The square off the edge of everything. Solid, and the answer to any
/// neighbour that leaves the lattice.
pub const OUTSIDE: u32 = 0;

/// No route at all, which is a different answer from a long one and has to
/// survive being added to.
pub const UNREACHABLE: u32 = u32::MAX / 4;

/// How far the lattice is padded past the floors. Two, because the furthest
/// question anything asks is about the square two beyond a crate.
const MARGIN: i32 = 2;

/// How many crates the pinning check will follow at once. Past this it says
/// nothing rather than something, which costs a prune and never an answer. No
/// board comes close.
const MOST_PINNED: usize = 32;

/// The board, precomputed. Nothing here changes while a search runs, which is
/// what lets it be read without a lock, a clone or a second thought.
pub struct Board {
    base_layer: i32,
    storeys: i32,
    origin: (i32, i32),
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
    /// Neighbours by direction, so stepping is an array read.
    pub neighbours: Vec<[u32; 4]>,
    /// Squares a body could stand on or pass through, given every door open and
    /// every hole filled. Water counts, because a trade reaches across it and
    /// somebody who wades walks on it.
    open: Vec<bool>,
    /// Squares that hold a crate in place. Everything solid forever, and water
    /// as well unless somebody can stand in it and shove out of it.
    pins: Vec<bool>,
    /// Squares a crate could still be worked from to some marker.
    reaches_goal: Vec<bool>,
    /// Squares a crate could still be spent from, on a hole, on the water or
    /// through a cracked wall. A crate that can do neither this nor the above is
    /// a crate the board has lost.
    reaches_sink: Vec<bool>,
    /// How many pushes from each square to each marker, on a board with nothing
    /// else on it. Every real board is this board plus obstructions, and an
    /// obstruction only ever costs more, so this never overstates the distance.
    goal_distance: Vec<Vec<u32>>,
    /// Squares whose filled, dropped or broken state a position has to carry.
    /// Everywhere else the board says what is there and nothing can change it.
    spendable: Vec<Position>,
    spendable_slot: Vec<u32>,
    /// What each crate is, in the order the rules list them. A position carries
    /// where they are and this says which is which.
    pub kinds: Vec<CrateKind>,
    pub party: usize,
    pub goals: Vec<u32>,
    pub win: WinCondition,
    /// Whether the board reads the squares a move crossed. Fragile floor and
    /// switches both do, and nothing else does, so a board with neither can be
    /// searched without recording where anything walked.
    pub reads_paths: bool,
    /// Whether walking changes nothing but where the body stands, which is what
    /// lets a search work in pushes rather than in steps.
    pub quiet: bool,
    /// Whether a crate closed in on both axes is really finished with. A drag
    /// or a trade lifts one out, so on those boards it is not.
    pub freezes: bool,
    /// Whether a crate can be got rid of by breaking it. Every other way of
    /// spending one is a square it can be pushed onto, which the reading below
    /// works out from the board; this one is a pair of hands and a boulder, and
    /// no square says anything about it. A board with both is a board where
    /// counting crates against markers proves nothing.
    pub breakable: bool,
}

impl Board {
    pub fn read(map: &Map) -> Self {
        let positions = map_positions(map);
        let mut low = positions.first().map(|at| at.cell).unwrap_or((0, 0));
        let mut high = low;
        for at in &positions {
            low = (low.0.min(at.cell.0), low.1.min(at.cell.1));
            high = (high.0.max(at.cell.0), high.1.max(at.cell.1));
        }
        let origin = (low.0 - MARGIN, low.1 - MARGIN);
        let width = high.0 - low.0 + 1 + MARGIN * 2;
        let height = high.1 - low.1 + 1 + MARGIN * 2;
        let layers = map_layers(map);
        let base_layer = layers.first().copied().unwrap_or(0);
        let storeys = layers.last().copied().unwrap_or(0) - base_layer + 1;
        let count = 1 + (width * height * storeys) as usize;

        let mut board = Self {
            base_layer,
            storeys,
            origin,
            width,
            height,
            tiles: vec![Tile::Void; count],
            neighbours: vec![[OUTSIDE; 4]; count],
            open: vec![false; count],
            pins: vec![true; count],
            reaches_goal: vec![false; count],
            reaches_sink: vec![false; count],
            goal_distance: Vec::new(),
            spendable: Vec::new(),
            spendable_slot: vec![u32::MAX; count],
            kinds: initial_state(map)
                .crates
                .iter()
                .map(|entry| entry.kind)
                .collect(),
            party: map.party_size(),
            goals: Vec::new(),
            win: map.rules.win,
            reads_paths: false,
            quiet: false,
            freezes: false,
            breakable: !map.stones.is_empty()
                && map.rules.stones_break_bare_handed
                && map.latent_abilities().smashes,
        };

        // What anybody could ever do here, counting the powers the light on
        // this board can lend. Everything below claims that some move is not
        // there, and a claim like that made against the smaller set of powers
        // would prune a move that really is.
        let abilities = map.latent_abilities();
        let mut stirred = false;
        for square in 1..count {
            let at = board.position(square as u32);
            let tile = map_tile(map, at);
            board.tiles[square] = tile;
            board.open[square] = tile.open_to_bodies();
            // Water is a wall to a crate and a road to whoever wades, so it
            // only closes an axis while nobody in the party can stand in it.
            board.pins[square] = tile.blocks_forever() && !(abilities.wades && tile == Tile::Water);
            stirred |= tile.stirs_the_board(&map.rules);
            board.reads_paths |= tile.reads_the_trail(&map.rules);
            if tile.records_spending() {
                board.spendable_slot[square] = board.spendable.len() as u32;
                board.spendable.push(at);
            }
            let mut around = [OUTSIDE; 4];
            for (way, direction) in Direction::ALL.iter().enumerate() {
                around[way] = board.index(at.offset(direction.delta()));
            }
            board.neighbours[square] = around;
        }

        board.freezes =
            !abilities.pull && !abilities.swap && !map.rules.crates_push_in_pairs && abilities.push;
        // Walking has to change nothing but where the body stands, and the one
        // body has to be a plain shover, since a drag or a trade moves a crate
        // on an ordinary step and there would be no walk to price. A beam a
        // body can stand in is a beam whose towers answer to where it stands,
        // so a warded body on a lit board is not one of these either.
        // Light that lends is the end of it: what a body can do then depends on
        // where it is standing, so the walk between two shoves is part of the
        // puzzle rather than the space between the parts of it. A gem being
        // seated or lifted moves that light about as well. Spikes are the same
        // argument about a square that is only sometimes deadly.
        board.quiet = map.party_size() == 1
            && abilities.push
            && !abilities.pull
            && !abilities.swap
            && !stirred
            && !light_lends(map)
            && map.gems.is_empty()
            && map.spikes.is_empty()
            // A watcher makes a square deadly by standing near it, which is a
            // thing about where a body walks rather than about where the crates
            // are, so the walk between two shoves is part of the puzzle.
            && map.watchers.is_empty()
            && (map.emitters.is_empty() || !abilities.warded);

        let goals: Vec<u32> = map.goals.iter().map(|at| board.index(*at)).collect();
        board.goals = goals;
        let backward = board.crate_graph(map, abilities);
        board.goal_distance = board
            .goals
            .iter()
            .map(|goal| spread(&backward, &[*goal], count))
            .collect();
        for square in 0..count {
            board.reaches_goal[square] = board
                .goal_distance
                .iter()
                .any(|table| table[square] < UNREACHABLE);
        }
        let sinks: Vec<u32> = (1..count as u32)
            .filter(|square| {
                matches!(
                    board.tiles[*square as usize],
                    Tile::Pit | Tile::Water | Tile::Brittle | Tile::Fragile | Tile::Incinerator
                )
            })
            .collect();
        let sunk = spread(&backward, &sinks, count);
        for (reaches, distance) in board.reaches_sink.iter_mut().zip(sunk.iter()) {
            *reaches = *distance < UNREACHABLE;
        }
        board
    }

    /// Every way a crate could travel, listed backwards so a walk out from the
    /// markers says which squares can still reach one.
    ///
    /// It is drawn on a board with nothing else on it and every door standing
    /// open, so it allows more than any real position does. That is the point.
    /// Another crate in the way can only stop this one sooner, which is why
    /// every square a shove crosses counts as somewhere it could come to rest
    /// rather than only the square it would reach unobstructed.
    fn crate_graph(&self, map: &Map, abilities: Abilities) -> Graph {
        let count = self.tiles.len();
        let mut edges: Vec<(u32, u32)> = Vec::new();
        let mut run = Vec::new();
        for square in 1..count as u32 {
            if !self.open[square as usize] {
                continue;
            }
            let at = self.position(square);
            for direction in Direction::ALL {
                let delta = direction.delta();
                if abilities.push {
                    let behind = self.index(at.offset((-delta.0, -delta.1)));
                    if self.open[behind as usize] {
                        crate_run(map, at, direction, &mut run);
                        for landing in &run {
                            let target = self.index(*landing);
                            if target != OUTSIDE {
                                edges.push((target, square));
                            }
                        }
                    }
                }
                // A drag walks the crate one square into where the dragger was
                // standing, so the crate needs somewhere to go and the dragger
                // needs somewhere beyond it to go to.
                if abilities.pull {
                    let ahead = self.index(at.offset(delta));
                    let beyond = self.index(at.offset((delta.0 * 2, delta.1 * 2)));
                    if self.open[ahead as usize] && self.open[beyond as usize] {
                        edges.push((ahead, square));
                    }
                }
                // A trade reaches down a clear line and lands the crate where
                // the trader was standing, however far back that is. The line
                // is one a body walks, not one a crate is shoved along, so open
                // water is part of it.
                if abilities.swap {
                    let mut cursor = at;
                    loop {
                        cursor = cursor.offset(delta);
                        let target = self.index(cursor);
                        if target == OUTSIDE || !self.open[target as usize] {
                            break;
                        }
                        edges.push((target, square));
                    }
                }
            }
        }
        Graph::gather(edges, count)
    }

    pub fn index(&self, at: Position) -> u32 {
        let layer = at.layer - self.base_layer;
        let column = at.cell.0 - self.origin.0;
        let row = at.cell.1 - self.origin.1;
        if column < 0
            || row < 0
            || layer < 0
            || column >= self.width
            || row >= self.height
            || layer >= self.storeys
        {
            return OUTSIDE;
        }
        1 + ((layer * self.height + row) * self.width + column) as u32
    }

    /// Where a square is. The square off the edge is nowhere, and answering
    /// with a default keeps that from being a subtraction below zero.
    pub fn position(&self, square: u32) -> Position {
        if square == OUTSIDE {
            return Position::default();
        }
        let flat = (square - 1) as i32;
        let column = flat % self.width;
        let row = (flat / self.width) % self.height;
        let layer = flat / (self.width * self.height);
        Position::new(
            self.base_layer + layer,
            (self.origin.0 + column, self.origin.1 + row),
        )
    }

    pub fn tile(&self, square: u32) -> Tile {
        self.tiles[square as usize]
    }

    /// Whether a crate on this square is finished with: no marker it could
    /// reach and no hole it could be spent in.
    pub fn lost(&self, square: u32) -> bool {
        !self.reaches_goal[square as usize] && !self.reaches_sink[square as usize]
    }

    /// Whether a crate on this square could still fill a marker.
    pub fn delivers(&self, square: u32) -> bool {
        self.reaches_goal[square as usize]
    }

    /// Whether a crate on this square could still be spent, in a hole, on the
    /// water or through a cracked wall. A board that wants every crate on a
    /// marker is finished by the crates that survive, so one that can be spent
    /// is not one the board is owed.
    pub fn sinkable(&self, square: u32) -> bool {
        self.reaches_sink[square as usize]
    }

    pub fn goal_distance(&self, goal: usize, square: u32) -> u32 {
        self.goal_distance[goal][square as usize]
    }

    /// Where the record of this square's filled, dropped or broken state lives,
    /// for the squares that have one.
    pub fn spendable_slot(&self, square: u32) -> Option<u32> {
        let slot = self.spendable_slot[square as usize];
        (slot != u32::MAX).then_some(slot)
    }

    pub fn spendable(&self) -> &[Position] {
        &self.spendable
    }

    /// How many machine words a position needs to carry what has been spent.
    pub fn spendable_words(&self) -> usize {
        self.spendable.len().div_ceil(64)
    }

    /// Whether this arrangement of the crates has already lost, given only
    /// where they are standing. Two things end a board: a crate somewhere it
    /// can never be worked out of, and not enough crates left that could still
    /// reach a marker.
    ///
    /// `live` is the squares of the crates still on the board, spent ones left
    /// out.
    pub fn stuck(&self, live: &[u32]) -> bool {
        // A board where a crate can be broken is a board where a crate off a
        // marker is not yet a crate the board has lost, and nothing here can
        // tell which of them is the boulder.
        if self.breakable {
            return false;
        }
        let mut frozen = [false; MOST_PINNED];
        if self.freezes {
            for index in 0..live.len().min(frozen.len()) {
                frozen[index] = self.frozen(live, index, 0);
            }
        }
        let pinned = |index: usize| frozen.get(index).copied().unwrap_or(false);
        match self.win {
            WinCondition::CratesOnGoals => live.iter().enumerate().any(|(index, square)| {
                !self.goals.contains(square) && (self.lost(*square) || pinned(index))
            }),
            WinCondition::GoalsCovered => {
                let usable = live
                    .iter()
                    .enumerate()
                    .filter(|(index, square)| {
                        self.delivers(**square) && (self.goals.contains(square) || !pinned(*index))
                    })
                    .count();
                usable < self.goals.len()
            }
            // A board finished by seating gems is owed nothing by its crates,
            // so no arrangement of them has lost it.
            WinCondition::SocketsFilled => false,
        }
    }

    /// Whether this crate can never move again. A crate held on both axes is
    /// finished, and what holds it is a wall that will never open or another
    /// crate that is itself finished. The crate being asked about counts as a
    /// wall while its own answer is worked out, so two crates propping each
    /// other up come out as the pair of stuck crates they are.
    ///
    /// `visiting` is the crates already on the chain, one bit each, so nothing
    /// is allocated to answer this.
    fn frozen(&self, live: &[u32], index: usize, visiting: u32) -> bool {
        if index >= MOST_PINNED {
            return false;
        }
        if visiting & (1 << index) != 0 {
            return true;
        }
        let visiting = visiting | (1 << index);
        let square = live[index];
        self.held(live, square, 0, visiting) && self.held(live, square, 2, visiting)
    }

    /// Whether the crate is held along one axis, named by the first of its two
    /// directions in [`Direction::ALL`].
    fn held(&self, live: &[u32], square: u32, axis: usize, visiting: u32) -> bool {
        let around = self.neighbours[square as usize];
        for way in [axis, axis + 1] {
            if self.pins[around[way] as usize] {
                return true;
            }
        }
        for way in [axis, axis + 1] {
            if let Some(other) = live.iter().position(|at| *at == around[way])
                && self.frozen(live, other, visiting)
            {
                return true;
            }
        }
        false
    }
}

/// Every way a crate could travel, listed backwards and laid out flat: one run
/// of sources per square, and one index saying where each square's run starts.
/// A list of lists would be one allocation per square, and this reading is
/// taken again for every candidate board a generator throws away.
struct Graph {
    starts: Vec<u32>,
    sources: Vec<u32>,
}

impl Graph {
    fn gather(mut edges: Vec<(u32, u32)>, count: usize) -> Self {
        edges.sort_unstable();
        let mut starts = vec![0u32; count + 1];
        for (target, _) in &edges {
            starts[*target as usize + 1] += 1;
        }
        for square in 0..count {
            starts[square + 1] += starts[square];
        }
        Self {
            sources: edges.into_iter().map(|(_, source)| source).collect(),
            starts,
        }
    }

    fn reaching(&self, square: u32) -> &[u32] {
        let from = self.starts[square as usize] as usize;
        let to = self.starts[square as usize + 1] as usize;
        &self.sources[from..to]
    }
}

/// How far every square is from the nearest of these, walking the graph
/// backwards.
fn spread(backward: &Graph, sources: &[u32], count: usize) -> Vec<u32> {
    let mut distance = vec![UNREACHABLE; count];
    let mut queue = VecDeque::new();
    for source in sources {
        if distance[*source as usize] == UNREACHABLE {
            distance[*source as usize] = 0;
            queue.push_back(*source);
        }
    }
    while let Some(node) = queue.pop_front() {
        let step = distance[node as usize] + 1;
        for from in backward.reaching(node) {
            if distance[*from as usize] == UNREACHABLE {
                distance[*from as usize] = step;
                queue.push_back(*from);
            }
        }
    }
    distance
}
