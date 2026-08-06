use crate::ecs::{Facing, MapOrigin, Screen, SokobanResources};
use crate::rules::{
    Death, Direction, MoveOutcome, attempt_handle, attempt_move, attempt_pull, attempt_ride,
    attempt_take, initial_state, killed_by,
};
use crate::schema::CrateKind;
use crate::systems::world::build::{CRATE_Y, PLAYER_Y, world_position};
use crate::systems::world::motion::{is_busy, start_motion};
use crate::systems::world::progress::{refresh_materials, restore_entities};
use nightshade::prelude::*;

const STEP_SECONDS: f32 = 0.135;
const SLIDE_SECONDS: f32 = 0.072;
const RIDE_SECONDS: f32 = 0.32;
const STEP_HOP: f32 = 0.085;
const FIRST_REPEAT: f32 = 0.26;
const REPEAT_INTERVAL: f32 = 0.12;
/// How long a death is left on screen before the move that caused it is taken
/// back. Long enough to see what happened and short enough not to be a
/// punishment.
const DYING_SECONDS: f32 = 0.9;

pub fn handle_global(game: Res<SokobanResources>, world: &mut World) {
    let toggle = world.res::<Input>().keyboard.just_pressed(KeyCode::Escape)
        || pad_pressed(world, gilrs::Button::Start);
    if !toggle {
        return;
    }
    if in_state(world, Screen::InGame) {
        if matches!(game.origin, MapOrigin::Authored) {
            next_state(world, Screen::Editor);
        } else {
            next_state(world, Screen::Paused);
        }
    } else if in_state(world, Screen::Paused) {
        next_state(world, Screen::InGame);
    } else if in_state(world, Screen::Story) && !cutscene_playing(world) {
        // The overworld is a board with no pause menu of its own, so the way
        // out of it is the way out of the story.
        next_state(world, Screen::Title);
    }
}

pub fn gameplay(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    // The board has just killed somebody and is showing it. Nothing to do with
    // the controls until it has finished.
    if game.dying > 0.0 {
        return;
    }
    // A scene holds the controls outright. Unlike a worked example it is not
    // something to be taken over, only something to be skipped.
    if cutscene_playing(world) {
        return;
    }
    // A worked example holds the controls until the player asks for them, and
    // asking is simply reaching for them. Nothing has to be stopped first.
    if game.playback.playing {
        if !wants_control(world) {
            return;
        }
        game.playback.playing = false;
    }
    let delta = world.res::<Time>().delta_time;

    if key_down(world, KeyCode::KeyR) || pad_pressed(world, gilrs::Button::West) {
        restart(game, world);
        return;
    }
    if key_down(world, KeyCode::KeyZ)
        || key_down(world, KeyCode::Backspace)
        || pad_pressed(world, gilrs::Button::East)
    {
        // Undo is not guarded itself, because a death has to undo the move that
        // caused it while that move is still playing out. Asking for one by
        // hand waits for the board to settle.
        if !is_busy(world) {
            undo(game, world);
        }
        return;
    }
    if key_down(world, KeyCode::KeyY) || pad_pressed(world, gilrs::Button::North) {
        if !is_busy(world) {
            redo(game, world);
        }
        return;
    }

    // Only a board with gems on it ever asks, so the key costs nothing
    // everywhere else and never has to be explained on a board without one.
    if !game.map.gems.is_empty()
        && !game.solved_announced
        && !is_busy(world)
        && (key_down(world, KeyCode::Space) || pad_pressed(world, gilrs::Button::South))
    {
        handle(game, world);
        return;
    }

    if !game.solved_announced && !is_busy(world) {
        // The bumpers take moves back, so a ride is on the sticks themselves,
        // which are the two buttons nothing else on a board wants.
        let up = key_down(world, KeyCode::KeyQ) || pad_pressed(world, gilrs::Button::RightThumb);
        let down = key_down(world, KeyCode::KeyE) || pad_pressed(world, gilrs::Button::LeftThumb);
        if up || down {
            ride(game, world, if up { 1 } else { -1 });
            return;
        }
    }

    if game.map.party_size() > 1 {
        let keyboard = &world.res::<Input>().keyboard;
        let picked = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
        ]
        .iter()
        .position(|key| keyboard.just_pressed(*key));
        // The bumpers walk the party, one each way, which is what a pair of
        // buttons either side of a pad is for.
        let party = game.map.party_size();
        let forward =
            keyboard.just_pressed(KeyCode::Tab) || pad_pressed(world, gilrs::Button::RightTrigger);
        let backward = pad_pressed(world, gilrs::Button::LeftTrigger);
        if let Some(index) = picked {
            take(game, world, index);
            return;
        }
        if forward {
            take(game, world, (game.state.active + 1) % party);
            return;
        }
        if backward {
            take(game, world, (game.state.active + party - 1) % party);
            return;
        }
    }

    let stick = stick_direction(world);
    let stick_edge = if stick == game.repeat.stick {
        None
    } else {
        stick
    };
    game.repeat.stick = stick;

    let pressed = pressed_direction(world)
        .or_else(|| pad_pressed_direction(world))
        .or(stick_edge);
    let held = held_direction(world)
        .or_else(|| pad_held_direction(world))
        .or(stick);

    let mut requested = pressed;
    match (pressed, held) {
        (Some(direction), _) => {
            game.repeat.held = Some(direction);
            game.repeat.timer = FIRST_REPEAT;
        }
        (None, Some(direction)) => {
            if game.repeat.held == Some(direction) {
                game.repeat.timer -= delta;
                if game.repeat.timer <= 0.0 {
                    game.repeat.timer = REPEAT_INTERVAL;
                    requested = Some(direction);
                }
            } else {
                game.repeat.held = Some(direction);
                game.repeat.timer = FIRST_REPEAT;
            }
        }
        (None, None) => game.repeat.held = None,
    }

    let Some(direction) = requested else {
        return;
    };
    if game.solved_announced || is_busy(world) {
        return;
    }
    if crate::rules::active_character(&game.map, &game.state).can_pull() && pull_held(world) {
        drag(game, world, direction);
        return;
    }
    step(game, world, direction);
}

