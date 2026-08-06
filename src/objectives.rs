//! What a board is asking for, and what has to happen first. A puzzle is rarely
//! a flat list of deliveries: a door wants something standing on a plate, a
//! marker sits behind a wall that has to be broken, a tower wants light. Those
//! are jobs in their own right and some of them gate the others.
//!
//! None of it is authored. The wiring is already in the schema, because a plate
//! and a gate that share a group are joined by that and by nothing else, and
//! whether a marker is behind a door is answered by taking the door away and
//! seeing what stops being reachable. This reads that structure out so the game
//! can show it and tick it off.

use crate::rules::{MapState, beam_field, gate_flags, lit_squares, seated_at};
use crate::schema::{Map, Position, Tile, connectivity, map_positions, map_tile};
use std::collections::HashSet;

/// What kind of job a node is. The wording is what a player would call it
/// rather than what the schema calls it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Job {
    /// Put a crate on a marker.
    Deliver { goal: Position },
    /// Hold a plate down, with a crate or with a body.
    Weigh { group: u8, at: Position },
    /// Throw a switch, which stays thrown.
    Throw { group: u8, at: Position },
    /// Get lamp light onto a sensor.
    Light { group: u8, at: Position },
    /// Get a beam into a tower. Not the same job as lighting a sensor, because
    /// a lamp throws light in every direction and an emitter throws it in one, so the
    /// two are fed by different things and answered by different questions.
    Power { group: u8, at: Position },
    /// A door that something else opens.
    Open { group: u8 },
    /// Fill a hole, which costs a crate.
    Bridge { at: Position },
    /// Break a cracked wall, which also costs a crate.
    Breach { at: Position },
    /// Carry a gem to a socket and seat it there.
    Seat { at: Position },
}

impl Job {
    pub fn label(self) -> String {
        match self {
            Self::Deliver { goal } => format!("deliver to {},{}", goal.cell.0, goal.cell.1),
            Self::Weigh { .. } => "hold the plate".to_string(),
            Self::Throw { .. } => "throw the switch".to_string(),
            Self::Light { .. } => "light the sensor".to_string(),
            Self::Power { .. } => "power the tower".to_string(),
            Self::Open { .. } => "open the door".to_string(),
            Self::Bridge { .. } => "fill the hole".to_string(),
            Self::Breach { .. } => "break the wall".to_string(),
            Self::Seat { .. } => "seat a gem".to_string(),
        }
    }
}

/// One job and what it waits on, by index into the list it lives in.
#[derive(Clone, Debug)]
pub struct Node {
    pub job: Job,
    /// Jobs that have to be done before this one can be. Indices into the same
    /// list, and always earlier ones, so the list can be walked in order.
    pub needs: Vec<usize>,
}

/// Everything a board is asking for, in an order where nothing waits on
/// something after it.
#[derive(Clone, Debug, Default)]
pub struct Objectives {
    pub nodes: Vec<Node>,
}

/// Reads the jobs off a board. Triggers first, then the doors they open, then
/// the deliveries, which is the order the dependencies run in.
pub fn objectives(map: &Map) -> Objectives {
    let mut nodes: Vec<Node> = Vec::new();

    // The things that answer for a door, and the door itself. A group is the
    // whole of the wiring, so anything naming a group feeds every gate that
    // names it, which is exactly what the rules do at runtime.
    let mut groups: Vec<u8> = Vec::new();
    for at in map_positions(map) {
        let (group, job) = match map_tile(map, at) {
            Tile::Plate(group) => (group, Job::Weigh { group, at }),
            Tile::Switch(group) => (group, Job::Throw { group, at }),
            Tile::Sensor(group) => (group, Job::Light { group, at }),
            Tile::Receiver(group) => (group, Job::Power { group, at }),
            Tile::Pit => {
                nodes.push(Node {
                    job: Job::Bridge { at },
                    needs: Vec::new(),
                });
                continue;
            }
            Tile::Brittle => {
                nodes.push(Node {
                    job: Job::Breach { at },
                    needs: Vec::new(),
                });
                continue;
            }
            Tile::Socket(_) => {
                nodes.push(Node {
                    job: Job::Seat { at },
                    needs: Vec::new(),
                });
                continue;
            }
            _ => continue,
        };
        nodes.push(Node {
            job,
            needs: Vec::new(),
        });
        if !groups.contains(&group) {
            groups.push(group);
        }
    }

    // A door waits on everything that names its group. Any one of them is
    // enough to open it, which the display says by listing them all under it.
    for group in groups {
        let has_gate = map_positions(map)
            .into_iter()
            .any(|at| matches!(map_tile(map, at), Tile::Gate(other) if other == group));
        if !has_gate {
            continue;
        }
        let needs = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| match node.job {
                Job::Weigh { group: other, .. }
                | Job::Throw { group: other, .. }
                | Job::Light { group: other, .. }
                | Job::Power { group: other, .. } => other == group,
                _ => false,
            })
            .map(|(index, _)| index)
            .collect();
        nodes.push(Node {
            job: Job::Open { group },
            needs,
        });
    }

    // A marker waits on whatever stands between a crate and it. The question is
    // asked of crates rather than of the player, because a delivery needs a
    // crate to arrive and the two do not go the same places. A player steps over
    // a hole that no crate will ever cross.
    for goal in &map.goals {
        let mut needs = Vec::new();
        let supplied = |opened: &[Position]| -> bool {
            let open = crate_region(map, *goal, opened);
            map.crates
                .iter()
                .chain(map.orbs.iter())
                .chain(map.lamps.iter())
                .any(|start| open.contains(start))
        };
        if !supplied(&[]) {
            for (index, node) in nodes.iter().enumerate() {
                let lifted: Vec<Position> = match node.job {
                    Job::Open { group } => shut_squares(map, group),
                    Job::Breach { at } | Job::Bridge { at } => vec![at],
                    _ => continue,
                };
                // Reachable by a crate with this out of the way and not without
                // it is what it means for the marker to be behind it.
                if supplied(&lifted) {
                    needs.push(index);
                }
            }
        }
        nodes.push(Node {
            job: Job::Deliver { goal: *goal },
            needs,
        });
    }

    Objectives { nodes }
}

