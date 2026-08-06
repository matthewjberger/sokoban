//! Fullscreen effects that answer to what just happened in the game rather than
//! to the scene. There is one. Undoing a move pulls the picture back through
//! itself for a moment, so taking a move back looks like taking it back.

use crate::ecs::{Settings, SokobanResources};
use nightshade::prelude::*;

/// How long one rewind takes to play out.
const REWIND_SECONDS: f32 = 0.55;
/// How far apart the colour channels are thrown at the peak of it, which is the
/// part that reads as time coming apart.
const SPLIT: f32 = 0.34;
/// How much colour is pulled out of the picture at the peak.
const DRAIN: f32 = 0.38;
/// How hard the frame closes in at the edges.
const CLOSE_IN: f32 = 0.28;

/// The slowest and fastest the triggers will take the clock.
const SLOWEST: f32 = 0.5;
const FASTEST: f32 = 2.0;

/// Holds the clock wherever the triggers and the run speed are holding it. The
/// engine owns the scale, so this only says what it should be, and it says one
/// every frame rather than only on a change, because letting go has to put it
/// back.
///
/// A run being fast forwarded moves the clock rather than any one timer, so the
/// moves, the beats between them and the wait between boards all come along
/// together and nothing has to be told about the others.
pub fn scale_time(game: Res<SokobanResources>, world: &mut World) {
    // How far each trigger is pulled rather than whether it is down, so the
    // clock is something to lean on. Let go and it is one again, because
    // nothing here latches.
    let slow = trigger_pull(world, gilrs::Axis::LeftZ, gilrs::Button::LeftTrigger2);
    let fast = trigger_pull(world, gilrs::Axis::RightZ, gilrs::Button::RightTrigger2);
    // What is left after the two are set against each other, so pulling both
    // the same amount really is pulling neither and the clock is one again.
    let lean = fast - slow;
    let held = if lean >= 0.0 {
        1.0 + lean * (FASTEST - 1.0)
    } else {
        1.0 + lean * (1.0 - SLOWEST)
    };
    let wanted = held * game.pace();
    if (time_scale(world) - wanted).abs() > f32::EPSILON {
        set_time_scale(world, wanted);
    }
}

/// How far a trigger is pulled, from nothing to all the way. A pad that reports
/// its triggers as buttons and not as axes still answers, because a button that
/// is down is a trigger that is pulled.
fn trigger_pull(world: &World, axis: gilrs::Axis, button: gilrs::Button) -> f32 {
    // A trigger at rest does not always read zero, and this one is on the game
    // clock, so a pad breathing on it would shake every animation on the board.
    // Below the notch it is not pulled at all.
    const NOTCH: f32 = 0.12;
    with_active_gamepad(world, |pad| {
        let pull = pad.value(axis).clamp(0.0, 1.0);
        if pull > NOTCH {
            (pull - NOTCH) / (1.0 - NOTCH)
        } else if pad.is_pressed(button) {
            1.0
        } else {
            0.0
        }
    })
    .unwrap_or(0.0)
}

/// Starts a rewind, if the player wants them.
pub fn begin_rewind(game: &mut SokobanResources) {
    if game.settings.rewind_effect {
        game.rewind = 1.0;
    }
}

/// Drives the rewind and puts the picture back afterwards. The curve peaks
/// immediately and eases out, because the moment worth marking is the instant
/// the move comes back rather than the settling after it.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if game.rewind <= 0.0 {
        return;
    }

    game.rewind = (game.rewind - world.res::<Time>().delta_time / REWIND_SECONDS).max(0.0);
    // Sharp on, slow off. A linear fade reads as a dip in brightness rather
    // than as a jolt the picture recovers from.
    let strength = game.rewind * game.rewind;

    let grading = &mut world.res_mut::<RenderSettings>().color_grading;
    grading.chromatic_aberration = SPLIT * strength;
    grading.saturation = 1.0 - DRAIN * strength;
    grading.vignette_intensity = CLOSE_IN * strength;
    grading.vignette_radius = 0.85 - 0.18 * strength;
    grading.vignette_smoothness = 0.45;
    grading.contrast = 1.0 + 0.12 * strength;
    grading.brightness = -0.03 * strength;
}

/// Pushes the switches the player has set onto the renderer. Reading them back
/// out of one place means the screen that edits them never has to know which
/// setting lives where.
pub fn apply(settings: &Settings, world: &mut World) {
    let render = world.res_mut::<RenderSettings>();
    render.bloom_enabled = settings.bloom;
    render.ssr_enabled = settings.reflections;
    render.ssao_enabled = settings.ambient_occlusion;
    render.water_enabled = settings.water;
    if !settings.rewind_effect {
        render.color_grading.chromatic_aberration = 0.0;
        render.color_grading.saturation = 1.0;
        render.color_grading.vignette_intensity = 0.0;
        render.color_grading.contrast = 1.0;
        render.color_grading.brightness = 0.0;
    }
}
