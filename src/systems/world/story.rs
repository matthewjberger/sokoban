//! Story mode. The overworld is loaded and walked exactly like a puzzle, and
//! this is only the part that notices the player is standing on a door, decides
//! whether it is open, and takes them through it and back again.

use crate::ecs::{MapOrigin, MapRequest, Screen, SokobanResources};
use crate::maps::{load_map, map_count};
use crate::schema::{Tile, map_tile};
use crate::story::{
    FLOOR_HEIGHT, FLOOR_WIDTH, area_at, area_of, area_unlocked, areas, level_unlocked,
};
use crate::systems::world::{build, motion::is_busy};
use nightshade::prelude::*;

/// Opens the overworld, putting the player back where they left it.
pub fn enter_overworld(game: &mut SokobanResources, world: &mut World) {
    let map = crate::story::overworld();
    let resume = game.story.depot.clone();
    game.story.cleared.resize(map_count(), false);
    build::start_map(
        game,
        world,
        MapRequest {
            map,
            origin: MapOrigin::Overworld,
        },
    );
    // Building a map starts it from the beginning, which is right for a room
    // and wrong for the depot. The depot is a board in progress, and the gates
    // standing open are being held open by crates the player put there.
    if let Some(state) = resume {
        game.state = state;
        crate::systems::world::progress::restore_entities(game, world, 0.0);
    }
    game.story.area = area_at(game.state.player());
    game.story.at_door = None;
    spawn_area_signs(game, world);
    announce_area(game);
}

/// Plays whatever scene the story owes the player, and holds the controls while
/// it runs. Scenes are camera and words over the same board the player walks,
/// so there is nothing to set up and nothing to tear down.
pub fn scenes(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Story) {
        return;
    }
    if cutscene_playing(world) {
        // Any key gets out of a scene that has been seen before.
        if world.res::<Input>().keyboard.just_pressed(KeyCode::Escape) {
            stop_cutscene(world);
        }
        return;
    }

    if !game.story.opening_seen {
        game.story.opening_seen = true;
        set_cutscene_camera(world, game.camera.entity);
        play_cutscene(world, crate::cutscenes::opening());
        return;
    }
    if let Some(area) = game.story.pending_scene.take() {
        set_cutscene_camera(world, game.camera.entity);
        play_cutscene(world, crate::cutscenes::area_opens(area));
        return;
    }
    if game.story.cleared.iter().filter(|done| **done).count() == map_count()
        && !game.story.ending_seen
    {
        game.story.ending_seen = true;
        set_cutscene_camera(world, game.camera.entity);
        play_cutscene(world, crate::cutscenes::ending());
    }
}

/// The door under the player, if there is one.
fn door_under(game: &SokobanResources) -> Option<usize> {
    match map_tile(&game.map, game.state.player()) {
        Tile::Gateway(level) => Some(level as usize),
        _ => None,
    }
}

/// Watches where the player is standing and says what is there. A door names
/// the puzzle behind it and whether it will open, and crossing into a new area
/// names the area.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Story) || is_busy(world) || cutscene_playing(world) {
        return;
    }

    // The depot is remembered a move at a time, so every way out of it leaves
    // it as it stood rather than only the way through a door.
    if game.story.depot.as_ref().map(|state| state.moves) != Some(game.state.moves) {
        game.story.depot = Some(game.state.clone());
    }

    let area = area_at(game.state.player());
    if area != game.story.area {
        game.story.area = area;
        announce_area(game);
    }

    let Some(level) = door_under(game) else {
        if game.story.at_door.is_some() {
            game.story.at_door = None;
            announce_area(game);
        }
        return;
    };

    if game.story.at_door != Some(level) {
        game.story.at_door = Some(level);
        game.notice = describe_door(game, level);
    }

    let asked = world.res::<Input>().keyboard.just_pressed(KeyCode::Enter)
        || world.res::<Input>().keyboard.just_pressed(KeyCode::Space)
        || crate::systems::input::pad_pressed(world, gilrs::Button::South);
    if !asked {
        return;
    }
    if !level_unlocked(level, &game.story.cleared) {
        game.notice = format!(
            "{} is shut. Finish {} first.",
            load_map(level).name,
            areas()[area_of(level).saturating_sub(1)].name
        );
        return;
    }

    game.pending = Some(MapRequest {
        map: load_map(level),
        origin: MapOrigin::Story(level),
    });
    next_state(world, Screen::InGame);
}

