//! The shipped boards. They live in the campaign file rather than in here:
//! this is only the two questions the rest of the game asks about them.

use crate::campaign::campaign;
use crate::schema::Map;

pub fn map_count() -> usize {
    campaign().levels.len()
}

/// One board, by its place in the campaign. A level number past the end is a
/// caller that has lost track of how many there are, and an empty board says so
/// louder than a wrong one would.
pub fn load_map(level: usize) -> Map {
    campaign()
        .levels
        .get(level)
        .map(|entry| entry.map.clone())
        .unwrap_or_default()
}
