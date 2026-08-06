//! Whether a board can be short circuited: finished by a route that skips part
//! of what it is made of. A puzzle is a chain of stages, and a chain with a way
//! round it is not one. The two ways round are a job in the completion graph
//! that never gets done and a mechanic on the board that never gets touched,
//! and both are read the same way. Play the solution and see what it never had
//! to do.
//!
//! The reading is made against the shortest solution, because that is the route
//! somebody who has worked the board out will take. Anything that route never
//! engages is something the board can be walked past, which is what lets the
//! generator throw a candidate away before anybody sees it and lets an author
//! be told which half of theirs is decoration.

use crate::objectives::{Job, Objectives, done, objectives};
use crate::rules::{
    GemSpot, MapState, MoveOutcome, Step, active_abilities, beam_field, initial_state, lit_squares,
    play,
};
use crate::schema::{
    Abilities, CrateKind, Direction, Map, Mechanic, Position, Slot, Tile, describe, map_slot_for,
    map_tile, mechanics,
};

/// What a solution never had to do. An empty one is a board where every stage
/// is on the way to the end and everything put down is load bearing.
#[derive(Clone, Debug, Default)]
pub struct Skipped {
    /// Jobs the board asks for that the solution finished without doing.
    pub jobs: Vec<Job>,
    /// Mechanics the board is about that the solution never engaged.
    pub mechanics: Vec<Mechanic>,
}

impl Skipped {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty() && self.mechanics.is_empty()
    }

    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if !self.jobs.is_empty() {
            parts.push(
                self.jobs
                    .iter()
                    .map(|job| job.label())
                    .collect::<Vec<String>>()
                    .join(", "),
            );
        }
        if !self.mechanics.is_empty() {
            parts.push(describe(&self.mechanics));
        }
        parts.join("   ")
    }
}

/// Terrain rather than a thing to do. Water shapes where a body can walk the
/// way a wall does, a mirror and a lens and a wedge shape where the light goes,
/// a burner shapes where a crate can be pushed, and a bed of spikes shapes when
/// a square can be crossed. None of them is something a solution engages: they
/// do their work by standing there, and a board is not the worse for a solution
/// that never has to touch them.
fn scenery(mechanic: Mechanic) -> bool {
    matches!(
        mechanic,
        Mechanic::Water
            | Mechanic::Mirror
            | Mechanic::Glass
            | Mechanic::Prism
            | Mechanic::Splitter
            | Mechanic::Spike
            | Mechanic::Watcher
            | Mechanic::Incinerator
    )
}

/// Everything the given route never had to do on the given board.
pub fn skipped(map: &Map, route: &[Step]) -> Skipped {
    let jobs = objectives(map);
    let mut ever: Vec<bool> = vec![false; jobs.nodes.len()];
    let mut used: Vec<Mechanic> = Vec::new();
    let mut visited: Vec<Slot> = Vec::new();
    let mut state = initial_state(map);
    // What the light was doing before anybody moved. A beam that ends up
    // somewhere else has been interfered with, which is the only way a crate
    // standing in one shows up, because the square it stops on is never lit.
    let opening = beam_field(map, &state).covered;

    read_position(map, &state, &opening, &mut visited, &mut used);
    tick(map, &state, &jobs, &mut ever);
    for step in route {
        let Some(outcome) = play(map, &state, *step) else {
            break;
        };
        read_move(map, &state, *step, &outcome, &mut used);
        state = outcome.state;
        read_position(map, &state, &opening, &mut visited, &mut used);
        tick(map, &state, &jobs, &mut ever);
    }

    if visited.len() > 1 {
        mark(&mut used, Mechanic::Floors);
    }
    if let Some(first) = visited.first()
        && visited.iter().any(|slot| slot.layer != first.layer)
    {
        mark(&mut used, Mechanic::Storeys);
    }

    Skipped {
        jobs: jobs
            .nodes
            .iter()
            .zip(ever.iter())
            .filter(|(_, ever)| !**ever)
            .map(|(node, _)| node.job)
            .collect(),
        mechanics: mechanics(map)
            .into_iter()
            .filter(|mechanic| !scenery(*mechanic) && !used.contains(mechanic))
            .collect(),
    }
}

