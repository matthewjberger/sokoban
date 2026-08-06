//! World colours, chosen by a map's [`Skin`]. The schema names an intent and
//! this table is where that intent turns into pixels, so a reskin is a data edit rather
//! than a code edit.

use crate::schema::{Character, GemColor, Skin};
use nightshade::prelude::*;

/// Every colour the world builder needs, chosen by the level's [`Skin`]. The
/// schema names an intent and this table is where that intent becomes pixels,
/// so a reskin never touches the builder.
pub struct Palette {
    pub floor_light: [f32; 4],
    pub floor_dark: [f32; 4],
    pub plinth: [f32; 4],
    pub plinth_rim: [f32; 4],
    pub wall: [f32; 4],
    pub wall_cap: [f32; 4],
    pub ice: [f32; 4],
    /// What colour light turns as it travels through the ice, which is what
    /// makes a thick sheet read as deeper than a thin one.
    pub ice_tint: [f32; 3],
    /// The bed under a frozen or flooded square, so there is something for the
    /// surface to refract rather than empty space.
    pub bed: [f32; 4],
    pub water_shallow: [f32; 3],
    pub water_deep: [f32; 3],
    pub pit: [f32; 4],
    pub crate_body: [f32; 4],
    pub crate_done: [f32; 4],
    pub crate_band: [f32; 4],
    pub crate_cap: [f32; 4],
    pub goal: [f32; 4],
    pub goal_done: [f32; 4],
    pub plate: [f32; 4],
    pub gate: [f32; 4],
    pub elevator: [f32; 4],
    pub one_way: [f32; 4],
    pub belt: [f32; 4],
    pub fragile: [f32; 4],
    pub switch: [f32; 4],
    pub brittle: [f32; 4],
    pub beam: [f32; 3],
    pub tower: [f32; 4],
    pub player_head: [f32; 4],
    pub player_trim: [f32; 4],
    pub portals: [[f32; 4]; 3],
    pub atmosphere: Atmosphere,
    pub fog: [f32; 3],
    pub warm_light: Vec3,
    pub cool_light: Vec3,
}

