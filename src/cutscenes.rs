//! The story's cutscenes. There are no animated actors on this board, only the
//! same blocks the puzzles are made of, so these are told with the camera, the
//! letterbox and the words, as a shot that moves over the depot while somebody
//! says what the place is for.

use crate::story::{FLOOR_HEIGHT, FLOOR_WIDTH, areas};
use nightshade::prelude::*;

fn shot(eye: Vec3, target: Vec3, field_of_view: f32) -> CutsceneShot {
    CutsceneShot::new(eye, target).with_field_of_view(field_of_view)
}

/// The middle of an area in world space, which is what every shot in these
/// scenes is aimed at.
fn heart(area: usize) -> Vec3 {
    let (column, row) = areas()[area].slot;
    Vec3::new(
        column as f32 * FLOOR_WIDTH as f32 + FLOOR_WIDTH as f32 * 0.5,
        0.0,
        row as f32 * FLOOR_HEIGHT as f32 + FLOOR_HEIGHT as f32 * 0.5,
    )
}

/// The opening. It sweeps the yard the player is about to be standing in and
/// hands over with the letterbox pulling back.
///
/// The depot is closing, and every other line in the game leans on that. The
/// rooms open in order because the count works through the building in order,
/// and the last wing is the way out.
pub fn opening() -> Cutscene {
    let yard = heart(0);
    Cutscene::new("The Depot")
        .fade_in(0.0, 1.4)
        .letterbox_in(0.0, 1.0)
        .title(0.8, 3.2, "THE DEPOT")
        .handheld(0.0, 17.0, 0.04, 0.03, 1.2)
        .camera_path(
            0.0,
            17.0,
            EasingFunction::SineInOut,
            vec![
                shot(yard + Vec3::new(0.0, 26.0, 26.0), yard, 50.0),
                shot(yard + Vec3::new(-9.0, 12.0, 14.0), yard, 46.0),
                shot(yard + Vec3::new(6.0, 9.0, 12.0), yard, 44.0),
                shot(yard + Vec3::new(0.0, 15.0, 16.0), yard, 48.0),
            ],
        )
        .dialogue(
            1.6,
            4.2,
            Some("Keeper"),
            "They are closing the depot. Tonight, if the count comes out.",
        )
        .dialogue(
            6.0,
            4.6,
            Some("Keeper"),
            "Every crate has a square it is meant to be standing on. None of them are.",
        )
        .dialogue(
            10.8,
            4.4,
            Some("Keeper"),
            "Six wings. Work through them and the doors open ahead of you.",
        )
        .dialogue(15.4, 2.6, Some("Keeper"), "Start with the yard.")
        .letterbox_out(15.6, 1.4)
}

/// Played when a wing opens, which is the only reward the depot has to give.
pub fn area_opens(area: usize) -> Cutscene {
    let place = heart(area);
    let name = areas()[area].name.as_str();
    let (line, after) = match area {
        1 => (
            "The freezer. They kept it running long after there was anything in it worth chilling.",
            "Nothing in there stays where you put it.",
        ),
        2 => (
            "The quarry. The floor here is what they dug through to lay the foundations.",
            "Mind the holes, and what stands at the bottom of them.",
        ),
        3 => (
            "The vault. Everything worth locking up ended its days in here.",
            "Its doors answer to something other than you.",
        ),
        4 => (
            "The gantry. This is the way out, and there is nothing under it any more.",
            "Whatever you finish here leaves the building.",
        ),
        _ => (
            "The lamp room. Every light in the depot was cut, cased and carried out of here.",
            "The stones still work. Set one down somewhere it can see, and stand in what it throws.",
        ),
    };

    Cutscene::new(name)
        .letterbox_in(0.0, 0.7)
        .handheld(0.0, 9.4, 0.04, 0.03, 1.3)
        .title(0.6, 2.8, name)
        .camera(
            0.0,
            9.0,
            EasingFunction::SineInOut,
            shot(place + Vec3::new(0.0, 22.0, 22.0), place, 50.0),
            shot(place + Vec3::new(0.0, 13.0, 15.0), place, 45.0),
        )
        .dialogue(1.0, 4.4, Some("Keeper"), line)
        .dialogue(5.6, 3.4, Some("Keeper"), after)
        .letterbox_out(8.2, 1.2)
}

/// The close, when the last room is done.
pub fn ending() -> Cutscene {
    let gantry = heart(4);
    Cutscene::new("The Count Comes Out")
        .letterbox_in(0.0, 0.8)
        .title(0.8, 3.4, "EVERY CRATE IN ITS PLACE")
        .handheld(0.0, 13.0, 0.03, 0.03, 1.1)
        .camera(
            0.0,
            12.0,
            EasingFunction::CubicInOut,
            shot(gantry + Vec3::new(0.0, 12.0, 14.0), gantry, 44.0),
            shot(gantry + Vec3::new(0.0, 42.0, 36.0), gantry, 52.0),
        )
        .dialogue(
            1.2,
            4.4,
            Some("Keeper"),
            "Every room. Not one crate left standing where it began.",
        )
        .dialogue(
            6.0,
            4.4,
            Some("Keeper"),
            "The count comes out. They can close it now, and it closes tidy.",
        )
        .dialogue(
            10.6,
            2.6,
            Some("Keeper"),
            "Walk it once more before you go.",
        )
        .fade_out(11.6, 2.0)
        .letterbox_out(11.6, 1.6)
}