/// Whether a job has ever been done is the union of whether it was done at each
/// position the route passed through, because most of them are held rather than
/// finished. A plate is only pressed while something stands on it.
fn tick(map: &Map, state: &MapState, jobs: &Objectives, ever: &mut [bool]) {
    for (slot, now) in done(map, state, jobs).into_iter().enumerate() {
        ever[slot] |= now;
    }
}

fn mark(used: &mut Vec<Mechanic>, mechanic: Mechanic) {
    if !used.contains(&mechanic) {
        used.push(mechanic);
    }
}

fn note(visited: &mut Vec<Slot>, slot: Slot) {
    if !visited.contains(&slot) {
        visited.push(slot);
    }
}

/// What a position by itself says. Everything here is a state of the board
/// rather than a move made, so it is asked once before the route starts and
/// again after every step. A board that begins with somebody standing in the
/// light is a board about the light from the first frame.
fn read_position(
    map: &Map,
    state: &MapState,
    opening: &[Position],
    visited: &mut Vec<Slot>,
    used: &mut Vec<Mechanic>,
) {
    for at in state.members.iter().copied().chain(
        state
            .crates
            .iter()
            .filter(|entry| !entry.sunk)
            .map(|entry| entry.at),
    ) {
        note(visited, map_slot_for(map, at).0);
        match map_tile(map, at) {
            Tile::Plate(_) => mark(used, Mechanic::Plate),
            Tile::Gate(_) => mark(used, Mechanic::Gate),
            _ => {}
        }
    }

    if state.latched.iter().any(|thrown| *thrown) {
        mark(used, Mechanic::Switch);
    }
    if !state.collapsed.is_empty() {
        mark(used, Mechanic::Fragile);
    }
    if !state.broken.is_empty() {
        mark(used, Mechanic::Brittle);
    }
    if state
        .pits_filled
        .iter()
        .any(|at| map_tile(map, *at) == Tile::Pit)
    {
        mark(used, Mechanic::Pit);
    }
    if state
        .gems
        .iter()
        .any(|spot| matches!(spot, GemSpot::Seated(_)))
    {
        mark(used, Mechanic::Socket);
    }

    // A socket with a gem in it is a source like any other, so a board whose
    // light comes from one is a board with light to read.
    if !map.emitters.is_empty() || !map.gems.is_empty() {
        let field = beam_field(map, state);
        let powered = field.powered.iter().any(|on| *on);
        let standing_in_it = state.members.iter().any(|at| field.burns(*at));
        // Standing in a colour is standing in the light as much as standing in
        // the one that burns, and it is the only one anybody stands in twice.
        let bathed = state
            .members
            .iter()
            .any(|at| field.lends(*at) != Abilities::NONE);
        if standing_in_it || bathed || powered || field.covered != opening {
            mark(used, Mechanic::Beam);
        }
        if powered {
            mark(used, Mechanic::Receiver);
        }
        // Nobody survives standing in a beam without being warded against it,
        // so anybody still standing in one is the answer to what the ward is
        // for.
        if standing_in_it {
            mark(used, Mechanic::Ward);
        }
    }

    if !map.lamps.is_empty()
        && lit_squares(map, state)
            .iter()
            .any(|at| matches!(map_tile(map, *at), Tile::Sensor(_)))
    {
        mark(used, Mechanic::Sensor);
    }
}

