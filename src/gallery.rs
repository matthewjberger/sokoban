//! A gallery of mechanics, each one on a board of its own with nothing else on
//! it. These are maps like any other, built from the same schema, but they are
//! demonstrations rather than puzzles. A lesson is free to be unsolvable, or
//! solvable in one push, because its job is to show what a rule does.
//!
//! All of it is data in a file beside the campaign, so a lesson is written by
//! editing that rather than by editing this.

use crate::rules::Step;
use crate::schema::{Map, map_relink};

/// What a lesson is about. Who you are and what the board does to you are two
/// different questions, and a list that mixes them reads as one long list of
/// unrelated things.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Topic {
    /// Who you are, and what that one can do with a crate.
    Character,
    /// What the board itself does.
    Mechanic,
}

impl Topic {
    pub const ALL: [Topic; 2] = [Topic::Character, Topic::Mechanic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Character => "WHO YOU ARE",
            Self::Mechanic => "THE BOARD",
        }
    }
}

/// One lesson: a board with one thing on it, what that thing is, what to try,
/// and the worked example played out a beat at a time when asked for.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Lesson {
    pub topic: Topic,
    pub name: String,
    /// What the mechanic is, in a sentence.
    pub blurb: String,
    /// What to do on this board to see it happen.
    pub practice: String,
    pub map: Map,
    pub demo: Vec<Step>,
}

const LESSONS_JSON: &str = include_str!("../levels/gallery.json");

/// The lessons, parsed once. A file that will not parse is a broken build
/// rather than a quietly empty gallery, so this says so plainly.
pub fn lessons() -> &'static [Lesson] {
    static PARSED: std::sync::OnceLock<Vec<Lesson>> = std::sync::OnceLock::new();
    PARSED.get_or_init(|| {
        nightshade::prelude::serde_json::from_str(LESSONS_JSON)
            .expect("the gallery file does not parse")
    })
}

/// One lesson, by its place in the list. Past the end is a caller that has lost
/// count, and the last lesson is a better answer than a panic.
pub fn lesson(index: usize) -> &'static Lesson {
    let list = lessons();
    &list[index.min(list.len().saturating_sub(1))]
}

/// A lesson's board, with the derived halves of it filled in. Relinking pairs
/// the pads and finds the emitters, and a board read from a file has not had it
/// done.
pub fn lesson_map(index: usize) -> Map {
    let mut map = lesson(index).map.clone();
    map_relink(&mut map);
    map
}
