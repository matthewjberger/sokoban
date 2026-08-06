//! What the two engines and the prunes have to be held to.
//!
//! Everything the solver does beyond walking every move is a claim that some
//! part of the graph can be skipped, and a claim like that is either true or it
//! quietly shortens par on a board nobody rechecks. So each of them is checked
//! here against a search that claims nothing: breadth first over every move,
//! throwing away only positions the board has already killed.

use super::board::Board;
use super::pushspace::estimate;
use super::{BATCH, Progress, ROOT, Search, retrace, search_key};
use crate::generator::{HAZARD_STAGES, PRESET_COUNT, Recipe, apply_hazards, generate, preset};
use crate::maps::{load_map, map_count};
use crate::rules::{Step, expansions, initial_state, lethal, map_solved, play};
use crate::schema::{Map, Tile, WinCondition, map_positions, map_tile};
use std::collections::{HashSet, VecDeque};

/// How far the search that claims nothing will walk. It is walking the whole
/// graph, so this is much lower than anything the solver runs at, and a board
/// it cannot decide is one these checks step over rather than guess about.
const HONEST_BUDGET: usize = 400_000;

/// The same, for the solver, which is being asked the same question and has to
/// be allowed to answer it.
const SOLVER_BUDGET: usize = 2_000_000;

enum Truth {
    Route(Vec<Step>),
    Impossible,
    /// Too big to decide inside the budget, which is not an answer and not a
    /// failure either.
    Unknown,
}

/// Breadth first over every move, pruning nothing but positions the board has
/// killed. This is slower than the solver by design, because it is what the
/// solver is measured against and must not share any reasoning with it.
fn honest(map: &Map, budget: usize) -> Truth {
    let start = initial_state(map);
    if map_solved(map, &start) {
        return Truth::Route(Vec::new());
    }
    let mut seen = HashSet::new();
    seen.insert(search_key(&start));
    let mut nodes: Vec<(u32, Step)> = Vec::new();
    let mut frontier = VecDeque::new();
    frontier.push_back((start, ROOT));
    let mut walked = 0usize;
    while let Some((state, parent)) = frontier.pop_front() {
        walked += 1;
        if walked > budget {
            return Truth::Unknown;
        }
        for (step, outcome) in expansions(map, &state) {
            if map_solved(map, &outcome.state) {
                let mut route = retrace(&nodes, parent);
                route.push(step);
                return Truth::Route(route);
            }
            if lethal(map, &outcome.state) || !seen.insert(search_key(&outcome.state)) {
                continue;
            }
            nodes.push((parent, step));
            frontier.push_back((outcome.state, (nodes.len() - 1) as u32));
        }
    }
    Truth::Impossible
}

fn run(search: &mut Search, map: &Map) -> Truth {
    loop {
        match search.advance(map, BATCH) {
            Progress::Running => {}
            Progress::Solved(route) => return Truth::Route(route),
            Progress::Unsolvable => return Truth::Impossible,
            Progress::Exhausted => return Truth::Unknown,
        }
    }
}

/// Whether a route the search handed back is a route somebody could make.
fn plays_out(map: &Map, route: &[Step]) -> bool {
    let mut state = initial_state(map);
    for step in route {
        let Some(outcome) = play(map, &state, *step) else {
            return false;
        };
        state = outcome.state;
    }
    map_solved(map, &state)
}

fn holds(map: &Map, wanted: &[Tile]) -> bool {
    map_positions(map)
        .into_iter()
        .any(|at| wanted.contains(&map_tile(map, at)))
}

/// The same board asked for the other way about. Every board the game ships
/// wants its markers filled, so nothing here would otherwise take the reading
/// that wants every crate placed, where a spare crate stops being spare.
fn either_way(map: &Map) -> Vec<Map> {
    let mut strict = map.clone();
    strict.rules.win = WinCondition::CratesOnGoals;
    vec![map.clone(), strict]
}

/// Small boards from every setting of both dials, which is what stops these
/// checks being a reading of one rule set copied fifty-seven times.
fn generated() -> Vec<Map> {
    let mut boards = Vec::new();
    for index in 0..PRESET_COUNT {
        let recipe = Recipe {
            attempts: 300,
            ..preset(index)
        };
        boards.extend(generate(&recipe));
    }
    for stage in 0..HAZARD_STAGES.len() {
        let mut recipe = Recipe {
            floor_width: 8,
            floor_height: 7,
            attempts: 300,
            ..Default::default()
        };
        apply_hazards(&mut recipe, stage);
        boards.extend(generate(&recipe));
    }
    boards
}