/// What a move says. The squares crossed answer for the floor, the step itself
/// answers for the character, and what came along answers for the crates.
fn read_move(
    map: &Map,
    before: &MapState,
    step: Step,
    outcome: &MoveOutcome,
    used: &mut Vec<Mechanic>,
) {
    for at in outcome
        .player_path
        .iter()
        .chain(outcome.crate_moves.iter().flat_map(|(_, path)| path.iter()))
    {
        match map_tile(map, *at) {
            Tile::Ice => mark(used, Mechanic::Ice),
            Tile::Portal => mark(used, Mechanic::Portal),
            Tile::OneWay(_) => mark(used, Mechanic::OneWay),
            Tile::Conveyor(_) => mark(used, Mechanic::Conveyor),
            Tile::Elevator => mark(used, Mechanic::Elevator),
            Tile::Fragile => mark(used, Mechanic::Fragile),
            Tile::Pit => mark(used, Mechanic::Pit),
            Tile::Water => mark(used, Mechanic::Wade),
            // A door is engaged by being walked through, and these two are the
            // doors that answer to something other than a plate.
            Tile::Shutter(_) => mark(used, Mechanic::Shutter),
            Tile::Lock(_) => mark(used, Mechanic::Lock),
            _ => {}
        }
    }

    // A watcher only ever moves one way, so one that has moved is one that was
    // traded with.
    if outcome.state.watchers != before.watchers {
        mark(used, Mechanic::Watcher);
    }

    match step {
        Step::Take(_) => mark(used, Mechanic::Party),
        Step::Ride(_) => mark(used, Mechanic::Elevator),
        Step::Drag(_) => mark(used, Mechanic::Drag),
        Step::Handle => mark(used, Mechanic::Gem),
        Step::Go(direction) => read_walk(map, before, direction, outcome, used),
    }

    // Two crates moving on one step is the pair rule doing the thing only it
    // does, and one crate moving is every other way a crate moves.
    if outcome.crate_moves.len() > 1 {
        mark(used, Mechanic::Pairs);
    }
    for (index, _) in &outcome.crate_moves {
        match before.crates[*index].kind {
            CrateKind::Orb => mark(used, Mechanic::Orb),
            CrateKind::Lamp => mark(used, Mechanic::Lamp),
            // Nothing moves a boulder, so a boulder in the list of what moved
            // was broken, which is the one thing that can happen to one.
            CrateKind::Stone => {
                mark(used, Mechanic::Stone);
                mark(used, Mechanic::Break);
            }
            // A mirror that moved is a mirror somebody put where the light
            // needed it, which is the whole of what one is for.
            CrateKind::Mirror(_) => mark(used, Mechanic::Mirror),
            CrateKind::Box => {}
        }
    }
}

/// What an ordinary step says, which is where the ways of moving a crate and
/// the ways of getting past a wall are told apart. A step that landed anywhere
/// but the square directly ahead was a stride, and what was in the way says
/// which kind, because open air is crossed and anything else is stepped
/// through.
fn read_walk(
    map: &Map,
    before: &MapState,
    direction: Direction,
    outcome: &MoveOutcome,
    used: &mut Vec<Mechanic>,
) {
    let ahead = before.player().offset(direction.delta());
    if outcome.player_path.first() != Some(&ahead) {
        if map_tile(map, ahead) == Tile::Void {
            mark(used, Mechanic::Blink);
        } else {
            mark(used, Mechanic::Phase);
        }
    }

    if outcome.crate_moves.is_empty() {
        return;
    }
    let abilities = active_abilities(map, before);
    if abilities.swap {
        mark(used, Mechanic::Swap);
        // A trade reaches down the line without walking it, so the squares
        // between are crossed rather than stood on. They still had to be open,
        // and a door among them is a door the trade needed opened.
        if let Some(destination) = outcome.player_path.first() {
            let mut at = before.player().offset(direction.delta());
            while at != *destination && map_tile(map, at) != Tile::Void {
                if matches!(map_tile(map, at), Tile::Gate(_)) {
                    mark(used, Mechanic::Gate);
                }
                at = at.offset(direction.delta());
            }
        }
    } else if abilities.magnetic {
        mark(used, Mechanic::Magnet);
    } else if abilities.push {
        mark(used, Mechanic::Push);
    }
}