/// Every square a gate of this group stands on.
fn shut_squares(map: &Map, group: u8) -> Vec<Position> {
    map_positions(map)
        .into_iter()
        .filter(|at| matches!(map_tile(map, *at), Tile::Gate(other) if other == group))
        .collect()
}

/// Every square a crate could occupy in the same stretch of floor as this one,
/// counting the named squares as floor and everything a crate cannot cross as
/// wall. A door stands shut here and a hole stays a hole, which is the whole
/// point, because lifting one and asking the question again is what says
/// whether the marker was behind it.
///
/// This is a region rather than a route. A crate is shoved in straight lines and
/// two squares in one region are not always one push apart, so a region says
/// what is certainly cut off rather than what is certainly deliverable. That is
/// the safe direction to be wrong in for a display of what waits on what.
fn crate_region(map: &Map, from: Position, opened: &[Position]) -> HashSet<Position> {
    let graph = connectivity(map);
    let solid = |at: Position| -> bool {
        if opened.contains(&at) {
            return false;
        }
        let tile = map_tile(map, at);
        // A burner is not somewhere a crate can be on the way to anywhere, for
        // the same reason a hole is not.
        tile.blocks_forever()
            || matches!(
                tile,
                Tile::Gate(_) | Tile::Brittle | Tile::Pit | Tile::Incinerator
            )
    };
    let mut seen = HashSet::new();
    if solid(from) {
        return seen;
    }
    let mut pending = vec![from];
    while let Some(at) = pending.pop() {
        if !seen.insert(at) {
            continue;
        }
        for neighbour in graph.neighbors(at) {
            if !solid(neighbour) && !seen.contains(&neighbour) {
                pending.push(neighbour);
            }
        }
    }
    seen
}

/// Whether each job is done, right now. Read off the position rather than
/// remembered, so undo and restart need no unwinding.
pub fn done(map: &Map, state: &MapState, objectives: &Objectives) -> Vec<bool> {
    let gates = gate_flags(map, state);
    let lit = lit_squares(map, state);
    let beams = beam_field(map, state);
    objectives
        .nodes
        .iter()
        .map(|node| match node.job {
            Job::Deliver { goal } => state
                .crates
                .iter()
                .any(|entry| !entry.sunk && entry.at == goal),
            Job::Weigh { at, .. } => {
                state.crates.iter().any(|entry| entry.at == at) || state.members.contains(&at)
            }
            Job::Throw { group, .. } => state.latched.get(group as usize).copied().unwrap_or(false),
            Job::Light { at, .. } => lit.contains(&at),
            Job::Power { group, .. } => beams.powered.get(group as usize).copied().unwrap_or(false),
            Job::Open { group } => gates.get(group as usize).copied().unwrap_or(false),
            Job::Bridge { at } => state.pits_filled.contains(&at),
            Job::Breach { at } => state.broken.contains(&at),
            Job::Seat { at } => seated_at(state, at).is_some(),
        })
        .collect()
}