pub fn palette_for(skin: Skin) -> Palette {
    match skin {
        Skin::Warehouse => Palette {
            floor_light: FLOOR_LIGHT,
            floor_dark: FLOOR_DARK,
            plinth: PLINTH,
            plinth_rim: PLINTH_RIM,
            wall: WALL,
            wall_cap: WALL_CAP,
            ice: ICE,
            ice_tint: [0.36, 0.62, 0.78],
            bed: [0.1, 0.13, 0.17, 1.0],
            water_shallow: [0.14, 0.42, 0.5],
            water_deep: [0.02, 0.1, 0.18],
            pit: PIT,
            crate_body: CRATE_BODY,
            crate_done: CRATE_BODY_DONE,
            crate_band: CRATE_BAND,
            crate_cap: CRATE_PLATE,
            goal: GOAL,
            goal_done: GOAL_DONE,
            plate: PLATE,
            gate: GATE,
            elevator: ELEVATOR,
            one_way: ONE_WAY,
            belt: BELT,
            fragile: FRAGILE,
            switch: SWITCH,
            brittle: BRITTLE,
            beam: [2.6, 0.5, 0.4],
            tower: [0.7, 0.32, 0.3, 1.0],
            player_head: PLAYER_HEAD,
            player_trim: PLAYER_TRIM,
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::Sunset,
            fog: [0.32, 0.22, 0.26],
            warm_light: Vec3::new(1.0, 0.72, 0.42),
            cool_light: Vec3::new(0.42, 0.6, 1.0),
        },
        Skin::Glacier => Palette {
            floor_light: [0.74, 0.82, 0.88, 1.0],
            floor_dark: [0.62, 0.72, 0.8, 1.0],
            plinth: [0.2, 0.26, 0.34, 1.0],
            plinth_rim: [0.13, 0.18, 0.25, 1.0],
            wall: [0.35, 0.44, 0.55, 1.0],
            wall_cap: [0.55, 0.66, 0.76, 1.0],
            ice: [0.76, 0.9, 0.98, 1.0],
            ice_tint: [0.42, 0.72, 0.88],
            bed: [0.08, 0.16, 0.24, 1.0],
            water_shallow: [0.2, 0.55, 0.62],
            water_deep: [0.03, 0.16, 0.26],
            pit: [0.03, 0.06, 0.1, 1.0],
            crate_body: [0.82, 0.6, 0.3, 1.0],
            crate_done: [0.4, 0.9, 0.7, 1.0],
            crate_band: [0.3, 0.28, 0.24, 1.0],
            crate_cap: [0.94, 0.8, 0.55, 1.0],
            goal: [0.35, 0.9, 0.85, 1.0],
            goal_done: [0.5, 1.0, 0.7, 1.0],
            plate: [0.95, 0.72, 0.3, 1.0],
            gate: [0.8, 0.35, 0.45, 1.0],
            elevator: [0.55, 0.85, 0.95, 1.0],
            one_way: [0.98, 0.86, 0.42, 1.0],
            belt: [0.46, 0.72, 0.62, 1.0],
            fragile: [0.66, 0.7, 0.78, 1.0],
            switch: [0.92, 0.52, 0.86, 1.0],
            brittle: [0.62, 0.58, 0.52, 1.0],
            beam: [0.6, 1.6, 2.8],
            tower: [0.5, 0.6, 0.8, 1.0],
            player_head: [0.95, 0.84, 0.72, 1.0],
            player_trim: [0.95, 0.98, 1.0, 1.0],
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::Sky,
            fog: [0.5, 0.6, 0.7],
            warm_light: Vec3::new(0.9, 0.8, 0.7),
            cool_light: Vec3::new(0.5, 0.7, 1.0),
        },
        Skin::Quarry => Palette {
            floor_light: [0.6, 0.56, 0.5, 1.0],
            floor_dark: [0.48, 0.44, 0.39, 1.0],
            plinth: [0.22, 0.19, 0.16, 1.0],
            plinth_rim: [0.14, 0.12, 0.1, 1.0],
            wall: [0.42, 0.37, 0.31, 1.0],
            wall_cap: [0.58, 0.52, 0.44, 1.0],
            ice: [0.7, 0.82, 0.86, 1.0],
            ice_tint: [0.4, 0.58, 0.66],
            bed: [0.09, 0.08, 0.06, 1.0],
            water_shallow: [0.18, 0.38, 0.36],
            water_deep: [0.03, 0.09, 0.1],
            pit: [0.02, 0.02, 0.02, 1.0],
            crate_body: [0.76, 0.52, 0.28, 1.0],
            crate_done: [0.5, 0.86, 0.42, 1.0],
            crate_band: [0.34, 0.22, 0.14, 1.0],
            crate_cap: [0.9, 0.72, 0.44, 1.0],
            goal: [0.95, 0.72, 0.24, 1.0],
            goal_done: [0.6, 1.0, 0.45, 1.0],
            plate: [0.94, 0.6, 0.22, 1.0],
            gate: [0.82, 0.3, 0.24, 1.0],
            elevator: [0.5, 0.76, 0.82, 1.0],
            one_way: [0.98, 0.82, 0.35, 1.0],
            belt: [0.5, 0.64, 0.46, 1.0],
            fragile: [0.66, 0.6, 0.5, 1.0],
            switch: [0.86, 0.5, 0.7, 1.0],
            brittle: [0.54, 0.46, 0.36, 1.0],
            beam: [2.8, 1.1, 0.3],
            tower: [0.72, 0.46, 0.24, 1.0],
            player_head: PLAYER_HEAD,
            player_trim: PLAYER_TRIM,
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::Sunset,
            fog: [0.38, 0.32, 0.25],
            warm_light: Vec3::new(1.0, 0.82, 0.55),
            cool_light: Vec3::new(0.55, 0.62, 0.72),
        },
        Skin::Vault => Palette {
            floor_light: [0.4, 0.36, 0.46, 1.0],
            floor_dark: [0.32, 0.29, 0.38, 1.0],
            plinth: [0.14, 0.12, 0.18, 1.0],
            plinth_rim: [0.09, 0.08, 0.12, 1.0],
            wall: [0.26, 0.22, 0.32, 1.0],
            wall_cap: [0.42, 0.36, 0.5, 1.0],
            ice: [0.6, 0.78, 0.92, 1.0],
            ice_tint: [0.3, 0.5, 0.8],
            bed: [0.05, 0.05, 0.1, 1.0],
            water_shallow: [0.16, 0.3, 0.6],
            water_deep: [0.02, 0.03, 0.12],
            pit: [0.02, 0.02, 0.04, 1.0],
            crate_body: [0.72, 0.44, 0.26, 1.0],
            crate_done: [0.5, 0.9, 0.5, 1.0],
            crate_band: [0.28, 0.18, 0.12, 1.0],
            crate_cap: [0.88, 0.66, 0.4, 1.0],
            goal: [0.6, 0.5, 1.0, 1.0],
            goal_done: [0.55, 1.0, 0.6, 1.0],
            plate: [1.0, 0.6, 0.25, 1.0],
            gate: [0.9, 0.3, 0.35, 1.0],
            elevator: [0.45, 0.8, 0.9, 1.0],
            one_way: [1.0, 0.8, 0.3, 1.0],
            belt: [0.4, 0.72, 0.6, 1.0],
            fragile: [0.5, 0.44, 0.56, 1.0],
            switch: [0.88, 0.44, 0.9, 1.0],
            brittle: [0.46, 0.4, 0.42, 1.0],
            beam: [2.2, 0.6, 2.4],
            tower: [0.6, 0.3, 0.7, 1.0],
            player_head: [0.92, 0.78, 0.64, 1.0],
            player_trim: [0.9, 0.95, 1.0, 1.0],
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::Nebula,
            fog: [0.14, 0.1, 0.2],
            warm_light: Vec3::new(1.0, 0.5, 0.35),
            cool_light: Vec3::new(0.5, 0.4, 1.0),
        },
        Skin::Drift => Palette {
            floor_light: [0.56, 0.54, 0.68, 1.0],
            floor_dark: [0.44, 0.42, 0.56, 1.0],
            plinth: [0.16, 0.15, 0.24, 1.0],
            plinth_rim: [0.1, 0.09, 0.16, 1.0],
            wall: [0.3, 0.28, 0.42, 1.0],
            wall_cap: [0.5, 0.48, 0.66, 1.0],
            ice: [0.66, 0.8, 0.96, 1.0],
            ice_tint: [0.34, 0.56, 0.86],
            bed: [0.04, 0.04, 0.09, 1.0],
            water_shallow: [0.18, 0.34, 0.62],
            water_deep: [0.02, 0.04, 0.14],
            pit: [0.01, 0.01, 0.03, 1.0],
            crate_body: [0.74, 0.5, 0.32, 1.0],
            crate_done: [0.48, 0.92, 0.56, 1.0],
            crate_band: [0.3, 0.2, 0.16, 1.0],
            crate_cap: [0.9, 0.7, 0.48, 1.0],
            goal: [0.5, 0.86, 1.0, 1.0],
            goal_done: [0.55, 1.0, 0.7, 1.0],
            plate: [0.98, 0.7, 0.34, 1.0],
            gate: [0.86, 0.34, 0.42, 1.0],
            elevator: [0.5, 0.84, 0.96, 1.0],
            one_way: [1.0, 0.84, 0.4, 1.0],
            belt: [0.44, 0.74, 0.66, 1.0],
            fragile: [0.6, 0.58, 0.7, 1.0],
            switch: [0.9, 0.5, 0.92, 1.0],
            brittle: [0.5, 0.46, 0.56, 1.0],
            beam: [1.4, 1.0, 2.8],
            tower: [0.5, 0.44, 0.82, 1.0],
            player_head: PLAYER_HEAD,
            player_trim: [0.92, 0.96, 1.0, 1.0],
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::Space,
            fog: [0.1, 0.1, 0.18],
            warm_light: Vec3::new(0.9, 0.8, 1.0),
            cool_light: Vec3::new(0.4, 0.5, 1.0),
        },
        Skin::Grove => Palette {
            floor_light: [0.56, 0.62, 0.44, 1.0],
            floor_dark: [0.44, 0.5, 0.34, 1.0],
            plinth: [0.2, 0.22, 0.16, 1.0],
            plinth_rim: [0.13, 0.15, 0.1, 1.0],
            wall: [0.36, 0.42, 0.28, 1.0],
            wall_cap: [0.52, 0.58, 0.4, 1.0],
            ice: [0.72, 0.86, 0.9, 1.0],
            ice_tint: [0.38, 0.64, 0.7],
            bed: [0.08, 0.11, 0.07, 1.0],
            water_shallow: [0.2, 0.5, 0.42],
            water_deep: [0.03, 0.14, 0.12],
            pit: [0.03, 0.04, 0.02, 1.0],
            crate_body: [0.78, 0.56, 0.3, 1.0],
            crate_done: [0.56, 0.94, 0.46, 1.0],
            crate_band: [0.36, 0.26, 0.14, 1.0],
            crate_cap: [0.92, 0.76, 0.48, 1.0],
            goal: [1.0, 0.82, 0.3, 1.0],
            goal_done: [0.62, 1.0, 0.44, 1.0],
            plate: [0.96, 0.66, 0.24, 1.0],
            gate: [0.8, 0.34, 0.3, 1.0],
            elevator: [0.52, 0.8, 0.78, 1.0],
            one_way: [0.98, 0.86, 0.4, 1.0],
            belt: [0.5, 0.7, 0.5, 1.0],
            fragile: [0.64, 0.66, 0.52, 1.0],
            switch: [0.88, 0.54, 0.72, 1.0],
            brittle: [0.56, 0.52, 0.4, 1.0],
            beam: [2.4, 1.4, 0.4],
            tower: [0.68, 0.52, 0.28, 1.0],
            player_head: PLAYER_HEAD,
            player_trim: PLAYER_TRIM,
            portals: PORTAL_COLORS,
            atmosphere: Atmosphere::CloudySky,
            fog: [0.46, 0.52, 0.4],
            warm_light: Vec3::new(1.0, 0.9, 0.68),
            cool_light: Vec3::new(0.6, 0.74, 0.6),
        },
    }
}

