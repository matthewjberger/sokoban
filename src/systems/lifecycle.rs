use crate::ecs::{Playback, Screen, SokobanResources, TitleMenu, register_sokoban_components};
use crate::systems::editor;
use crate::systems::screens::{
    complete, editor_panel, finale, gallery, hud, level_select, objectives_panel, pause,
    random_setup, settings, title,
};
use crate::systems::world::build;
use nightshade::prelude::*;

pub fn initialize(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    world.ecs.add_world_at(GAME, register_sokoban_components());

    configure_render(world);
    upload_vfx_textures(world);
    crate::systems::world::effects::apply(&game.settings, world);
    spawn_view_camera(game, world);

    let mut tree = UiTreeBuilder::new(world);
    let title_handles = title::build(&mut tree);
    let level_handles = level_select::build(&mut tree);
    let hud_handles = hud::build(&mut tree);
    let pause_handles = pause::build(&mut tree);
    let complete_handles = complete::build_panel(&mut tree);
    let finale_handles = finale::build_finale(&mut tree);
    let random_handles = random_setup::build(&mut tree);
    let settings_handles = settings::build(&mut tree);
    let gallery_handles = gallery::build(&mut tree);
    let editor_handles = editor_panel::build(&mut tree);
    let objective_handles = objectives_panel::build(&mut tree);
    game.ui.root = tree.finish();
    game.ui.title = title_handles;
    game.ui.levels = level_handles;
    game.ui.hud = hud_handles;
    game.ui.pause = pause_handles;
    game.ui.complete = complete_handles;
    game.ui.finale = finale_handles;
    game.ui.random = random_handles;
    game.ui.settings = settings_handles;
    game.ui.gallery = gallery_handles;
    game.ui.editor = editor_handles;
    game.ui.objectives = objective_handles;

    game.selected_map = 0;
    title::update_menu(game, world);
}

fn configure_render(world: &mut World) {
    world.res_mut::<DebugDraw>().show_grid = false;

    let settings = world.res_mut::<RenderSettings>();
    settings.bloom_enabled = true;
    settings.bloom_intensity = 0.22;
    settings.bloom_threshold = 1.0;
    settings.ssao_enabled = true;
    settings.ssao_radius = 0.7;
    settings.ssao_intensity = 0.9;
    // The board is looked at from overhead and mostly holds still, so a pass
    // that jitters the camera by a fraction of a pixel every frame and blends
    // what it finds has nothing to gain and a whole board to shimmer. Off, and
    // the edges are as still as the board is.
    settings.taa_enabled = false;
    // Ice and water are only worth their materials if there is something for
    // them to reflect, and the water surface does not draw at all without its
    // own pass turned on.
    settings.water_enabled = true;
    settings.ssr_enabled = true;
    // Full strength, because how much of it a surface takes is now decided by
    // the surface. A mirror shows everything and a concrete floor shows a
    // glint at a grazing angle and nothing head on.
    settings.ssr_intensity = 1.0;
    settings.ssr_max_distance = 14.0;
    settings.ssr_fade_start = 0.6;
    settings.ssr_fade_end = 1.0;
}

fn spawn_view_camera(game: &mut SokobanResources, world: &mut World) {
    let camera = spawn_camera(
        world,
        Vec3::new(0.0, 10.0, 8.0),
        "SokobanCamera".to_string(),
    );
    world.set(
        camera,
        Camera {
            projection: Projection::Perspective(PerspectiveCamera {
                aspect_ratio: None,
                y_fov_rad: 45.0_f32.to_radians(),
                z_near: 0.1,
                z_far: Some(400.0),
            }),
            smoothing: None,
        },
    );
    world.res_mut::<ActiveCamera>().0 = Some(camera);
    game.camera.entity = camera;
}

