mod campaign;
mod cutscenes;
mod ecs;
mod gallery;
mod generator;
mod insight;
mod maps;
mod objectives;
mod palette;
mod plugin;
mod rules;
mod schema;
mod shortcut;
mod solver;
mod storage;
mod story;
mod systems;
mod theme;

use clap::{Parser, Subcommand};
use generator::{PRESET_COUNT, Recipe, preset};
use maps::{load_map, map_count};
use nightshade::prelude::{App, CutscenePlugin, DefaultPlugins, GamepadPlugin};
use plugin::SokobanPlugin;
use schema::{Position, validate};
use solver::{DEFAULT_STATE_BUDGET, solve, solve_path};

/// The game, and the checks that answer for the data it ships. Every board is
/// a value in a file rather than code, so what would otherwise be a test suite
/// is a set of readings taken off that data, and they are here rather than in
/// a tool of their own because they need the rules to take them.
#[derive(Parser)]
#[command(name = "sokoban", version, about, long_about = None)]
struct Arguments {
    #[command(subcommand)]
    check: Option<Check>,
}

#[derive(Subcommand)]
enum Check {
    /// Runs the static pass and the exhaustive search over every shipped board.
    Analyze {
        /// How many positions the search may walk per board before it gives up.
        #[arg(default_value_t = DEFAULT_STATE_BUDGET)]
        budget: usize,
    },
    /// Generates boards from every preset and every hazard setting.
    Random {
        /// How many boards to ask each setting for.
        #[arg(default_value_t = 3)]
        count: usize,
    },
    /// Prints what each board is asking for and what waits on what.
    Jobs,
    /// Reads the campaign in order and says what each board is about.
    Story,
    /// Reads the move graphs and says whether there is anything to work out.
    Insight {
        /// One board by its number, for reworking a single room without
        /// reading the whole campaign.
        level: Option<usize>,
    },
    /// Plays one board as each character, so no two of them are one character
    /// with two names.
    Characters,
    /// Checks the overworld: its doors, its plates, and the order it opens in.
    Depot,
    /// Plays every gallery demonstration through the rules.
    Lessons,
    /// Solves every gallery board and writes the worked example back into the
    /// gallery file, so a lesson shows its rule and then finishes the puzzle
    /// that rule is on rather than stopping once the rule has fired.
    Demos,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Arguments::parse().check {
        Some(Check::Analyze { budget }) => analyze_campaign(budget),
        Some(Check::Random { count }) => analyze_generator(count),
        Some(Check::Jobs) => check_jobs(),
        Some(Check::Story) => check_story(),
        Some(Check::Insight { level }) => check_insight(level.and_then(|one| one.checked_sub(1))),
        Some(Check::Characters) => check_characters(),
        Some(Check::Depot) => check_depot(),
        Some(Check::Lessons) => check_lessons(),
        Some(Check::Demos) => write_demos(),
        None => {
            return App::new()
                .add_plugins(DefaultPlugins)
                .add_plugin(GamepadPlugin)
                .add_plugin(CutscenePlugin)
                .add_plugin(SokobanPlugin)
                .run();
        }
    }
    Ok(())
}