/// What a character is made of. Who is walking a board is a property of the map
/// rather than of its skin, so none of these move with the palette. A magnet
/// reads as a magnet in a warehouse, a glacier, a quarry or a vault alike, and
/// two members of one party are two colours because they are two classes.
pub fn character_body(character: Character) -> [f32; 4] {
    match character {
        Character::Pusher => PUSHER_BODY,
        Character::Dragger => DRAGGER_BODY,
        Character::Magnet => MAGNET_BODY,
        Character::Swapper => SWAPPER_BODY,
        Character::Wader => WADER_BODY,
        Character::Phaser => PHASER_BODY,
        Character::Warden => WARDEN_BODY,
        Character::Blinker => BLINKER_BODY,
        Character::Breaker => BREAKER_BODY,
    }
}

/// What a gem is made of, which is one colour and a great deal of light. A gem
/// reads as the same gem in every room it turns up in, so none of these move
/// with the skin.
pub fn gem_body(color: GemColor) -> [f32; 4] {
    match color {
        GemColor::Ruby => [0.95, 0.16, 0.28, 1.0],
        GemColor::Amber => [0.98, 0.68, 0.14, 1.0],
        GemColor::Jade => [0.2, 0.9, 0.5, 1.0],
        GemColor::Azure => [0.26, 0.6, 1.0, 1.0],
    }
}

