//! Whether a puzzle has anything in it. A board can be solvable and still be
//! nothing to solve, and the difference is not a matter of taste. It is in the
//! shape of the move graph, which is already built to answer whether the board
//! can be finished at all.
//!
//! Two things make a puzzle. Either the route goes backwards before it goes
//! forwards, so the obvious move is the wrong one, or there is a moment with
//! one right answer among several that looked available. A board with neither
//! is a walk with extra steps, and this says so rather than leaving it to be
//! noticed by whoever plays it.

use crate::rules::{MapState, deadlocked, expansions, initial_state, lethal, map_solved, play};
use crate::schema::{Map, Position};
use crate::shortcut::{Skipped, skipped};
use crate::solver::{search_key, solve_path};
use std::collections::{HashMap, HashSet, VecDeque};

/// How many states the analysis will hold before it gives up. The graph is kept
/// this time rather than walked and dropped, so this is lower than the search's
/// own budget and the honest answer on a board too big is that it does not know.
pub const INSIGHT_BUDGET: usize = 1_200_000;

/// What the move graph says about a board.
#[derive(Clone, Debug)]
pub struct Insight {
    /// The shortest solution, in moves.
    pub depth: usize,
    pub explored: usize,
    /// Whether the whole graph was walked. Nothing below can be trusted without
    /// this, since every count is over what was seen.
    pub complete: bool,
    /// Reachable positions that can no longer be finished. A board with none of
    /// these cannot be got wrong, only got slowly.
    pub traps: usize,
    /// Moments where the shortest route has exactly one square to be on, while
    /// other squares were reachable in the same number of moves. This is the
    /// a-ha in its measurable form, where several things looked possible and
    /// one was.
    pub pivots: usize,
    /// Moves in the solution that leave the board looking worse than before,
    /// which is the other kind of a-ha, where the crate has to go the wrong way
    /// first.
    pub regressions: usize,
    /// Whether the obvious play finishes the board on its own. This is weaker
    /// evidence than it sounds, because always making the best available shove
    /// is a decent solver, and a board with plenty to get wrong can still happen to
    /// survive it. It only condemns a board that has nothing else in it.
    pub greedy: bool,
    /// Whether the obvious play walks into a position the board cannot be
    /// finished from. This is the good kind of trap, where the move that looks
    /// like progress is the one that ends the puzzle.
    pub garden_path: bool,
    /// Crates the solution has to move somewhere that is not a marker. A crate
    /// spent filling a hole, breaking a wall, holding a plate or standing in a
    /// beam is a side job that has to be done before the deliveries can be, and
    /// counting them is how a board that is only deliveries gets noticed.
    pub enablers: usize,
    /// What the shortest solution never had to do. A board with anything here
    /// has a way round part of itself, which is the plainest fault a puzzle can
    /// have and the one that survives every other check.
    pub skipped: Skipped,
}

/// What the board amounts to, once the graph has been read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// There is something to work out.
    Insight,
    /// Solvable, and nothing in it that the obvious play would not find.
    Thin,
    /// The obvious play finishes it without ever looking wrong.
    Obvious,
    Unsolvable,
    /// Too big to decide inside the budget.
    Undecided,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Insight => "insight",
            Self::Thin => "THIN",
            Self::Obvious => "OBVIOUS",
            Self::Unsolvable => "UNSOLVABLE",
            Self::Undecided => "undecided",
        }
    }
}

impl Insight {
    pub fn verdict(&self) -> Verdict {
        if !self.complete {
            return Verdict::Undecided;
        }
        if self.depth == 0 {
            return Verdict::Unsolvable;
        }
        // Structure first. A board where one shove out of several is the right
        // one, or where something has to go the wrong way, has something in it
        // whether or not a heuristic happened to thread it.
        if self.pivots > 0 || self.regressions > 0 || self.garden_path {
            return Verdict::Insight;
        }
        if self.greedy {
            return Verdict::Obvious;
        }
        Verdict::Thin
    }

    /// The one line worth printing per board.
    pub fn describe(&self) -> String {
        if !self.complete {
            return format!("undecided after {} states", self.explored);
        }
        format!(
            "{:8}  {:>3} moves  {:>2} pivots  {:>2} back  {:>2} side  {:>11}  {:>8}  {:>6} traps{}",
            self.verdict().label(),
            self.depth,
            self.pivots,
            self.regressions,
            self.enablers,
            if self.garden_path { "garden path" } else { "" },
            if self.greedy { "yields" } else { "" },
            self.traps,
            if self.skipped.is_empty() {
                String::new()
            } else {
                format!("   skips {}", self.skipped.describe())
            },
        )
    }
}

/// How far one square is from another, for a crate. Another storey is further
/// away than anything on this one, by more than any walk across a floor.
fn reach(from: Position, to: Position) -> i32 {
    let storeys = (from.layer - to.layer).abs() * 64;
    storeys + (from.cell.0 - to.cell.0).abs() + (from.cell.1 - to.cell.1).abs()
}

