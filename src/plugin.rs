use crate::ecs::{Screen, SokobanResources};
use nightshade::prelude::*;

pub struct SokobanPlugin;

/// Every screen the game has. Each one gets the same entry hook, which is where
/// a screen sets up whatever it needs to be looked at.
const SCREENS: [Screen; 11] = [
    Screen::Title,
    Screen::LevelSelect,
    Screen::Story,
    Screen::Settings,
    Screen::RandomSetup,
    Screen::Gallery,
    Screen::InGame,
    Screen::Paused,
    Screen::MapComplete,
    Screen::CampaignComplete,
    Screen::Editor,
];

impl Plugin for SokobanPlugin {
    fn build(&self, app: &mut App) {
        let window = app.world.res_mut::<Window>();
        window.title = "Sokoban".to_string();
        // The panels scale themselves to whatever the window can show, and this
        // is the size below which that stops being worth doing.
        window.min_window_size = Some((640, 420));
        app.insert_resource(SokobanResources::default());
        app.insert_state(Screen::Title);
        for screen in SCREENS {
            app.on_enter(
                screen,
                move |mut game: ResMut<SokobanResources>, world: &mut World| {
                    crate::systems::lifecycle::enter(&mut game, world, screen);
                },
            );
        }

        app.add_system(Stage::Startup, crate::systems::lifecycle::initialize);

        // Intent first, meaning what the player asked for and what the rules
        // make of it. Nothing here draws anything.
        app.add_systems(
            Stage::Update,
            (
                crate::systems::input::handle_global,
                crate::systems::world::work::update,
                crate::systems::world::effects::scale_time,
                crate::systems::screens::title::handle_input,
                crate::systems::screens::level_select::handle_input,
                crate::systems::screens::pause::handle_input,
                crate::systems::screens::complete::handle_input,
                crate::systems::screens::finale::handle_input,
                crate::systems::screens::random_setup::handle_input,
                crate::systems::screens::settings::handle_input,
                while_in(
                    Screen::Story,
                    (
                        crate::systems::world::story::scenes,
                        crate::systems::world::story::update,
                    ),
                ),
                crate::systems::screens::gallery::handle_input,
                crate::systems::screens::hud::handle_input,
                // The player goes first. Reaching for the controls during a
                // worked example stops it, and playback that has just been
                // stopped must not get one more move in on the way out.
                while_in_any(
                    [Screen::InGame, Screen::Gallery, Screen::Story],
                    (
                        crate::systems::input::gameplay,
                        crate::systems::input::settle_death,
                        crate::systems::world::progress::check_solved,
                    ),
                ),
                // A worked example and a solution playing itself out are the
                // same thing, so both run wherever a board is in play.
                while_in_any(
                    [Screen::InGame, Screen::Gallery],
                    crate::systems::world::playback::advance,
                ),
                while_in(
                    Screen::Editor,
                    (
                        crate::systems::editor::update,
                        crate::systems::screens::editor_panel::update,
                    ),
                ),
            ),
        );

        // Then presentation, in the order one part depends on the last. Motion
        // moves the actors, visibility decides which storey is on show, the
        // props follow the state, parts follow their owners, and the camera and
        // interface follow all of it.
        app.add_systems(
            Stage::Update,
            (
                crate::systems::world::motion::advance,
                crate::systems::world::visibility::update,
                crate::systems::world::motion::update_facing,
                crate::systems::world::props::update,
                crate::systems::world::beams::update,
                crate::systems::world::gems::update,
                crate::systems::world::watchers::update,
                crate::systems::world::motion::sync_parts,
                crate::systems::world::effects::update,
                crate::systems::world::camera::update,
                crate::systems::screens::hud::update,
                crate::systems::screens::fit::update,
                crate::systems::screens::objectives_panel::update,
            ),
        );
    }
}