pub fn enter(game: &mut SokobanResources, world: &mut World, screen: Screen) {
    // A search that no longer concerns where the player has arrived is dropped
    // rather than left to hand its answer over to whatever is up by then.
    if game
        .work
        .as_ref()
        .is_some_and(|work| !crate::systems::world::work::survives(work, screen))
    {
        game.work = None;
    }
    if let Some(request) = game.pending.take()
        && matches!(screen, Screen::InGame | Screen::Editor)
    {
        build::start_map(game, world, request);
    }
    // Arming has to follow the build, which clears any run in progress along
    // with the board it belonged to.
    if matches!(screen, Screen::InGame) {
        crate::systems::world::playback::arm_solution(game);
    }

    if matches!(screen, Screen::MapComplete) {
        complete::populate(game, world);
    }
    if matches!(screen, Screen::CampaignComplete) {
        finale::populate_finale(game, world);
    }
    if matches!(screen, Screen::RandomSetup) {
        random_setup::update_labels(game, world);
    }
    if matches!(screen, Screen::Settings) {
        settings::update_labels(game, world);
    }
    if matches!(screen, Screen::Story) {
        crate::systems::world::story::enter_overworld(game, world);
    }
    if matches!(screen, Screen::Gallery) {
        let lesson = game.selected_lesson;
        gallery::show(game, world, lesson);
        gallery::update_labels(game, world);
    } else {
        game.camera.shift = Vec2::new(0.0, 0.0);
    }
    // A run in progress belongs to a board in play. Anywhere else ends it, or
    // the flag that hands it the controls would still be holding them on
    // whatever comes next. Solve queues its run and then steps onto the board,
    // so the board itself is exactly where a run must survive arriving.
    if !matches!(screen, Screen::InGame | Screen::Gallery) {
        game.playback = Playback::default();
    }
    if matches!(screen, Screen::Title) {
        game.title_menu = TitleMenu::default();
        title::update_menu(game, world);
        // No board behind the menu. Whatever was being played is taken down
        // and the sky is put up in its place.
        crate::systems::world::attract::start(game, world);
    }
    if matches!(screen, Screen::Editor) {
        editor::on_enter(game, world);
    } else {
        editor::on_exit(game, world);
    }

    apply_visibility(game, world, screen);

    let focus = match screen {
        Screen::Title => Some(title::menu_focus(game)),
        // The first board on the list, so a pad arrives on something to press
        // rather than on the way out.
        Screen::LevelSelect => game
            .ui
            .levels
            .items
            .first()
            .copied()
            .or(Some(game.ui.levels.back_button)),
        Screen::RandomSetup => Some(game.ui.random.generate_button),
        Screen::Settings => Some(game.ui.settings.back_button),
        Screen::Story => None,
        Screen::Gallery => None,
        Screen::Paused => Some(game.ui.pause.resume_button),
        Screen::MapComplete => Some(game.ui.complete.next_button),
        Screen::CampaignComplete => Some(game.ui.finale.menu_button),
        Screen::InGame | Screen::Editor => None,
    };
    world.res_mut::<RetainedUiGamepadNav>().enabled = focus.is_some();
    world.res_mut::<RetainedUiInteraction>().focused_entity = focus;
    world.res_mut::<RetainedUiOverlays>().focus_ring_visible = focus.is_some();
    if focus.is_none() {
        world.res_mut::<RetainedUiGamepadNav>().held_direction = None;
    }

    set_cursor_visible(world, true);
}

fn apply_visibility(game: &SokobanResources, world: &mut World, screen: Screen) {
    let handles = &game.ui;
    ui_set_visible(world, handles.title.root, matches!(screen, Screen::Title));
    ui_set_visible(
        world,
        handles.levels.root,
        matches!(screen, Screen::LevelSelect),
    );
    ui_set_visible(
        world,
        handles.hud.root,
        matches!(
            screen,
            Screen::InGame | Screen::Paused | Screen::MapComplete | Screen::Story
        ),
    );
    ui_set_visible(world, handles.pause.root, matches!(screen, Screen::Paused));
    ui_set_visible(
        world,
        handles.complete.root,
        matches!(screen, Screen::MapComplete),
    );
    ui_set_visible(
        world,
        handles.finale.root,
        matches!(screen, Screen::CampaignComplete),
    );
    ui_set_visible(world, handles.editor.root, matches!(screen, Screen::Editor));
    ui_set_visible(
        world,
        handles.gallery.root,
        matches!(screen, Screen::Gallery),
    );
    ui_set_visible(
        world,
        handles.random.root,
        matches!(screen, Screen::RandomSetup),
    );
    ui_set_visible(
        world,
        handles.settings.root,
        matches!(screen, Screen::Settings),
    );
}