/// Whether the player has reached for the controls. Only edge triggered input
/// counts, so a resting stick never takes the board away from an example that
/// nobody asked to interrupt.
fn wants_control(world: &World) -> bool {
    pressed_direction(world).is_some()
        || pad_pressed_direction(world).is_some()
        || stick_direction(world).is_some()
        || key_down(world, KeyCode::KeyR)
        || key_down(world, KeyCode::KeyZ)
        || key_down(world, KeyCode::Backspace)
        || key_down(world, KeyCode::KeyQ)
        || key_down(world, KeyCode::KeyE)
        || key_down(world, KeyCode::KeyY)
        || key_down(world, KeyCode::Space)
        || pad_pressed(world, gilrs::Button::West)
        || pad_pressed(world, gilrs::Button::East)
        || pad_pressed(world, gilrs::Button::North)
        || pad_pressed(world, gilrs::Button::South)
        || pad_pressed(world, gilrs::Button::LeftTrigger)
        || pad_pressed(world, gilrs::Button::RightTrigger)
}

/// Whether the player is asking to drag rather than shove. Only maps that allow
/// pulling ever ask, so the modifier costs nothing everywhere else.
fn pull_held(world: &World) -> bool {
    let keyboard = &world.res::<Input>().keyboard;
    keyboard.is_key_pressed(KeyCode::ShiftLeft)
        || keyboard.is_key_pressed(KeyCode::ShiftRight)
        || with_active_gamepad(world, |pad| pad.is_pressed(gilrs::Button::South)).unwrap_or(false)
}

/// One drag. Falls back to a shove when there is nothing behind to drag, so
/// holding the modifier never leaves the player unable to move.
pub fn drag(game: &mut SokobanResources, world: &mut World, direction: Direction) {
    let player = game.active_body();
    if let Some(facing) = world.get_mut::<Facing>(player) {
        facing.target = direction.yaw();
    }

    let Some(outcome) = attempt_pull(&game.map, &game.state, direction) else {
        step(game, world, direction);
        return;
    };
    commit(game, world, outcome, STEP_SECONDS, SLIDE_SECONDS, STEP_HOP);
}

