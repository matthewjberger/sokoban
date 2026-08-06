use crate::ecs::SokobanResources;
use nightshade::prelude::*;

const FIELD_OF_VIEW: f32 = 45.0;
const FRAME_MARGIN: f32 = 1.08;
const FOLLOW_RATE: f32 = 4.5;
const SETTLE_EPSILON: f32 = 0.001;

pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if game.camera.extent.x <= 0.0 {
        return;
    }
    // A cutscene is driving this camera. Two things writing one transform each
    // frame means the one that runs last wins, and it would be this. The frame
    // it ends, this takes the camera back from wherever the scene left it, and
    // it eases back rather than cutting, because clearing the settled flag here
    // would make that first frame a jump instead of a move.
    if cutscene_playing(world) {
        return;
    }

    let aspect = window_viewport_size(world)
        .map(|(width, height)| width as f32 / height.max(1) as f32)
        .unwrap_or(16.0 / 9.0)
        .max(0.6);

    let tangent = (FIELD_OF_VIEW * 0.5).to_radians().tan();
    let pitch = game.camera.pitch;
    let horizontal = (game.camera.extent.x * 0.5 + 0.7) / (tangent * aspect);
    let vertical = (game.camera.extent.y * 0.5 * pitch.cos() + 1.6) / tangent;
    game.camera.distance = horizontal.max(vertical) * FRAME_MARGIN;

    let target = game.camera.focus
        + Vec3::new(
            game.camera.shift.x,
            game.camera.distance * pitch.sin(),
            game.camera.distance * pitch.cos() + game.camera.shift.y,
        );
    let rotation = nalgebra_glm::quat_angle_axis(-pitch, &Vec3::new(1.0, 0.0, 0.0));

    let delta = world.res::<Time>().delta_time;
    let blend = if game.camera.settled {
        1.0 - (-delta * FOLLOW_RATE).exp()
    } else {
        1.0
    };
    game.camera.settled = true;

    let camera = game.camera.entity;
    let Some(transform) = world.get_mut::<LocalTransform>(camera) else {
        return;
    };
    let offset = target - transform.translation;
    // An asymptotic follow never quite arrives, and a camera that moves by a
    // fraction of a pixel every frame reads as a permanent shimmer once
    // temporal antialiasing smears it. Snap the last sliver and then leave the
    // transform alone entirely so the frame is genuinely still.
    if offset.magnitude() <= SETTLE_EPSILON {
        if offset != Vec3::zeros() {
            transform.translation = target;
            transform.rotation = rotation;
        }
        return;
    }
    transform.translation += offset * blend;
    transform.rotation = rotation;
}
