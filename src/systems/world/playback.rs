//! Playing a list of moves rather than reading one from the controls. A worked
//! example in the gallery and a solution the search found are the same thing to
//! everything below this, so both arrive here and are played the same way.

use crate::ecs::{MapOrigin, SokobanResources};
use crate::rules::Step;
use crate::systems::world::motion::is_busy;
use nightshade::prelude::*;

/// How long to wait between moves, so a viewer can follow what just happened
/// before the next thing happens.
const BEAT: f32 = 0.22;
/// The pause before the first move, so the board can be read as it started.
pub const LEAD_IN: f32 = 0.85;
/// The pause on the finished board before a looping run starts over.
const HOLD: f32 = 2.4;

/// Hands a list of moves to the board and starts it from the beginning. The
/// board is reset first, because a run that starts halfway through a position
/// is not the run that was worked out.
pub fn start(game: &mut SokobanResources, world: &mut World, script: Vec<Step>, looping: bool) {
    crate::systems::input::restart(game, world);
    game.playback.script = script;
    game.playback.looping = looping;
    game.playback.playing = true;
    game.playback.step = 0;
    game.playback.timer = LEAD_IN;
}

pub fn stop(game: &mut SokobanResources) {
    game.playback.playing = false;
}

/// Points the search at the board, on a run that has been set to solve itself.
/// The search runs a slice a frame and the answer starts playing on whatever
/// frame it comes out. Reaching for the controls takes the board back.
pub fn arm_solution(game: &mut SokobanResources) {
    if !game.settings.auto_solve || !matches!(game.origin, MapOrigin::Endless) {
        return;
    }
    crate::systems::world::work::solve(game);
}

/// Plays the run a beat at a time, waiting for each move to finish before
/// starting the next so the board stays readable the whole way through.
pub fn advance(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !game.playback.playing || is_busy(world) {
        return;
    }

    game.playback.timer -= world.res::<Time>().delta_time;
    if game.playback.timer > 0.0 {
        return;
    }

    if game.playback.step >= game.playback.script.len() {
        // A demonstration goes round again so a lesson left alone keeps showing
        // what it is for. A solution has been watched and is done.
        if game.playback.looping {
            let script = std::mem::take(&mut game.playback.script);
            start(game, world, script, true);
        } else {
            game.playback.playing = false;
        }
        return;
    }

    match game.playback.script[game.playback.step] {
        Step::Go(direction) => crate::systems::input::step(game, world, direction),
        Step::Drag(direction) => crate::systems::input::drag(game, world, direction),
        Step::Ride(direction) => crate::systems::input::ride(game, world, direction),
        Step::Take(index) => crate::systems::input::take(game, world, index),
        Step::Handle => crate::systems::input::handle(game, world),
    }
    game.playback.step += 1;
    // The wait after the last move is the one that matters most, since it is
    // the only moment the finished board is on screen.
    game.playback.timer = if game.playback.step >= game.playback.script.len() {
        HOLD
    } else {
        BEAT
    };
}