/// The light a gem throws, bright enough for the bloom pass to find. It is the
/// gem's own colour lifted past white, which is what makes a coloured beam read
/// as light rather than as a painted bar.
pub fn gem_light(color: GemColor) -> [f32; 3] {
    let body = gem_body(color);
    [body[0] * 2.6, body[1] * 2.6, body[2] * 2.6]
}

/// The classes, held as far apart as nine colours on one board can be held.
/// Nothing here is within reach of a crate or of the floors they stand on, and
/// the closest pair of them is further apart than any pair used to be, because
/// two members of a party are told apart by colour before anything else.
pub const PUSHER_BODY: [f32; 4] = [0.2, 0.5, 0.95, 1.0];
pub const DRAGGER_BODY: [f32; 4] = [0.36, 0.8, 0.38, 1.0];
pub const MAGNET_BODY: [f32; 4] = [0.9, 0.18, 0.2, 1.0];
pub const SWAPPER_BODY: [f32; 4] = [0.62, 0.36, 0.98, 1.0];
pub const WADER_BODY: [f32; 4] = [0.1, 0.86, 0.82, 1.0];
/// Every floor in the game is somewhere between mid and pale, so the one class
/// that has to be told apart from all of them is the dark one. A near white body
/// disappeared into the glacier and the quarry, and a shadow that walks through
/// walls is the right thing to look like anyway.
pub const PHASER_BODY: [f32; 4] = [0.2, 0.19, 0.26, 1.0];
/// Not the orange it was, which a crate wears and a plate wears with it.
pub const WARDEN_BODY: [f32; 4] = [0.96, 0.32, 0.72, 1.0];
pub const BLINKER_BODY: [f32; 4] = [0.98, 0.86, 0.24, 1.0];
/// Not the brown it was, which was a crate seen from across the room.
pub const BREAKER_BODY: [f32; 4] = [0.05, 0.55, 0.42, 1.0];

