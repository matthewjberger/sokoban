//! The campaign, as data. The boards, the order they stand in, what each one
//! waits on, and the four areas they are grouped into all live in one JSON file
//! beside the code rather than in the code.
//!
//! It is embedded rather than read from disk, because the browser build has no
//! disk to read from and a board that only exists on a desktop is not a board
//! the game ships. Embedding is a build time decision about where the bytes
//! come from. It is still one file to edit and no Rust to write, and the editor
//! can write it.

use crate::schema::{Map, Skin};
use nightshade::prelude::serde_json;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// One region of the overworld: where its floor sits in the lattice, what it is
/// called, what it is paved with, and how many doors stand in it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub blurb: String,
    pub slot: (i32, i32),
    pub skin: Skin,
    pub doors: usize,
}

/// One board and what has to be finished before its door will open.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    /// Boards that have to be cleared first, by their place in this list. A
    /// board naming nothing is open from the start, and there is one of those.
    #[serde(default)]
    pub requires: Vec<usize>,
    pub map: Map,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Campaign {
    pub areas: Vec<Area>,
    pub levels: Vec<Level>,
}

const CAMPAIGN_JSON: &str = include_str!("../levels/campaign.json");

/// The campaign, parsed once. A file that will not parse is a broken build
/// rather than a broken game, so this says so plainly instead of quietly
/// shipping an empty campaign.
pub fn campaign() -> &'static Campaign {
    static PARSED: OnceLock<Campaign> = OnceLock::new();
    PARSED.get_or_init(|| {
        serde_json::from_str(CAMPAIGN_JSON).expect("the campaign file does not parse")
    })
}