/// What a board looks like it has left to do. Every marker is matched with the
/// crate that will fill it and the distances added up, taking the best matching
/// there is.
///
/// The matching is the part that matters. Scoring each crate against whichever
/// marker happens to be nearest it reads a board with two markers and two crates
/// as though either crate would do for either marker, which is how a player who
/// has understood nothing would read it, but it also means a crate being shoved
/// towards the marker it is actually for can look like it is getting worse. So
/// this asks the question a player asks, which is which crate is for which
/// marker and how far each is from the one it is for.
fn distance(map: &Map, state: &MapState) -> i32 {
    let crates: Vec<Position> = state
        .crates
        .iter()
        .filter(|entry| !entry.sunk)
        .map(|entry| entry.at)
        .collect();

    // Enumerating the matchings is only cheap while there are few of them. Past
    // that the nearest marker per crate is the honest approximation, and no board
    // in the campaign is anywhere near the limit.
    if map.goals.len() > 6 || crates.len() > 8 {
        return crates
            .iter()
            .map(|at| {
                map.goals
                    .iter()
                    .map(|goal| reach(*at, *goal))
                    .min()
                    .unwrap_or(0)
            })
            .sum();
    }

    let mut taken = vec![false; crates.len()];
    matched(&crates, &map.goals, 0, &mut taken)
}

/// The cheapest way to give every remaining marker a crate of its own. A board
/// with fewer crates left than markers cannot be finished, and saying so with a
/// large number is what stops a crate being sunk for the look of it.
fn matched(crates: &[Position], goals: &[Position], goal: usize, taken: &mut Vec<bool>) -> i32 {
    if goal >= goals.len() {
        return 0;
    }
    let mut best = None;
    for (slot, at) in crates.iter().enumerate() {
        if taken[slot] {
            continue;
        }
        taken[slot] = true;
        let cost = reach(*at, goals[goal]) + matched(crates, goals, goal + 1, taken);
        taken[slot] = false;
        best = Some(best.map_or(cost, |top: i32| top.min(cost)));
    }
    best.unwrap_or(4096)
}

/// Where the obvious play goes, and whether it gets there.
///
/// The obvious play is not "take the move that helps most", because most moves
/// are walking and walking never helps. A player walks wherever they need to and
/// then makes the shove that looks best from there. So each step here is a search
/// over everywhere the player can walk to without touching a crate, and then the
/// one shove out of all of them that leaves the board looking closest to
/// finished. Stopping when no shove improves it is what somebody does when they
/// have run out of ideas, and where that leaves them is the point. Finishing
/// means there was nothing to work out, and ending up somewhere the board can no
/// longer be finished from means the board taught something.
struct Obvious {
    /// Every position the play committed to, which is the run of boards after
    /// each shove rather than every square walked over on the way.
    visited: Vec<MapState>,
    finished: bool,
}

fn obvious_play(map: &Map) -> Obvious {
    let mut state = initial_state(map);
    let mut visited = vec![state.clone()];
    let mut score = distance(map, &state);
    // A shove that improves the board can only happen so many times before the
    // board is finished, so this bound is never what ends a play that was going
    // to get there.
    for _ in 0..512 {
        if map_solved(map, &state) {
            return Obvious {
                visited,
                finished: true,
            };
        }

        let mut best: Option<(i32, MapState)> = None;
        let mut walked = HashSet::new();
        walked.insert(search_key(&state));
        let mut reach = VecDeque::new();
        reach.push_back(state.clone());
        while let Some(current) = reach.pop_front() {
            for (_, outcome) in expansions(map, &current) {
                if lethal(map, &outcome.state) {
                    continue;
                }
                // Anything that leaves the crates where they were is walking,
                // and walking is free, so a player does as much of it as they
                // need to before deciding anything.
                if outcome.state.crates == current.crates {
                    if walked.insert(search_key(&outcome.state)) {
                        reach.push_back(outcome.state);
                    }
                    continue;
                }
                if map_solved(map, &outcome.state) {
                    return Obvious {
                        visited,
                        finished: true,
                    };
                }
                let candidate = distance(map, &outcome.state);
                if candidate < score && best.as_ref().is_none_or(|(top, _)| candidate < *top) {
                    best = Some((candidate, outcome.state));
                }
            }
        }

        let Some((candidate, next)) = best else {
            return Obvious {
                visited,
                finished: false,
            };
        };
        score = candidate;
        state = next;
        visited.push(state.clone());
    }
    Obvious {
        visited,
        finished: false,
    }
}