/// A boulder is rock wherever it is standing, and so is the steel over the
/// burner beside it, so neither moves with the skin.
pub const STONE_BODY: [f32; 4] = [0.46, 0.44, 0.42, 1.0];
pub const INCINERATOR_LEAF: [f32; 4] = [0.3, 0.27, 0.26, 1.0];
/// What is under the leaves, and how hard it burns. The surface is dark and
/// the light coming off it is not, so the seam reads as a fire seen through a
/// gap rather than as a square painted orange.
pub const INCINERATOR_GLOW: [f32; 4] = [0.5, 0.16, 0.05, 1.0];
pub const INCINERATOR_FLAME: [f32; 3] = [4.2, 1.1, 0.22];
pub const SPIKE_BODY: [f32; 4] = [0.72, 0.74, 0.78, 1.0];
pub const SOCKET_BODY: [f32; 4] = [0.74, 0.7, 0.62, 1.0];
/// A pane is nearly colourless, because what makes it read as glass is what is
/// behind it rather than what it is. The tint is what a thick enough sheet of
/// it would go, which is the green cast every real window has on its edge.
pub const GLASS_BODY: [f32; 4] = [0.88, 0.95, 0.97, 1.0];
pub const GLASS_TINT: [f32; 3] = [0.62, 0.86, 0.78];

/// A watcher is the same thing in every room it stands in, so it wears no
/// skin's colour: a dark post and one red eye, which is as close to a warning
/// sign as a board gets.
pub const WATCHER_BODY: [f32; 4] = [0.14, 0.12, 0.16, 1.0];
pub const WATCHER_EYE: [f32; 4] = [1.0, 0.16, 0.12, 1.0];
pub const WATCHER_GLARE: [f32; 3] = [4.0, 0.3, 0.2];
/// What it paints on the four squares it can reach, which is the whole of what
/// makes it readable rather than a surprise.
pub const WATCHER_REACH: [f32; 4] = [0.7, 0.1, 0.12, 1.0];

pub const PORTAL_COLORS: [[f32; 4]; 3] = [
    [0.55, 0.45, 0.95, 1.0],
    [0.3, 0.8, 0.85, 1.0],
    [0.95, 0.45, 0.75, 1.0],
];

pub const FLOOR_LIGHT: [f32; 4] = [0.72, 0.66, 0.58, 1.0];
pub const FLOOR_DARK: [f32; 4] = [0.62, 0.56, 0.49, 1.0];
pub const PLINTH: [f32; 4] = [0.24, 0.19, 0.2, 1.0];
pub const PLINTH_RIM: [f32; 4] = [0.16, 0.12, 0.14, 1.0];
pub const WALL: [f32; 4] = [0.4, 0.33, 0.36, 1.0];
pub const WALL_CAP: [f32; 4] = [0.55, 0.47, 0.48, 1.0];
pub const ICE: [f32; 4] = [0.68, 0.85, 0.95, 1.0];
pub const PIT: [f32; 4] = [0.05, 0.04, 0.06, 1.0];
pub const CRATE_BODY: [f32; 4] = [0.78, 0.5, 0.24, 1.0];
pub const CRATE_BODY_DONE: [f32; 4] = [0.45, 0.82, 0.44, 1.0];
pub const CRATE_BAND: [f32; 4] = [0.38, 0.24, 0.13, 1.0];
pub const CRATE_PLATE: [f32; 4] = [0.9, 0.68, 0.38, 1.0];
pub const GOAL: [f32; 4] = [0.3, 0.85, 0.78, 1.0];
pub const GOAL_DONE: [f32; 4] = [0.45, 1.0, 0.5, 1.0];
pub const PLATE: [f32; 4] = [0.95, 0.62, 0.2, 1.0];
pub const ONE_WAY: [f32; 4] = [0.96, 0.83, 0.38, 1.0];
pub const BELT: [f32; 4] = [0.44, 0.68, 0.56, 1.0];
pub const FRAGILE: [f32; 4] = [0.58, 0.52, 0.5, 1.0];
pub const SWITCH: [f32; 4] = [0.9, 0.48, 0.85, 1.0];
pub const BRITTLE: [f32; 4] = [0.58, 0.5, 0.45, 1.0];

/// A mirror and an orb are the same polished metal wherever they turn up, so
/// neither moves with the skin.
pub const MIRROR_FACE: [f32; 4] = [0.86, 0.9, 0.95, 1.0];
pub const ORB_BODY: [f32; 4] = [0.82, 0.85, 0.9, 1.0];
pub const LAMP_BODY: [f32; 4] = [1.0, 0.93, 0.72, 1.0];
pub const GATE: [f32; 4] = [0.85, 0.28, 0.3, 1.0];
pub const ELEVATOR: [f32; 4] = [0.4, 0.78, 0.9, 1.0];
pub const PLAYER_HEAD: [f32; 4] = [0.94, 0.78, 0.62, 1.0];
pub const PLAYER_TRIM: [f32; 4] = [0.95, 0.95, 0.98, 1.0];
