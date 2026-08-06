//! The long searches, run while the game keeps running. Proving a puzzle is a
//! walk over positions and a big board is a long one, so doing it on the frame
//! that asked for it stops the window dead. Everything that asks for one lands
//! here instead. A slice of each frame goes into whichever search is waiting,
//! and the answer is handed back on whatever frame it comes out. It is the same
//! answer either way, and a game that keeps drawing while it waits.

use crate::ecs::{Making, MapOrigin, MapRequest, Screen, SokobanResources};
use crate::generator::{Outcome, Run, escalate};
use crate::schema::{Map, complexity};
use crate::shortcut::skipped;
use crate::solver::{DEFAULT_STATE_BUDGET, Progress, Search};
use crate::systems::world::{build, playback};
use nightshade::prelude::*;

/// How many positions to walk per frame. Enough that a small board is answered
/// before the button has finished animating, and little enough that a big one
/// costs a fraction of a frame rather than the frame.
const SLICE: usize = 20_000;

/// The same, kept in step with whatever pace the game is being watched at.
/// Fast forwarding a run means the board after this one is wanted sooner, so
/// the search is given more of the frame to find it in and the player waits on
/// the board rather than on the answer.
fn slice(game: &SokobanResources) -> usize {
    SLICE * game.pace() as usize
}

/// How far a search will go before giving up, now that giving it longer costs
/// nobody a frame. High enough to decide every board the game ships, and held
/// there, because a search that runs the machine out of memory is worse than
/// one that says it does not know.
const BUDGET: usize = DEFAULT_STATE_BUDGET;

/// How often a search says how far it has got. Often enough to read as work
/// happening, and rarely enough that the line is not rebuilt every frame.
const SAY_EVERY: f32 = 0.2;

/// A search in progress, and what is waiting on it.
pub enum Work {
    /// A board being generated, and what it is for. The run is the largest
    /// thing any of these carries, so it is boxed and the rest are not.
    Making(Box<Run>, Making),
    /// The board in play, being searched for a route to play back.
    Solving(Map, Search),
    /// The board in the editor, being searched so the author can be told
    /// whether it holds up.
    Analysing(Map, Search),
}

/// Whether a search survives arriving at this screen. A search belongs to
/// whatever asked for it, so going somewhere with nothing to do with it is an
/// answer to the question. Going somewhere it still concerns is not, and a run
/// being generated has to live through a pause.
pub fn survives(work: &Work, screen: Screen) -> bool {
    match work {
        Work::Making(..) => matches!(
            screen,
            Screen::InGame | Screen::Paused | Screen::RandomSetup | Screen::MapComplete
        ),
        Work::Solving(..) => matches!(screen, Screen::InGame | Screen::Paused),
        Work::Analysing(..) => matches!(screen, Screen::Editor),
    }
}

/// Whether a board is being made right now. A screen that asked for one has to
/// be able to say so, or the button it was asked with looks like it did
/// nothing.
pub fn making(game: &SokobanResources) -> bool {
    matches!(game.work, Some(Work::Making(..)))
}

/// Starts making a board. Another board already being made is left alone, so
/// leaning on the button does not throw away what has been done. A board being
/// worked out is not left alone, because the board it was for is finished with.
pub fn make(game: &mut SokobanResources, making: Making) {
    if matches!(game.work, Some(Work::Making(..))) {
        return;
    }
    match making {
        Making::RunStart => {
            game.endless_cleared = 0;
            game.endless_weight = 0;
        }
        Making::RunNext => game.endless_cleared += 1,
        Making::Single => {}
    }
    // A single board is the one the dials describe. A board of a run is that
    // board plus everything the run has climbed since it started.
    let recipe = match making {
        Making::Single => game.recipe,
        Making::RunStart | Making::RunNext => {
            escalate(&game.recipe, game.endless_cleared, game.endless_weight)
        }
    };
    game.work = Some(Work::Making(Box::new(Run::new(&recipe)), making));
    game.random_status = "generating".to_string();
    game.notice = "generating the next board".to_string();
}

/// Points the search at the board in play and plays its answer out when it has
/// one.
pub fn solve(game: &mut SokobanResources) {
    if game.work.is_some() {
        return;
    }
    let map = game.map.clone();
    let search = Search::new(&map, BUDGET);
    game.work = Some(Work::Solving(map, search));
    game.notice = "working the board out".to_string();
}

