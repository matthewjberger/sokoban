mod components;
mod handles;
mod resources;

pub use components::*;
pub use handles::*;
pub use resources::*;

use nightshade::prelude::nightshade_ecs;

nightshade_ecs::dynamic_schema! {
    pub fn register_sokoban_components {
        tile_motion: TileMotion => TILE_MOTION,
        facing: Facing => FACING,
        part: Part => PART,
        spinner: Spinner => SPINNER,
        goal_marker: GoalMarker => GOAL_MARKER,
        gate_visual: GateVisual => GATE_VISUAL,
        plate_visual: PlateVisual => PLATE_VISUAL,
        layer_tag: LayerTag => LAYER_TAG,
        fragile_visual: FragileVisual => FRAGILE_VISUAL,
        switch_visual: SwitchVisual => SWITCH_VISUAL,
        brittle_visual: BrittleVisual => BRITTLE_VISUAL,
        tower_visual: TowerVisual => TOWER_VISUAL,
        sensor_visual: SensorVisual => SENSOR_VISUAL,
        spike_visual: SpikeVisual => SPIKE_VISUAL,
        socket_visual: SocketVisual => SOCKET_VISUAL,
        gem_visual: GemVisual => GEM_VISUAL,
    }
}