/// One step in a direction. The keyboard, the pad, and a worked example all
/// come through here, so a demonstration obeys exactly the rules a player does.
pub fn step(game: &mut SokobanResources, world: &mut World, direction: Direction) {
    let player = game.active_body();
    if let Some(facing) = world.get_mut::<Facing>(player) {
        facing.target = direction.yaw();
    }

    let Some(outcome) = attempt_move(&game.map, &game.state, direction) else {
        game.state.facing = direction;
        bump(game, world, direction);
        return;
    };

    commit(game, world, outcome, STEP_SECONDS, SLIDE_SECONDS, STEP_HOP);
}

/// Lifting, seating or putting down a gem. What it does is decided by the
/// square underfoot and what is already in hand, so the one key covers all
/// three and never has to ask which was meant.
pub fn handle(game: &mut SokobanResources, world: &mut World) {
    let Some(outcome) = attempt_handle(&game.map, &game.state) else {
        return;
    };
    commit(game, world, outcome, STEP_SECONDS, SLIDE_SECONDS, 0.0);
}

/// Points the controls at another member of the party.
pub fn take(game: &mut SokobanResources, world: &mut World, index: usize) {
    let Some(outcome) = attempt_take(&game.map, &game.state, index) else {
        return;
    };
    commit(game, world, outcome, STEP_SECONDS, SLIDE_SECONDS, 0.0);
}

pub fn ride(game: &mut SokobanResources, world: &mut World, direction: i32) {
    let Some(outcome) = attempt_ride(&game.map, &game.state, direction) else {
        return;
    };
    commit(game, world, outcome, RIDE_SECONDS, RIDE_SECONDS, 0.0);
}

/// Whether the move just made has killed anybody, and starting the dying if so.
/// The search never walks into a beam or into water, so this is only ever the
/// player doing it, and what answers it is the move coming back once the body
/// has finished going down.
fn check_burned(game: &mut SokobanResources, world: &mut World) {
    if game.dying > 0.0 {
        return;
    }
    let Some(death) = killed_by(&game.map, &game.state) else {
        return;
    };
    game.notice = death.notice().to_string();
    game.dying = DYING_SECONDS;

    let body = game.active_body();
    let resting = world_position(game.state.player(), PLAYER_Y);
    match death {
        // Down like a crate into a pit, since that is what the board has just
        // done with them.
        Death::Drowned | Death::Burned => {
            start_motion(world, body, vec![resting], 0.12, 0.0, true);
        }
        // Spikes do not swallow anybody. They come up through the floor, so
        // the body jolts on them and drops where it stands.
        Death::Impaled | Death::Watched => {
            start_motion(
                world,
                body,
                vec![resting + Vec3::new(0.0, 0.22, 0.0), resting],
                0.09,
                0.0,
                false,
            );
        }
    }
    crate::systems::world::progress::fade_goals(game, world, true);

    // A burst where they went, which is the difference between a body sinking
    // and a body simply being lower down than it was. The board keeps one and
    // fires it again rather than leaving a spent emitter behind every death.
    let splash = game.entities.splash;
    let at = world_position(game.state.player(), 0.25);
    if world.is_alive(splash) {
        if let Some(transform) = world.get_mut::<LocalTransform>(splash) {
            transform.translation = at;
        }
        if let Some(emitter) = world.get_mut::<ParticleEmitter>(splash) {
            emitter.color_gradient = match death {
                Death::Drowned => ColorGradient::smoke(),
                Death::Burned | Death::Impaled | Death::Watched => ColorGradient::sparks(),
            };
            emitter.emissive_strength = match death {
                Death::Drowned => 0.6,
                Death::Burned => 2.4,
                Death::Impaled | Death::Watched => 1.6,
            };
            emitter.burst_count = match death {
                Death::Drowned => 40,
                Death::Burned => 26,
                Death::Impaled | Death::Watched => 32,
            };
            emitter.has_fired = false;
            emitter.enabled = true;
        }
    }
}