/// Points the search at the board in the editor.
pub fn analyse(game: &mut SokobanResources) {
    if game.work.is_some() {
        return;
    }
    let map = game.editor.map.clone();
    let search = Search::new(&map, BUDGET);
    game.work = Some(Work::Analysing(map, search));
    game.editor.status = "searching".to_string();
}

pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    match game.work.take() {
        None => {}
        Some(Work::Making(run, making)) => advance_making(game, world, run, making),
        Some(Work::Solving(map, search)) => advance_solving(game, world, map, search),
        Some(Work::Analysing(map, search)) => advance_analysing(game, map, search),
    }
}

fn advance_making(
    game: &mut SokobanResources,
    world: &mut World,
    mut run: Box<Run>,
    making: Making,
) {
    match run.advance(slice(game)) {
        Outcome::Working => {
            if game.elapsed - game.work_said_at >= SAY_EVERY {
                game.work_said_at = game.elapsed;
                let progress = format!(
                    "generating   ·   {} boards tried, {} positions walked",
                    run.attempted(),
                    run.explored()
                );
                game.random_status = progress.clone();
                game.notice = progress;
            }
            game.work = Some(Work::Making(run, making));
        }
        Outcome::Ready(map) => hand_over(game, world, *map, making),
        Outcome::Barren => {
            let cleared = game.endless_cleared;
            game.random_status = "no solvable map at these settings".to_string();
            game.notice = match making {
                Making::RunNext => format!("{cleared} cleared, and no board after this one"),
                _ => "no solvable map at these settings".to_string(),
            };
            if matches!(making, Making::RunStart) {
                next_state(world, Screen::RandomSetup);
            }
        }
    }
}

/// Where a finished board goes. A single board and the first of a run are both
/// something to go to. The next board of a run already going replaces the one
/// under the player without a screen in between.
fn hand_over(game: &mut SokobanResources, world: &mut World, map: Map, making: Making) {
    let origin = match making {
        Making::Single => MapOrigin::Random,
        Making::RunStart | Making::RunNext => MapOrigin::Endless,
    };
    let weight = complexity(&map);
    if matches!(making, Making::RunStart | Making::RunNext) {
        game.endless_weight = weight;
    }
    game.random_status = format!("{}  ·  par {}", map.hint, map.par);
    match making {
        Making::RunNext => {
            let cleared = game.endless_cleared;
            build::start_map(game, world, MapRequest { map, origin });
            // Arming first, because it has a line of its own to say and what
            // the board is is the more useful of the two to be left holding.
            playback::arm_solution(game);
            game.notice = format!(
                "board {}, {cleared} cleared  ·  weight {weight}",
                cleared + 1
            );
        }
        _ => {
            game.notice.clear();
            game.pending = Some(MapRequest { map, origin });
            next_state(world, Screen::InGame);
        }
    }
}

fn advance_solving(game: &mut SokobanResources, world: &mut World, map: Map, mut search: Search) {
    // The board changing out from under a search makes its answer a route
    // through somewhere else, so the search goes with the board it was for.
    if map != game.map {
        return;
    }
    match search.advance(&map, slice(game)) {
        // Nothing is said per frame. A count of positions walked is not
        // progress a player can do anything with, and writing one every frame
        // would talk over whatever the board itself had to say.
        Progress::Running => game.work = Some(Work::Solving(map, search)),
        Progress::Solved(script) => {
            game.notice = format!("solving in {} moves", script.len());
            playback::start(game, world, script, false);
        }
        Progress::Unsolvable => game.notice = "this board cannot be finished".to_string(),
        Progress::Exhausted => game.notice = "no solution found in the time available".to_string(),
    }
}

fn advance_analysing(game: &mut SokobanResources, map: Map, mut search: Search) {
    match search.advance(&map, slice(game)) {
        Progress::Running => {
            game.editor.status = format!("searching   ·   {} positions", search.explored());
            game.work = Some(Work::Analysing(map, search));
        }
        Progress::Solved(route) => {
            // An author is told the same two things the generator decides on:
            // that the board can be finished, and that finishing it needs all of
            // it.
            let short = skipped(&map, &route);
            if map == game.editor.map {
                game.editor.map.par = route.len() as u32;
            }
            game.editor.status = if short.is_empty() {
                format!("solvable in {} moves", route.len())
            } else {
                format!(
                    "solvable in {} moves, but short circuited: {}",
                    route.len(),
                    short.describe()
                )
            };
        }
        Progress::Unsolvable => game.editor.status = "no solution exists".to_string(),
        Progress::Exhausted => {
            game.editor.status = "undecided: run sokoban analyze for a longer search".to_string()
        }
    }
}