/// Plays one board as each character. Two characters that solve exactly the
/// same boards are one character with two names, so this is the check that the
/// four of them are really four.
///
/// The board is a corridor one square wide. A crate sits on the plate holding
/// the gate open, and the player starts beside it with the only way out being
/// straight away from that crate. Choosing to leave it is the whole puzzle.
/// Prints what each board is asking for and what waits on what. Nothing here is
/// authored. The wiring comes from the schema and the dependencies from taking a
/// door away and seeing what stops being reachable, so this is how that reading
/// gets checked against boards whose answer is known.
fn check_jobs() {
    use crate::objectives::objectives;
    use crate::rules::initial_state;

    let mut flat = Vec::new();
    for level in 0..map_count() {
        let map = load_map(level);
        let jobs = objectives(&map);
        let state = initial_state(&map);
        let done = crate::objectives::done(&map, &state, &jobs);
        println!("{:>3}. {}", level + 1, map.name);
        for (index, node) in jobs.nodes.iter().enumerate() {
            let waits = if node.needs.is_empty() {
                String::new()
            } else {
                format!(
                    "   after {}",
                    node.needs
                        .iter()
                        .map(|need| jobs.nodes[*need].job.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            println!(
                "     {} {:22}{waits}",
                if done[index] { "X" } else { "-" },
                node.job.label()
            );
        }
        // A board whose markers wait on nothing is a board where the jobs are a
        // list rather than a graph, which is worth knowing per board rather than
        // being an average.
        if jobs.nodes.iter().all(|node| node.needs.is_empty()) {
            flat.push(map.name.clone());
        }
    }
    println!(
        "flat boards, where nothing waits on anything: {}",
        flat.len()
    );
    println!("{}", flat.join(", "));
}

/// Reads the campaign in order and says what each board is about, which it
/// works out from the board rather than from a list kept beside it. A campaign
/// teaches when every board brings something new or combines what came before,
/// and this is where a board that does neither gets found.
fn check_story() {
    use crate::schema::{Mechanic, complexity, describe, mechanics};
    use crate::story::{area_of, areas};

    println!("the campaign, by what each board is about");
    let mut seen: Vec<Mechanic> = Vec::new();
    let mut idle = Vec::new();
    let mut crowded = Vec::new();
    let mut mixers = 0usize;
    let mut area = usize::MAX;

    for level in 0..map_count() {
        if area_of(level) != area {
            area = area_of(level);
            println!("  {}  ·  {}", areas()[area].name, areas()[area].blurb);
        }
        let map = load_map(level);
        let about = mechanics(&map);
        let fresh: Vec<Mechanic> = about
            .iter()
            .copied()
            .filter(|mechanic| !seen.contains(mechanic))
            .collect();
        let carried = about.len() - fresh.len();
        seen.extend(fresh.iter().copied());

        // A board earns its place by teaching something, by putting several
        // taught things in one room, or by what its move graph holds. This check
        // only sees the first two, so a board it names is one to look up in
        // --insight rather than one to throw away.
        if fresh.is_empty() && carried < 3 {
            idle.push(map.name.clone());
        }
        if fresh.len() > 2 {
            crowded.push(map.name.clone());
        }
        if about.len() >= 3 {
            mixers += 1;
        }
        println!(
            "{:>3}. {:18} {:>2} new  {:>2} carried  weight {:>3}   {}",
            level + 1,
            map.name,
            fresh.len(),
            carried,
            complexity(&map),
            describe(&about)
        );
    }

    let missing: Vec<&str> = Mechanic::ALL
        .iter()
        .filter(|mechanic| !seen.contains(mechanic))
        .map(|mechanic| mechanic.name())
        .collect();
    println!(
        "{} of {} mechanics taught, {mixers} boards combine three or more",
        seen.len(),
        Mechanic::ALL.len()
    );
    if !missing.is_empty() {
        println!("never taught: {}", missing.join(", "));
    }
    if !idle.is_empty() {
        println!(
            "repeat known mechanics without combining them, so their worth is in the graph: {}",
            idle.join(", ")
        );
    }
    if !crowded.is_empty() {
        println!("too much at once: {}", crowded.join(", "));
    }
}

/// Reads every campaign board's move graph and says whether there is anything
/// in it to work out. A board that can be finished by always making it look
/// better is a walk, and this is where that gets found rather than after it
/// ships.
fn check_insight(only: Option<usize>) {
    use crate::insight::{INSIGHT_BUDGET, Verdict, insight};

    println!("the campaign, by what its move graph holds");
    let mut thin = Vec::new();
    let mut walked_past = Vec::new();
    let mut gardens = 0usize;
    let mut yields = 0usize;
    for level in 0..map_count() {
        if only.is_some_and(|wanted| wanted != level) {
            continue;
        }
        let map = load_map(level);
        let report = insight(&map, INSIGHT_BUDGET);
        if report.garden_path {
            gardens += 1;
        }
        if report.greedy {
            yields += 1;
        }
        if !report.skipped.is_empty() {
            walked_past.push(map.name.clone());
        }
        // The opening board is a demonstration of what the verb is, so being
        // solvable by the obvious move is what it is for rather than a fault
        // in it. Every board after it has to be worth playing.
        let teaching = level == 0;
        println!(
            "{:>3}. {:18} {}{}",
            level + 1,
            map.name,
            report.describe(),
            if teaching { "  (opening lesson)" } else { "" }
        );
        if !teaching && matches!(report.verdict(), Verdict::Thin | Verdict::Obvious) {
            thin.push(map.name.clone());
        }
    }

    if only.is_some() {
        return;
    }

    let overworld = crate::story::overworld();
    let report = insight(&overworld, INSIGHT_BUDGET);
    println!("     {:18} {}", "the depot", report.describe());

    println!(
        "{traps} of the boards punish the obvious play, {yields} yield to it entirely",
        traps = gardens,
        yields = yields
    );
    if thin.is_empty() {
        println!("every board has something in it");
    } else {
        println!("nothing to work out on: {}", thin.join(", "));
    }
    if walked_past.is_empty() {
        println!("every board needs everything it puts down");
    } else {
        println!(
            "short circuited, finishable with part of them skipped: {}",
            walked_past.join(", ")
        );
    }
}

fn check_characters() {
    use schema::{Character, Tile, map_blank, map_set_tile};

    let build = |character: Character| {
        let mut map = map_blank(12, 7);
        map.character = character;
        for y in 1..6 {
            for x in 1..11 {
                if y != 3 {
                    map_set_tile(&mut map, Position::new(0, (x, y)), Tile::Wall);
                }
            }
        }
        map_set_tile(&mut map, Position::new(0, (2, 3)), Tile::Plate(0));
        map_set_tile(&mut map, Position::new(0, (5, 3)), Tile::Gate(0));
        map.player = Position::new(0, (3, 3));
        map.crates = vec![Position::new(0, (2, 3)), Position::new(0, (8, 3))];
        map.goals = vec![Position::new(0, (7, 3))];
        schema::map_relink(&mut map);
        map
    };

    for character in Character::ALL {
        let map = build(character);
        let abilities = character.abilities();
        let flags = [
            ("push", abilities.push),
            ("drag", abilities.pull),
            ("forced", abilities.magnetic),
            ("swap", abilities.swap),
            ("wades", abilities.wades),
            ("phases", abilities.phasing),
            ("warded", abilities.warded),
            ("blinks", abilities.blinks),
            ("breaks", abilities.smashes),
        ];
        let named: Vec<&str> = flags
            .iter()
            .filter(|(_, on)| *on)
            .map(|(name, _)| *name)
            .collect();
        println!(
            "{:<8} {:<28} {}",
            character.label(),
            named.join(" "),
            solve(&map, DEFAULT_STATE_BUDGET).describe()
        );
    }
}

/// Checks the overworld as data. The depot is a map like any other, so the
/// questions worth asking about it are the ones worth asking about a map: does
/// every door stand on a square that is a door, can every crate reach the plate
/// it is meant for, and does the order the doors open in reach all of them.
fn check_depot() {
    use rules::{beam_field, initial_state};
    use schema::{Tile, map_reachable, map_tile};
    use story::{area_of, areas, door_position, level_unlocked, overworld};

    let mut map = overworld();
    schema::map_relink(&mut map);
    let state = initial_state(&map);

    let mut faults = Vec::new();
    for level in 0..map_count() {
        let at = door_position(level);
        if !matches!(map_tile(&map, at), Tile::Gateway(_)) {
            faults.push(format!("door {} does not stand on a gateway", level + 1));
        }
    }

    // A crate can only be shoved in straight lines, so a plate set off the line
    // of its crate is a plate that can never be pressed.
    let plates: Vec<Position> = schema::map_positions(&map)
        .into_iter()
        .filter(|at| matches!(map_tile(&map, *at), Tile::Plate(_)))
        .collect();
    for plate in &plates {
        let partner = map.crates.iter().find(|crate_at| {
            crate_at.layer == plate.layer
                && (crate_at.cell.0 == plate.cell.0 || crate_at.cell.1 == plate.cell.1)
        });
        let Some(crate_at) = partner else {
            faults.push(format!(
                "plate at {},{} has no crate on a line with it",
                plate.cell.0, plate.cell.1
            ));
            continue;
        };
        // Lining up is half of it. A shove is made from the square behind the
        // crate, so that square has to exist, be walkable, and be somewhere the
        // player can actually get to.
        let step = (
            (plate.cell.0 - crate_at.cell.0).signum(),
            (plate.cell.1 - crate_at.cell.1).signum(),
        );
        let behind = crate_at.offset((-step.0, -step.1));
        if map_tile(&map, behind).blocks_walking() {
            faults.push(format!(
                "the crate for the plate at {},{} is shoved from {},{}, which is solid",
                plate.cell.0, plate.cell.1, behind.cell.0, behind.cell.1
            ));
        }
    }

    let reachable = map_reachable(&map);
    for level in 0..map_count() {
        if !reachable.contains(&door_position(level)) {
            faults.push(format!("door {} is walled off for good", level + 1));
        }
    }
    for plate in &plates {
        if !reachable.contains(plate) {
            faults.push(format!(
                "plate at {},{} is walled off for good",
                plate.cell.0, plate.cell.1
            ));
        }
    }

    // Clearing whatever is open, over and over, has to reach every room. A door
    // nothing ever opens is a room nobody can play.
    let mut cleared = vec![false; map_count()];
    let mut wave = 0;
    loop {
        let opened: Vec<usize> = (0..map_count())
            .filter(|level| !cleared[*level] && level_unlocked(*level, &cleared))
            .collect();
        if opened.is_empty() {
            break;
        }
        wave += 1;
        let names: Vec<String> = opened
            .iter()
            .map(|level| {
                format!(
                    "{} ({})",
                    load_map(*level).name,
                    areas()[area_of(*level)].name
                )
            })
            .collect();
        println!("wave {wave}: {}", names.join(", "));
        for level in opened {
            cleared[level] = true;
        }
    }
    let stranded = cleared.iter().filter(|done| !**done).count();
    if stranded > 0 {
        faults.push(format!("{stranded} rooms never open"));
    }

    println!(
        "{} floors, {} doors, {} plates, {} crates, {} beams",
        map.floors.len(),
        map_count(),
        plates.len(),
        map.crates.len(),
        beam_field(&map, &state).segments.len()
    );
    let roots = crate::campaign::campaign()
        .levels
        .iter()
        .filter(|entry| entry.requires.is_empty())
        .count();
    println!("{roots} room(s) open from the start, cleared in {wave} waves");
    if faults.is_empty() {
        println!("the depot holds together");
    } else {
        for fault in faults {
            println!("FAULT: {fault}");
        }
    }
}

/// Plays every gallery demonstration through the rules. A step the rules refuse
/// is a script that walks into a wall, which would show the player a mechanic
/// failing to work rather than working.
fn check_lessons() {
    use gallery::lessons;
    use rules::{Step, attempt_move, attempt_pull, attempt_ride, goals_covered, initial_state};

    for (index, entry) in lessons().iter().enumerate() {
        let map = gallery::lesson_map(index);
        let mut state = initial_state(&map);
        let mut refused = 0;
        for step in &entry.demo {
            let outcome = match step {
                Step::Go(direction) => attempt_move(&map, &state, *direction),
                Step::Drag(direction) => attempt_pull(&map, &state, *direction),
                Step::Ride(direction) => attempt_ride(&map, &state, *direction),
                Step::Take(index) => rules::attempt_take(&map, &state, *index),
                Step::Handle => rules::attempt_handle(&map, &state),
            };
            match outcome {
                Some(outcome) => state = outcome.state,
                None => refused += 1,
            }
        }
        let sunk = state.crates.iter().filter(|entry| entry.sunk).count();
        let latched = state.latched.iter().filter(|on| **on).count();
        // What the lamps are reaching at the end, which is the only way to see
        // whether a board about light did anything about light.
        let lit = rules::lit_squares(&map, &state)
            .into_iter()
            .filter(|at| matches!(schema::map_tile(&map, *at), schema::Tile::Sensor(_)))
            .count();
        // A lesson is a board as much as a demonstration, so the run has to
        // leave it finished. One that shows the rule and stops halfway through
        // the puzzle is a lesson with the answer cut off.
        let finished = rules::map_solved(&map, &state);
        // And what the run never had to touch, which on a lesson is the
        // question that matters: a board whose own subject can be walked past
        // is a board that demonstrates nothing.
        let walked_past = shortcut::skipped(&map, &entry.demo);
        println!(
            "{:<18} {:>2} steps  {refused} refused  {} pushes  {sunk} sunk  {} dropped  {latched} latched  {lit} lit  {} of {} goals  {}",
            entry.name,
            entry.demo.len(),
            state.pushes,
            state.collapsed.len(),
            goals_covered(&map, &state),
            map.goals.len(),
            if finished { "SOLVED" } else { "unfinished" }
        );
        if !walked_past.mechanics.is_empty() {
            println!("{:<18}   walks past  {}", "", {
                use crate::schema::describe;
                describe(&walked_past.mechanics)
            });
        }
    }
}

/// Solves every gallery board and writes the answer back as that lesson's
/// worked example. A lesson is a board as much as a demonstration, and one
/// whose example stops the moment the rule has fired leaves the player looking
/// at a half finished puzzle and guessing what the rule was for. The search
/// finds the shortest way through, so what is shown is the rule doing the job
/// it is there to do.
///
/// A board the search cannot finish keeps the example it already has, because
/// some lessons are demonstrations of something that cannot be finished and
/// saying so is better than dropping the example on the floor.
fn write_demos() {
    use gallery::lessons;
    use rules::map_solved;

    const GALLERY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/levels/gallery.json");

    let mut written = 0usize;
    let mut kept = Vec::new();
    let mut entries = lessons().to_vec();
    for (index, entry) in entries.iter_mut().enumerate() {
        let map = gallery::lesson_map(index);
        let Some(route) = solve_path(&map, DEFAULT_STATE_BUDGET) else {
            kept.push(entry.name.clone());
            continue;
        };
        let mut state = rules::initial_state(&map);
        for step in &route {
            let Some(outcome) = rules::play(&map, &state, *step) else {
                break;
            };
            state = outcome.state;
        }
        if !map_solved(&map, &state) {
            kept.push(entry.name.clone());
            continue;
        }
        println!("{:<18} {:>2} moves", entry.name, route.len());
        entry.demo = route;
        written += 1;
    }

    let text = match nightshade::prelude::serde_json::to_string_pretty(&entries) {
        Ok(text) => text,
        Err(error) => {
            println!("the gallery will not serialize: {error}");
            return;
        }
    };
    if let Err(error) = std::fs::write(GALLERY, text + "\n") {
        println!("the gallery will not save: {error}");
        return;
    }
    println!("{written} worked examples written");
    if !kept.is_empty() {
        println!("no solution, so their example stands: {}", kept.join(", "));
    }
}

/// Plays the route the search hands back for a map and reports whether it
/// actually finishes it. A solution the game is about to play out in front of
/// somebody has to be a real one, and the only way to know is to make the
/// moves.
fn route_report(map: &schema::Map, budget: usize) -> String {
    use rules::{expansions, initial_state, map_solved};

    let Some(route) = solve_path(map, budget) else {
        return "no route recovered".to_string();
    };
    let mut state = initial_state(map);
    for (index, step) in route.iter().enumerate() {
        let Some((_, outcome)) = expansions(map, &state)
            .into_iter()
            .find(|(candidate, _)| candidate == step)
        else {
            return format!("route stalls at move {}", index + 1);
        };
        state = outcome.state;
    }
    if map_solved(map, &state) {
        format!("route of {} plays out", route.len())
    } else {
        "route does not finish the map".to_string()
    }
}

/// Runs the static pass and the exhaustive search over every shipped map. The
/// campaign is data, so this is the check that the data is sound.
fn analyze_campaign(budget: usize) {
    use solver::Search;

    for index in 0..map_count() {
        let map = load_map(index);
        let issues = validate(&map);
        let report = if issues.is_empty() {
            format!(
                "{}  ·  {}",
                solve(&map, budget).describe(),
                route_report(&map, budget)
            )
        } else {
            issues
                .iter()
                .map(|issue| issue.describe())
                .collect::<Vec<String>>()
                .join("; ")
        };
        println!(
            "{:>2}. {:<14} {} floors  crates {} goals {}  par {:<3} {:<6} {report}",
            index + 1,
            map.name,
            map.floors.len(),
            map.crates.len(),
            map.goals.len(),
            map.par,
            Search::new(&map, budget).engine(),
        );
    }
}

/// Generates maps from every preset and reports what came back. The generator
/// only returns boards the solver has already finished, so this is a check on
/// how often it finds one and how hard the result is.
fn analyze_generator(count: usize) {
    for index in 0..PRESET_COUNT {
        let recipe: Recipe = preset(index);
        for attempt in 0..count {
            match generator::generate(&recipe) {
                Some(map) => println!(
                    "preset {index} run {attempt}: {} floors  crates {} goals {}  par {}",
                    map.floors.len(),
                    map.crates.len(),
                    map.goals.len(),
                    map.par
                ),
                None => println!("preset {index} run {attempt}: no solvable map found"),
            }
        }
    }

    // Every setting of the hazard dial has to be able to produce something, or
    // it is a control that only ever answers no.
    for (index, stage) in generator::HAZARD_STAGES.iter().enumerate() {
        let mut recipe = Recipe {
            floor_width: 9,
            floor_height: 8,
            ..Default::default()
        };
        generator::apply_hazards(&mut recipe, index);
        let found = (0..count)
            .filter(|_| generator::generate(&recipe).is_some())
            .count();
        println!("hazards {:<14} {found} of {count} runs solved", stage.name);
    }

    // Every setting of the shape dials has to be able to produce something too.
    // A storey or a side floor multiplies the board, and a board the search
    // cannot finish inside its budget is a dial that only ever answers no.
    // The last of these is the largest board the setup screen can ask for, which
    // is the one that decides whether the dials are controls or traps.
    for (layers, wings, width, height, notch) in [
        // The smallest floor the setup screen offers, which is the one a budget
        // scaled against a larger board rounds away to nothing.
        (1, 0, 7, 7, 2),
        (1, 0, 8, 7, 2),
        (2, 0, 8, 7, 2),
        (3, 0, 8, 7, 2),
        (1, 1, 8, 7, 2),
        (1, 2, 8, 7, 2),
        (2, 1, 10, 9, 3),
        (3, 2, 20, 14, 5),
    ] {
        let mut recipe = Recipe {
            layers,
            wings,
            floor_width: width,
            floor_height: height,
            ..Default::default()
        };
        generator::apply_complexity(&mut recipe, notch);
        let found = (0..count)
            .filter(|_| generator::generate(&recipe).is_some())
            .count();
        println!(
            "shape  storeys {layers} wings {wings} floor {width}x{height} notch {notch}   {found} of {count} runs solved"
        );
    }

    check_run(count.max(6));
}

/// Plays a run out on paper: board after board, each one asked for by the run
/// that has cleared the ones before it. A run that keeps handing out the same
/// board is a demonstration rather than a run, so the reading that matters is
/// whether the weights climb and whether the dials can still produce a board
/// once they have.
fn check_run(boards: usize) {
    use crate::schema::complexity;

    let base = Recipe::default();
    let mut reached = 0;
    for cleared in 0..boards {
        let recipe = generator::escalate(&base, cleared, reached);
        let Some(map) = generator::generate(&recipe) else {
            println!("run board {}: nothing at these settings", cleared + 1);
            break;
        };
        reached = complexity(&map);
        println!(
            "run board {:>2}: {} by {}  crates {}  goals {}  par {:<3} weight {reached}",
            cleared + 1,
            map.floor_width,
            map.floor_height,
            map.crates.len(),
            map.goals.len(),
            map.par,
        );
    }
}