/// The one burst a board keeps, fired wherever something has just happened. A
/// board has at most one of these going at a time, so moving it and telling it
/// to go again is cheaper and steadier than building another.
fn burst(
    game: &SokobanResources,
    world: &mut World,
    at: Vec3,
    gradient: ColorGradient,
    count: u32,
) {
    let splash = game.entities.splash;
    if !world.is_alive(splash) {
        return;
    }
    if let Some(transform) = world.get_mut::<LocalTransform>(splash) {
        transform.translation = at;
    }
    if let Some(emitter) = world.get_mut::<ParticleEmitter>(splash) {
        emitter.color_gradient = gradient;
        emitter.emissive_strength = 0.8;
        emitter.burst_count = count;
        emitter.has_fired = false;
        emitter.enabled = true;
    }
}

/// Runs the dying, and puts the board back when it is done.
pub fn settle_death(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if game.dying <= 0.0 {
        return;
    }
    game.dying -= world.res::<Time>().delta_time;
    if game.dying > 0.0 {
        return;
    }
    game.dying = 0.0;
    crate::systems::world::progress::fade_goals(game, world, false);
    undo(game, world);
}

/// Commits a move: the old state goes on the undo stack, the actors are handed
/// their paths, and the materials catch up with what is on a goal now.
fn commit(
    game: &mut SokobanResources,
    world: &mut World,
    outcome: MoveOutcome,
    step_seconds: f32,
    slide_seconds: f32,
    hop: f32,
) {
    game.undo_stack.push(game.state.clone());
    // A board that has gone somewhere new has nothing left to put back.
    game.redo_stack.clear();

    let player_path: Vec<Vec3> = outcome
        .player_path
        .iter()
        .map(|at| world_position(*at, PLAYER_Y))
        .collect();
    let sliding = player_path.len() > 1;
    let mover = game
        .entities
        .members
        .get(outcome.mover)
        .copied()
        .unwrap_or_default();
    start_motion(
        world,
        mover,
        player_path,
        if sliding { slide_seconds } else { step_seconds },
        if sliding { 0.0 } else { hop },
        false,
    );

    for (index, path) in &outcome.crate_moves {
        let Some(entity) = game.entities.crates.get(*index).copied() else {
            continue;
        };
        let world_path: Vec<Vec3> = path.iter().map(|at| world_position(*at, CRATE_Y)).collect();
        let long = world_path.len() > 1;
        // A boulder that has been broken does not go down a hole. It comes
        // apart where it stood, which the props pass shows by taking it to
        // pieces rather than by lowering it out of sight.
        let broken =
            outcome.sunk.contains(index) && game.state.crates[*index].kind == CrateKind::Stone;
        if broken && let Some(at) = path.last() {
            burst(
                game,
                world,
                world_position(*at, CRATE_Y),
                ColorGradient::smoke(),
                30,
            );
        }
        start_motion(
            world,
            entity,
            world_path,
            if long { slide_seconds } else { step_seconds },
            0.0,
            outcome.sunk.contains(index) && !broken,
        );
    }

    // A gem that changed hands, or came out of a socket, or was put down, is
    // the only reason the crystals have to be moved. One being carried follows
    // whoever is carrying it without anybody having to say so.
    let carried_before = game.state.gems.clone();
    let watched_before = game.state.watchers.clone();
    game.state = outcome.state;
    if game.state.gems != carried_before {
        crate::systems::world::gems::restore(game, world, step_seconds);
    }
    // A trade is the one move that moves a watcher, so the post follows.
    if game.state.watchers != watched_before {
        crate::systems::world::watchers::restore(game, world, step_seconds);
    }
    refresh_materials(game, world);
    check_burned(game, world);
}

fn bump(game: &SokobanResources, world: &mut World, direction: Direction) {
    let delta = direction.delta();
    let home = world_position(game.state.player(), PLAYER_Y);
    let nudge = home + Vec3::new(delta.0 as f32 * 0.16, 0.0, delta.1 as f32 * 0.16);
    start_motion(
        world,
        game.active_body(),
        vec![nudge, home],
        0.07,
        0.0,
        false,
    );
}

fn undo(game: &mut SokobanResources, world: &mut World) {
    let Some(previous) = game.undo_stack.pop() else {
        return;
    };
    game.redo_stack
        .push(std::mem::replace(&mut game.state, previous));
    game.solved_announced = false;
    game.solved_delay = 0.0;
    crate::systems::world::effects::begin_rewind(game);
    restore_entities(game, world, 0.1);
}