/// Two engines that disagree about a board are two engines one of which is
/// wrong. Every board push space will take is put to both of them, and the
/// answer has to be the same number of moves out of each.
#[test]
fn the_engines_agree() {
    let mut checked = 0;
    let mut boards: Vec<Map> = (0..map_count()).map(load_map).collect();
    boards.extend(generated());
    let boards: Vec<Map> = boards.iter().flat_map(either_way).collect();
    for map in &boards {
        if !Board::read(map).quiet {
            continue;
        }
        let pushes = run(&mut Search::new(map, SOLVER_BUDGET), map);
        let moves = run(&mut Search::in_move_space(map, SOLVER_BUDGET), map);
        match (pushes, moves) {
            (Truth::Route(pushes), Truth::Route(moves)) => {
                assert_eq!(
                    pushes.len(),
                    moves.len(),
                    "{} answered differently by the two engines",
                    map.name
                );
                assert!(
                    plays_out(map, &pushes),
                    "{} route does not play out",
                    map.name
                );
                checked += 1;
            }
            (Truth::Impossible, Truth::Impossible) => checked += 1,
            (Truth::Unknown, _) | (_, Truth::Unknown) => {}
            // A generated board is gone the moment the run that made it ends,
            // so the board itself is the failure rather than its name.
            _ => panic!(
                "{} is finished by one engine and not by the other\n{}",
                map.name,
                crate::storage::to_json(map)
            ),
        }
    }
    assert!(checked > 8, "only {checked} boards reached push space");
}

/// The bound has to be a floor under what is left to do, never a guess above
/// it. An overstatement would let the search settle for a route that is not the
/// shortest, which is par quietly changing rather than anything failing.
///
/// Both engines would share an inflated bound and agree with each other while
/// both were wrong, so this is measured against the search that claims nothing
/// rather than against the other engine. Every position on a shortest route has
/// a known cost remaining, which is the length of the rest of that route.
#[test]
fn the_bound_never_overstates() {
    let carried = [
        Tile::Ice,
        Tile::Portal,
        Tile::Conveyor(crate::schema::Direction::Up),
        Tile::Conveyor(crate::schema::Direction::Down),
        Tile::Conveyor(crate::schema::Direction::Left),
        Tile::Conveyor(crate::schema::Direction::Right),
    ];
    let mut checked = 0;
    let mut boards: Vec<Map> = (0..map_count()).map(load_map).collect();
    boards.extend(generated());
    let boards: Vec<Map> = boards.iter().flat_map(either_way).collect();
    for map in &boards {
        if !holds(map, &carried) {
            continue;
        }
        let Truth::Route(route) = honest(map, HONEST_BUDGET) else {
            continue;
        };
        let board = Board::read(map);
        let mut state = initial_state(map);
        let mut live = Vec::new();
        for index in 0..=route.len() {
            live.clear();
            live.extend(
                state
                    .crates
                    .iter()
                    .filter(|entry| !entry.sunk)
                    .map(|entry| board.index(entry.at)),
            );
            if let Some(bound) = estimate(&board, &live) {
                assert!(
                    bound as usize <= route.len() - index,
                    "{} overstates what is left at move {index}: {bound} against {}",
                    map.name,
                    route.len() - index
                );
            }
            if index == route.len() {
                break;
            }
            state = play(map, &state, route[index])
                .expect("a shortest route replays")
                .state;
        }
        checked += 1;
    }
    assert!(checked > 4, "only {checked} boards carried anything");
}

/// A crate in a corner is out of the game only while nothing can lift it out.
/// A drag can, and so can a trade, so the prunes read the board and the party
/// together. Getting that wrong calls a live board dead, which is the loudest
/// failure available and still worth pinning down here rather than in the
/// campaign.
#[test]
fn the_prunes_hold_where_crates_can_be_lifted() {
    let mut checked = 0;
    for level in 0..map_count() {
        let map = load_map(level);
        let abilities = map.party_abilities();
        if !abilities.pull && !abilities.swap {
            continue;
        }
        let Truth::Route(honest_route) = honest(&map, HONEST_BUDGET) else {
            continue;
        };
        let Truth::Route(pruned) = run(&mut Search::new(&map, SOLVER_BUDGET), &map) else {
            panic!(
                "{} is decided without the prunes and not with them",
                map.name
            );
        };
        assert_eq!(
            pruned.len(),
            honest_route.len(),
            "{} answers differently once the prunes are on",
            map.name
        );
        checked += 1;
    }
    assert!(checked > 2, "only {checked} boards had a hand to lift with");
}

/// Every board the campaign ships, answered by the search that claims nothing
/// and by the one that ships, has to come out the same length. This is the
/// check that no prune anywhere below has quietly moved par.
#[test]
fn the_campaign_answers_the_same_either_way() {
    let mut checked = 0;
    let mut moved = Vec::new();
    let boards: Vec<Map> = (0..map_count())
        .map(load_map)
        .flat_map(|map| either_way(&map))
        .collect();
    for map in &boards {
        let Truth::Route(honest_route) = honest(map, HONEST_BUDGET) else {
            continue;
        };
        let Truth::Route(shipped) = run(&mut Search::new(map, SOLVER_BUDGET), map) else {
            panic!(
                "{} is decided by the honest search and not by the solver",
                map.name
            );
        };
        assert_eq!(shipped.len(), honest_route.len(), "{} moved par", map.name);
        if shipped != honest_route {
            moved.push(map.name.clone());
        }
        checked += 1;
    }
    // Which route comes back where several are equally short is allowed to
    // change. How long it is is not.
    println!("same length by another route: {}", moved.join(", "));
    assert!(checked > 20, "only {checked} boards were decided both ways");
}
