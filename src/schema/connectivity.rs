//! The map as a graph. Squares are nodes and every way of getting from one to
//! another is an edge: a step, a portal, an elevator ride. It is small, it is
//! genuinely an adjacency list, and holding it as one means reachability and
//! cut off regions come from the graph library rather than from another hand
//! written traversal.

use crate::schema::{
    Direction, Map, Position, Tile, map_positions, map_step, map_teleport_exit, map_tile,
};
use petgraph::algo::connected_components;
use petgraph::prelude::UnGraphMap;
use std::collections::HashSet;

/// Every square a player could occupy, linked by every move that connects
/// them. Crates are left out, because this is the shape of the board rather
/// than a position in a game. Every link here is walkable both ways, elevators and portals
/// included, so the graph is undirected and says so.
///
/// Only what is solid forever cuts the board. A gate opens and a brittle wall
/// breaks, so counting either as a wall would report a goal behind it as walled
/// off when it is merely shut.
pub type Connectivity = UnGraphMap<Position, ()>;

pub fn connectivity(map: &Map) -> Connectivity {
    let mut graph = Connectivity::new();
    // What anybody here can do, asked once. Where a body can get is a question
    // about the party rather than about whoever happens to be in front, and
    // about the light as well, since a power to go and stand in is a power the
    // party has.
    let powers = map.latent_abilities();
    let standable = |at: Position| -> bool {
        let tile = map_tile(map, at);
        !tile.blocks_forever() || (powers.wades && tile == Tile::Water)
    };
    let squares: Vec<Position> = map_positions(map)
        .into_iter()
        .filter(|at| standable(*at))
        .collect();
    for at in &squares {
        graph.add_node(*at);
    }

    for at in squares {
        // A step, a stride through a wall and a crossing of open air are all
        // the same question, and it is answered in one place so this graph and
        // the rules cannot come to different conclusions about the same board.
        for direction in Direction::ALL {
            if let Some(landing) = map_step(map, at, direction, powers) {
                graph.add_edge(at, landing, ());
            }
        }
        if let Some(exit) = map_teleport_exit(map, at)
            && standable(exit)
        {
            graph.add_edge(at, exit, ());
        }
        if map.rules.elevators_move_player && map_tile(map, at) == Tile::Elevator {
            for direction in [1, -1] {
                let target = Position::new(at.layer + direction, at.cell);
                if map_tile(map, target) == Tile::Elevator {
                    graph.add_edge(at, target, ());
                }
            }
        }
    }

    graph
}

/// Every square the player could stand on if crates were not in the way.
/// Reachability is the cheapest useful check on a generated map, because
/// everything the puzzle needs has to be inside this set.
pub fn map_reachable(map: &Map) -> HashSet<Position> {
    let graph = connectivity(map);
    if !graph.contains_node(map.player) {
        return HashSet::new();
    }
    let mut walk = petgraph::visit::Bfs::new(&graph, map.player);
    let mut seen = HashSet::new();
    while let Some(at) = walk.next(&graph) {
        seen.insert(at);
    }
    seen
}

/// How many separate places the board falls into. A map in more than one piece
/// is usually an accident, and saying so is more use to an author than naming
/// each stranded square.
pub fn region_count(map: &Map) -> usize {
    connected_components(&connectivity(map))
}