/// Putting back a move that was taken back. The two stacks are each other's
/// mirror, so this is the undo with the ends swapped and nothing else.
fn redo(game: &mut SokobanResources, world: &mut World) {
    let Some(next) = game.redo_stack.pop() else {
        return;
    };
    game.undo_stack
        .push(std::mem::replace(&mut game.state, next));
    game.solved_announced = false;
    game.solved_delay = 0.0;
    restore_entities(game, world, 0.1);
}

pub fn restart(game: &mut SokobanResources, world: &mut World) {
    game.state = initial_state(&game.map);
    game.undo_stack.clear();
    game.redo_stack.clear();
    game.solved_announced = false;
    game.solved_delay = 0.0;
    restore_entities(game, world, 0.12);
}

fn key_down(world: &World, key: KeyCode) -> bool {
    world.res::<Input>().keyboard.just_pressed(key)
}

fn pressed_direction(world: &World) -> Option<Direction> {
    let keyboard = &world.res::<Input>().keyboard;
    DIRECTION_KEYS
        .iter()
        .find(|(_, keys)| keys.iter().any(|key| keyboard.just_pressed(*key)))
        .map(|(direction, _)| *direction)
}

fn held_direction(world: &World) -> Option<Direction> {
    let keyboard = &world.res::<Input>().keyboard;
    DIRECTION_KEYS
        .iter()
        .find(|(_, keys)| keys.iter().any(|key| keyboard.is_key_pressed(*key)))
        .map(|(direction, _)| *direction)
}

const DIRECTION_KEYS: [(Direction, [KeyCode; 2]); 4] = [
    (Direction::Up, [KeyCode::KeyW, KeyCode::ArrowUp]),
    (Direction::Down, [KeyCode::KeyS, KeyCode::ArrowDown]),
    (Direction::Left, [KeyCode::KeyA, KeyCode::ArrowLeft]),
    (Direction::Right, [KeyCode::KeyD, KeyCode::ArrowRight]),
];

const DIRECTION_BUTTONS: [(Direction, gilrs::Button); 4] = [
    (Direction::Up, gilrs::Button::DPadUp),
    (Direction::Down, gilrs::Button::DPadDown),
    (Direction::Left, gilrs::Button::DPadLeft),
    (Direction::Right, gilrs::Button::DPadRight),
];

pub fn pad_pressed(world: &World, button: gilrs::Button) -> bool {
    world.res::<Gamepad>().just_pressed(button)
}

fn pad_pressed_direction(world: &World) -> Option<Direction> {
    DIRECTION_BUTTONS
        .iter()
        .find(|(_, button)| pad_pressed(world, *button))
        .map(|(direction, _)| *direction)
}

fn pad_held_direction(world: &World) -> Option<Direction> {
    with_active_gamepad(world, |pad| {
        DIRECTION_BUTTONS
            .iter()
            .find(|(_, button)| pad.is_pressed(*button))
            .map(|(direction, _)| *direction)
    })
    .flatten()
}

fn stick_direction(world: &World) -> Option<Direction> {
    const DEADZONE: f32 = 0.55;
    /// How far the stick has to favour one axis before that axis is the answer.
    /// A board has four directions and a stick has all of them, so a stick held
    /// at the corner is not a direction: it is two, and picking whichever is
    /// momentarily larger makes it both, one per frame, which is a body running
    /// across the room on its own.
    const DOMINANCE: f32 = 1.4;
    with_active_gamepad(world, |pad| {
        let horizontal = pad.value(gilrs::Axis::LeftStickX);
        let vertical = pad.value(gilrs::Axis::LeftStickY);
        if horizontal.abs() < DEADZONE && vertical.abs() < DEADZONE {
            return None;
        }
        if horizontal.abs() > vertical.abs() * DOMINANCE {
            Some(if horizontal > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            })
        } else if vertical.abs() > horizontal.abs() * DOMINANCE {
            Some(if vertical > 0.0 {
                Direction::Up
            } else {
                Direction::Down
            })
        } else {
            None
        }
    })
    .flatten()
}
