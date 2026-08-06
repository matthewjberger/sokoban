use crate::ecs::{FACING, Facing, PART, Part, TILE_MOTION, TileMotion};
use crate::systems::world::build::{CRATE_SUNK_Y, CRATE_Y};
use nightshade::prelude::*;
use std::f32::consts::{PI, TAU};

const SINK_RATE: f32 = 3.0;
const RISE_RATE: f32 = 3.6;
const TURN_RATE: f32 = 16.0;

pub fn start_motion(
    world: &mut World,
    entity: Entity,
    path: Vec<Vec3>,
    seconds_per_step: f32,
    hop_height: f32,
    sinking: bool,
) {
    if path.is_empty() {
        return;
    }
    let translation = world
        .get::<LocalTransform>(entity)
        .map(|transform| transform.translation)
        .unwrap_or_default();
    let sink_progress = world
        .get::<TileMotion>(entity)
        .map(|motion| motion.sink_progress)
        .unwrap_or(0.0);
    let mut start = translation;
    start.y += sink_progress * (CRATE_Y - CRATE_SUNK_Y);

    if let Some(motion) = world.get_mut::<TileMotion>(entity) {
        motion.start = start;
        motion.path = path;
        motion.segment = 0;
        motion.progress = 0.0;
        motion.seconds_per_step = seconds_per_step.max(0.01);
        motion.hop_height = hop_height;
        motion.sinking = sinking;
        motion.active = true;
    }
}

/// Whether one actor is still travelling. The visibility pass waits on this
/// so a storey change lands when the ride ends rather than when the move is
/// decided.
pub fn is_moving(world: &World, entity: Entity) -> bool {
    world
        .get::<TileMotion>(entity)
        .is_some_and(|motion| motion.active)
}

pub fn is_busy(world: &World) -> bool {
    world
        .query_ref::<&TileMotion>()
        .iter()
        .any(|(_, motion)| motion.active)
}

pub fn advance(world: &mut World) {
    let delta = world.res::<Time>().delta_time;
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(TILE_MOTION).collect();
    for entity in entities {
        let Some(position) = step_motion(world, entity, delta) else {
            continue;
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = position;
        }
    }
}

fn step_motion(world: &mut World, entity: Entity, delta: f32) -> Option<Vec3> {
    let motion = world.get_mut::<TileMotion>(entity)?;
    if !motion.active {
        return None;
    }
    if motion.path.is_empty() {
        motion.active = false;
        return None;
    }

    let mut finished_path = motion.segment >= motion.path.len();
    if !finished_path {
        motion.progress += delta / motion.seconds_per_step;
        while motion.progress >= 1.0 && motion.segment + 1 < motion.path.len() {
            motion.progress -= 1.0;
            motion.segment += 1;
        }
        if motion.progress >= 1.0 {
            motion.segment = motion.path.len();
            motion.progress = 0.0;
            finished_path = true;
        }
    }

    let mut position = if finished_path {
        motion.path[motion.path.len() - 1]
    } else {
        let from = if motion.segment == 0 {
            motion.start
        } else {
            motion.path[motion.segment - 1]
        };
        let to = motion.path[motion.segment];
        let progress = motion.progress;
        let eased = if motion.path.len() == 1 {
            progress * progress * (3.0 - 2.0 * progress)
        } else if motion.segment + 1 == motion.path.len() {
            1.0 - (1.0 - progress) * (1.0 - progress)
        } else {
            progress
        };
        let mut point = from + (to - from) * eased;
        point.y += (progress * PI).sin() * motion.hop_height;
        point
    };

    if motion.sinking {
        if finished_path {
            motion.sink_progress = (motion.sink_progress + delta * SINK_RATE).min(1.0);
        }
    } else {
        motion.sink_progress = (motion.sink_progress - delta * RISE_RATE).max(0.0);
    }
    position.y -= motion.sink_progress * (CRATE_Y - CRATE_SUNK_Y);

    let sink_settled = if motion.sinking {
        motion.sink_progress >= 1.0
    } else {
        motion.sink_progress <= 0.0
    };
    if finished_path && sink_settled {
        motion.active = false;
    }

    Some(position)
}

pub fn update_facing(world: &mut World) {
    let delta = world.res::<Time>().delta_time;
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(FACING).collect();
    for entity in entities {
        let yaw = {
            let Some(facing) = world.get_mut::<Facing>(entity) else {
                continue;
            };
            let mut difference = facing.target - facing.current;
            while difference > PI {
                difference -= TAU;
            }
            while difference < -PI {
                difference += TAU;
            }
            facing.current += difference * (1.0 - (-delta * TURN_RATE).exp());
            facing.current
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.rotation = nalgebra_glm::quat_angle_axis(yaw, &Vec3::new(0.0, 1.0, 0.0));
        }
    }
}

pub fn sync_parts(world: &mut World) {
    let entities: Vec<Entity> = world.ecs.worlds[GAME].query_entities(PART).collect();
    for entity in entities {
        let Some(part) = world.get::<Part>(entity).copied() else {
            continue;
        };
        let Some(owner) = world
            .get::<LocalTransform>(part.owner)
            .map(|transform| transform.translation)
        else {
            continue;
        };
        let yaw = world
            .get::<Facing>(part.owner)
            .map(|facing| facing.current)
            .unwrap_or(0.0);
        let spin = nalgebra_glm::quat_angle_axis(yaw, &Vec3::new(0.0, 1.0, 0.0));
        let tilt = nalgebra_glm::quat_angle_axis(part.pitch, &Vec3::new(1.0, 0.0, 0.0));
        let translation = owner + nalgebra_glm::quat_rotate_vec3(&spin, &part.offset);
        let rotation = if part.follows_rotation {
            spin * tilt
        } else {
            tilt
        };
        if let Some(transform) = world.get_mut::<LocalTransform>(entity) {
            transform.translation = translation;
            transform.rotation = rotation;
        }
    }
}