fn describe_door(game: &SokobanResources, level: usize) -> String {
    let name = load_map(level).name;
    if level_unlocked(level, &game.story.cleared) {
        let done = game.story.cleared.get(level).copied().unwrap_or(false);
        let mark = if done { "cleared" } else { "open" };
        format!("{name}  ·  {mark}  ·  ENTER to go in")
    } else {
        format!("{name}  ·  shut")
    }
}

/// Each area names itself where it stands, so the depot reads as four rooms
/// with names rather than one floor with a caption somewhere off the edge.
fn spawn_area_signs(game: &mut SokobanResources, world: &mut World) {
    for (index, area) in areas().iter().enumerate() {
        let (column, row) = area.slot;
        let open = area_unlocked(index, &game.story.cleared);
        let anchor = Vec3::new(
            (column * FLOOR_WIDTH) as f32 + (FLOOR_WIDTH - 1) as f32 * 0.5,
            1.4,
            (row * FLOOR_HEIGHT) as f32 - 0.4,
        );
        let entity = spawn_3d_billboard_text_with_properties(
            world,
            &area.name,
            anchor,
            TextProperties {
                font_size: 30.0,
                color: if open {
                    Vec4::new(1.0, 0.86, 0.62, 1.0)
                } else {
                    Vec4::new(0.5, 0.44, 0.42, 1.0)
                },
                alignment: TextAlignment::Center,
                vertical_alignment: VerticalAlignment::Middle,
                outline_width: 0.8,
                outline_color: Vec4::new(0.0, 0.0, 0.0, 0.95),
                ..Default::default()
            },
        );
        crate::systems::world::build::track_entity(game, world, entity, 0);
    }
}

/// Frames the area the player is in rather than the whole storey. The camera
/// follows its own settling, so crossing a boundary pans across rather than
/// cutting, and each area reads as a room instead of a quarter of a floor.
fn frame_area(game: &mut SokobanResources) {
    let (column, row) = areas()[game.story.area].slot;
    game.camera.focus = Vec3::new(
        (column * FLOOR_WIDTH) as f32 + (FLOOR_WIDTH - 1) as f32 * 0.5,
        0.0,
        (row * FLOOR_HEIGHT) as f32 + (FLOOR_HEIGHT - 1) as f32 * 0.5,
    );
    game.camera.extent = Vec2::new(FLOOR_WIDTH as f32, FLOOR_HEIGHT as f32);
}

fn announce_area(game: &mut SokobanResources) {
    frame_area(game);
    let area = &areas()[game.story.area];
    let cleared = (0..map_count())
        .filter(|level| area_of(*level) == game.story.area)
        .filter(|level| game.story.cleared.get(*level).copied().unwrap_or(false))
        .count();
    let total = (0..map_count())
        .filter(|level| area_of(*level) == game.story.area)
        .count();
    game.notice = format!("{}  ·  {}  ·  {cleared} of {total}", area.name, area.blurb);
}

/// Records a puzzle finished from the overworld and goes back to the door it
/// was entered by. Anything the clearing opened is said on arrival.
pub fn finish_puzzle(game: &mut SokobanResources, world: &mut World, level: usize) {
    if game.story.cleared.len() < map_count() {
        game.story.cleared.resize(map_count(), false);
    }
    let opened_before = area_unlocked(area_of(level) + 1, &game.story.cleared);
    if let Some(flag) = game.story.cleared.get_mut(level) {
        *flag = true;
    }
    next_state(world, Screen::Story);

    let next_area = area_of(level) + 1;
    if next_area < areas().len() && !opened_before && area_unlocked(next_area, &game.story.cleared)
    {
        game.story.pending_scene = Some(next_area);
    }
}
