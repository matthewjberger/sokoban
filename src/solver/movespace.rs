//! One move at a time, breadth first. This is the engine for every board where
//! walking is part of the puzzle rather than the space between the parts of it:
//! floor that drops away behind you, ice that carries you past where you meant
//! to stop, a switch thrown by walking over it, a party to point the controls
//! at. On those boards two positions with the crates in the same places and the
//! body somewhere else are genuinely two positions, and the only honest way to
//! count moves is to make them.
//!
//! Breadth first is what makes the first answer the shortest one. Every edge
//! costs one move, so the first time a finished board turns up it has turned up
//! at the fewest moves that can reach it, and that is what par means.

use super::board::Board;
use super::packed::Packed;
use super::{Progress, ROOT, Straight, retrace};
use crate::rules::{Detail, Step, deadlocked, expand, initial_state, lethal, map_solved};
use crate::schema::Map;
use std::collections::{HashSet, VecDeque};
use std::ops::ControlFlow;

pub struct MoveSpace {
    seen: HashSet<u128, Straight>,
    /// The step that reached each position and the position above it. Holding
    /// these rather than whole boards means remembering the route costs a few
    /// bytes a position instead of a whole one.
    nodes: Vec<(u32, Step)>,
    frontier: VecDeque<(Packed, u32)>,
    /// Buffers the walk reuses rather than allocating one of each per position.
    scratch: Vec<u64>,
    live: Vec<u32>,
    detail: Detail,
    explored: usize,
    budget: usize,
    /// The board was already finished when the search was handed it, which is
    /// an answer rather than something to walk for.
    finished: bool,
}

impl MoveSpace {
    pub fn new(map: &Map, board: &Board, budget: usize) -> Self {
        let start = initial_state(map);
        let finished = map_solved(map, &start);
        let packed = Packed::read(board, &start);
        let mut scratch = Vec::new();
        let mut seen = HashSet::default();
        seen.insert(packed.key(board, &mut scratch));
        let mut frontier = VecDeque::new();
        if !finished {
            frontier.push_back((packed, ROOT));
        }
        Self {
            seen,
            nodes: Vec::new(),
            frontier,
            scratch,
            live: Vec::new(),
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
        for _ in 0..slice {
            let Some((packed, parent)) = self.frontier.pop_front() else {
                return Progress::Unsolvable;
            };
            self.explored += 1;
            if self.explored > self.budget {
                return Progress::Exhausted;
            }
            let state = packed.spread(board);
            // Pointing the controls at somebody and immediately pointing them
            // at somebody else costs two moves to do what the second one does
            // on its own, so no shortest route ever contains the pair.
            let repeated = parent != ROOT && matches!(self.nodes[parent as usize].1, Step::Take(_));
            let detail = self.detail;

            let MoveSpace {
                seen,
                nodes,
                frontier,
                scratch,
                live,
                ..
            } = self;
            let mut reached = None;
            expand(map, &state, detail, |step, outcome| {
                if repeated && matches!(step, Step::Take(_)) {
                    return ControlFlow::Continue(());
                }
                if map_solved(map, &outcome.state) {
                    reached = Some(step);
                    return ControlFlow::Break(());
                }
                if lethal(map, &outcome.state) || deadlocked(map, &outcome.state) {
                    return ControlFlow::Continue(());
                }
                live.clear();
                live.extend(
                    outcome
                        .state
                        .crates
                        .iter()
                        .filter(|entry| !entry.sunk)
                        .map(|entry| board.index(entry.at)),
                );
                if board.stuck(live) {
                    return ControlFlow::Continue(());
                }
                let child = Packed::read(board, &outcome.state);
                if !seen.insert(child.key(board, scratch)) {
                    return ControlFlow::Continue(());
                }
                nodes.push((parent, step));
                frontier.push_back((child, (nodes.len() - 1) as u32));
                ControlFlow::Continue(())
            });

            if let Some(step) = reached {
                let mut route = retrace(&self.nodes, parent);
                route.push(step);
                return Progress::Solved(route);
            }
        }
        Progress::Running
    }
}