/// Walks the whole move graph and keeps it, which is what separates this from
/// the search. The search wants one route out, and this wants to know what else
/// was there.
pub fn insight(map: &Map, budget: usize) -> Insight {
    let mut report = Insight {
        depth: 0,
        explored: 0,
        complete: true,
        traps: 0,
        pivots: 0,
        regressions: 0,
        greedy: false,
        garden_path: false,
        enablers: 0,
        skipped: Skipped::default(),
    };

    let start = initial_state(map);
    let mut index_of: HashMap<u128, u32> = HashMap::new();
    let mut depth: Vec<u32> = Vec::new();
    let mut edges: Vec<Vec<u32>> = Vec::new();
    let mut won: Vec<bool> = Vec::new();

    index_of.insert(search_key(&start), 0);
    depth.push(0);
    edges.push(Vec::new());
    won.push(map_solved(map, &start));

    let mut queue = VecDeque::new();
    queue.push_back((start, 0u32));
    while let Some((state, index)) = queue.pop_front() {
        if depth.len() > budget {
            report.complete = false;
            break;
        }
        for (_, outcome) in expansions(map, &state) {
            let key = search_key(&outcome.state);
            let child = match index_of.get(&key) {
                Some(child) => *child,
                None => {
                    let child = depth.len() as u32;
                    index_of.insert(key, child);
                    depth.push(depth[index as usize] + 1);
                    edges.push(Vec::new());
                    let finished = map_solved(map, &outcome.state);
                    won.push(finished);
                    // A finished board is the end of its line, and so is one
                    // that has been ruined or walked into the light. All three
                    // are worth counting and none is worth expanding.
                    if !finished && !deadlocked(map, &outcome.state) && !lethal(map, &outcome.state)
                    {
                        queue.push_back((outcome.state, child));
                    }
                    child
                }
            };
            edges[index as usize].push(child);
        }
    }
    report.explored = depth.len();
    if !report.complete {
        return report;
    }

    let Some(best) = (0..depth.len())
        .filter(|node| won[*node])
        .map(|node| depth[node])
        .min()
    else {
        return report;
    };
    report.depth = best as usize;

    // Which positions can still be finished. Walking the edges backwards from
    // every finished board marks them all at once, and what is left unmarked is
    // every way there is to ruin the puzzle.
    let mut backward: Vec<Vec<u32>> = vec![Vec::new(); depth.len()];
    for (node, children) in edges.iter().enumerate() {
        for child in children {
            backward[*child as usize].push(node as u32);
        }
    }
    let mut alive = vec![false; depth.len()];
    let mut pending: VecDeque<u32> = (0..depth.len() as u32)
        .filter(|node| won[*node as usize])
        .inspect(|node| alive[*node as usize] = true)
        .collect();
    while let Some(node) = pending.pop_front() {
        for parent in &backward[node as usize] {
            if !alive[*parent as usize] {
                alive[*parent as usize] = true;
                pending.push_back(*parent);
            }
        }
    }
    report.traps = alive.iter().filter(|reaches| !**reaches).count();

    // Which positions a shortest solution can pass through. A position is on one
    // when it is a finished board at the shortest depth, or a move from a
    // position that is.
    let mut on_route = vec![false; depth.len()];
    let mut order: Vec<u32> = (0..depth.len() as u32).collect();
    order.sort_unstable_by_key(|node| std::cmp::Reverse(depth[*node as usize]));
    for node in order {
        let at = node as usize;
        if depth[at] > best {
            continue;
        }
        if won[at] && depth[at] == best {
            on_route[at] = true;
            continue;
        }
        on_route[at] = edges[at]
            .iter()
            .any(|child| depth[*child as usize] == depth[at] + 1 && on_route[*child as usize]);
    }

    // A pivot is a move number where the route has one square to be on and the
    // board offered others. One square out of one is not a choice, so those are
    // not counted.
    let mut width = vec![(0usize, 0usize); best as usize + 1];
    for node in 0..depth.len() {
        if depth[node] > best {
            continue;
        }
        let slot = &mut width[depth[node] as usize];
        slot.0 += 1;
        if on_route[node] {
            slot.1 += 1;
        }
    }
    report.pivots = width
        .iter()
        .skip(1)
        .take(best as usize - 1)
        .filter(|(reachable, routed)| *routed == 1 && *reachable > 1)
        .count();

    // Where the shortest route makes the board look worse on purpose, and what
    // it has to do that is not a delivery.
    if let Some(route) = solve_path(map, budget) {
        report.skipped = skipped(map, &route);
        let start = initial_state(map);
        let mut state = start.clone();
        let mut score = distance(map, &state);
        for step in route {
            let Some(outcome) = play(map, &state, step) else {
                break;
            };
            state = outcome.state;
            let next = distance(map, &state);
            if next > score {
                report.regressions += 1;
            }
            score = next;
        }
        // A crate that was moved and did not end on a marker was moved for some
        // other reason, and that reason is the side job the board is really
        // about. A crate nobody touched is scenery and does not count.
        report.enablers = state
            .crates
            .iter()
            .zip(start.crates.iter())
            .filter(|(now, before)| now.at != before.at || now.sunk != before.sunk)
            .filter(|(now, _)| now.sunk || !map.goals.contains(&now.at))
            .count()
            + state.latched.iter().filter(|thrown| **thrown).count();
    }

    let obvious = obvious_play(map);
    report.greedy = obvious.finished;
    // The obvious play ending somewhere the board can no longer be finished from
    // is the whole shape of a good trap, and it is only knowable against the set
    // of positions that can still reach a win.
    report.garden_path = obvious.visited.iter().any(|state| {
        index_of
            .get(&search_key(state))
            .is_some_and(|node| !alive[*node as usize])
    });
    report
}
