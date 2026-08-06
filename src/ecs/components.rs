use crate::schema::Position;
use nalgebra_glm::Vec3;
use nightshade::prelude::Entity;

/// Which storey an entity belongs to. The visibility pass shows the storey in
/// play and hides the rest, which is what keeps an overhead camera readable on
/// a stacked map.
#[derive(Default, Clone, Copy, Debug)]
pub struct LayerTag {
    pub layer: i32,
}

#[derive(Default, Clone, Debug)]
pub struct TileMotion {
    pub start: Vec3,
    pub path: Vec<Vec3>,
    pub segment: usize,
    pub progress: f32,
    pub seconds_per_step: f32,
    pub hop_height: f32,
    pub active: bool,
    pub sinking: bool,
    pub sink_progress: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Facing {
    pub current: f32,
    pub target: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Part {
    pub owner: Entity,
    pub offset: Vec3,
    pub pitch: f32,
    pub follows_rotation: bool,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Spinner {
    pub base: Vec3,
    pub spin_speed: f32,
    pub bob_height: f32,
    pub bob_speed: f32,
    pub phase: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct GoalMarker {
    pub index: usize,
    pub base: Vec3,
    pub glow: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct GateVisual {
    pub group: usize,
    pub at: Position,
    pub base: Vec3,
    pub openness: f32,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct PlateVisual {
    pub at: Position,
    pub base: Vec3,
    pub pressed: f32,
}

/// The slab over a fragile square. It drops out of sight once the square has
/// gone, leaving the hole that was built underneath it all along.
#[derive(Default, Clone, Copy, Debug)]
pub struct FragileVisual {
    pub at: Position,
    pub base: Vec3,
    pub fall: f32,
}

/// A brittle wall. It stands until a crate is spent on it, then sinks out of
/// the way to leave the square open.
#[derive(Default, Clone, Copy, Debug)]
pub struct BrittleVisual {
    pub at: Position,
    pub base: Vec3,
    pub fall: f32,
}

/// A sensor that opens while a lamp is reaching it.
#[derive(Default, Clone, Copy, Debug)]
pub struct SensorVisual {
    pub at: Position,
    pub base: Vec3,
    pub lit: f32,
}

/// A tower that lights while a beam is reaching it.
#[derive(Default, Clone, Copy, Debug)]
pub struct TowerVisual {
    pub group: usize,
    pub base: Vec3,
    pub lit: f32,
    /// The lightning that crackles off it while it is drinking a beam.
    pub arc: Entity,
    /// The sparks it throws while it is drinking one.
    pub sparks: Entity,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct SwitchVisual {
    pub group: usize,
    pub base: Vec3,
    pub thrown: f32,
}

/// A bed of spikes. They stand up while their group is powered, which is the
/// same wiring a gate answers to shown a different way.
#[derive(Default, Clone, Copy, Debug)]
pub struct SpikeVisual {
    pub group: usize,
    pub base: Vec3,
    pub raised: f32,
}

/// A plinth that holds a gem. It lights when it has one, so a board can be read
/// for which of its sockets are still empty without counting the gems.
#[derive(Default, Clone, Copy, Debug)]
pub struct SocketVisual {
    pub at: Position,
    pub base: Vec3,
    pub lit: f32,
}

/// One gem, which is the one thing on the board that is sometimes on a square,
/// sometimes in a socket and sometimes in somebody's hands.
#[derive(Default, Clone, Copy, Debug)]
pub struct GemVisual {
    pub index: usize,
    /// Its own place in the turn, so a room full of them never spins as one.
    pub phase: f32,
}
