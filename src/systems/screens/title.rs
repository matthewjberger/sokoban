//! The front screen. Two menus in one place: the first says what kind of thing
//! you want to do, and the one behind PLAY says which way you want to play. A
//! shipped board is picked on its own screen and a random one is not picked at
//! all, so nothing here has to carry a list.

use crate::ecs::{
    Making, MapOrigin, MapRequest, Screen, SokobanResources, TitleHandles, TitleMenu,
};
use crate::maps::load_map;
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::systems::world::work;
use crate::theme::*;
use nightshade::prelude::*;

const MENU_GAP: f32 = 10.0;
/// What a column leaves below itself for the control legend along the foot.
const MENU_FOOTER_SPACE: f32 = 54.0;

const TITLE_TEXT: &str = "SOKOBAN";
const SUBTITLE_TEXT: &str = "PUSH EVERY CRATE ONTO A MARKER";

pub fn build(tree: &mut UiTreeBuilder) -> TitleHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_intro(UiAnimationType::Fade, 0.4)
        .entity();

    let mut handles = TitleHandles {
        root,
        ..Default::default()
    };

    tree.in_parent(root, |tree| {
        tree.add_node()
            .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(BACKDROP)
            .entity();

        tree.add_node()
            .window(
                Ab(vec2(0.0, 110.0)) + Rl(vec2(50.0, 0.0)),
                Ab(vec2(900.0, 92.0)),
                Anchor::TopCenter,
            )
            .with_text(TITLE_TEXT, 68.0)
            .text_center()
            .with_text_outline(ACCENT, 2.0)
            .color_raw::<UiBase>(WHITE)
            .entity();

        tree.add_node()
            .window(
                Ab(vec2(0.0, 194.0)) + Rl(vec2(50.0, 0.0)),
                Ab(vec2(760.0, 24.0)),
                Anchor::TopCenter,
            )
            .with_text(SUBTITLE_TEXT, 16.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        build_menus(tree, &mut handles);

        handles.status_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -MENU_FOOTER_SPACE + 8.0)),
                Ab(vec2(900.0, 20.0)),
                Anchor::TopCenter,
            )
            .with_text("", 14.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        tree.add_node()
            .window(
                Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -26.0)),
                Ab(vec2(1000.0, 18.0)),
                Anchor::TopCenter,
            )
            .with_text(
                "WASD / D-PAD MOVE   ·   Q E RIDE ELEVATORS   ·   Z UNDO   ·   R RESTART   ·   TRIGGERS CHANGE SPEED",
                12.0,
            )
            .text_center()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    });

    handles
}

/// A column of buttons standing on the foot of the screen. Its height comes
/// from how many buttons are in it, so a menu with more of them grows upward
/// rather than off the bottom.
fn build_column(tree: &mut UiTreeBuilder, buttons: usize) -> Entity {
    let count = buttons as f32;
    let height = MENU_BUTTON_HEIGHT * count + MENU_GAP * (count - 1.0);
    tree.add_node()
        .window(
            Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -(height + MENU_FOOTER_SPACE))),
            Ab(vec2(MENU_BUTTON_SIZE.x, height)),
            Anchor::TopCenter,
        )
        .flow(FlowDirection::Vertical, 0.0, MENU_GAP)
        .entity()
}

fn build_menus(tree: &mut UiTreeBuilder, handles: &mut TitleHandles) {
    handles.root_column = build_column(tree, 5);
    tree.in_parent(handles.root_column, |tree| {
        handles.play_button = widgets::build(tree, "PLAY");
        handles.gallery_button = widgets::build(tree, "GALLERY");
        handles.editor_button = widgets::build(tree, "CREATE");
        handles.settings_button = widgets::build(tree, "SETTINGS");
        handles.quit_button = widgets::build(tree, "QUIT");
    });

    handles.play_column = build_column(tree, 6);
    tree.in_parent(handles.play_column, |tree| {
        handles.story_button = widgets::build(tree, "STORY MODE");
        handles.campaign_button = widgets::build(tree, "CAMPAIGN");
        handles.levels_button = widgets::build(tree, "PICK A BOARD");
        handles.random_button = widgets::build(tree, "RANDOM PUZZLE");
        handles.endless_button = widgets::build(tree, "ENDLESS RUN");
        handles.play_back_button = widgets::build(tree, "BACK");
    });
}

/// Shows whichever menu is open. Called when the screen is entered and after
/// anything that changes which one that is, so the two are never both up.
pub fn update_menu(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.title;
    let root = matches!(game.title_menu, TitleMenu::Root);
    ui_set_visible(world, handles.root_column, root);
    ui_set_visible(world, handles.play_column, !root);
}

/// Puts whatever the generator is doing under the menu. There is no screen
/// between the button and the board any more, so the wait is shown here. The
/// line holds still while the work runs, so writing it every frame is a write
/// the text cache throws away rather than a layout of the whole tree.
fn say_status(game: &SokobanResources, world: &mut World) {
    let label = game.ui.title.status_label;
    let line = game.random_status.clone();
    ui_set_text(world, label, &line);
}

/// Which button a pad should land on when a menu opens.
pub fn menu_focus(game: &SokobanResources) -> Entity {
    match game.title_menu {
        TitleMenu::Root => game.ui.title.play_button,
        TitleMenu::Play => game.ui.title.story_button,
    }
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Title) {
        return;
    }

    say_status(game, world);

    let handles = game.ui.title.clone();
    let keyboard = &world.res::<Input>().keyboard;
    let mut gallery = keyboard.just_pressed(KeyCode::KeyG);
    let mut editor = keyboard.just_pressed(KeyCode::KeyE);
    let mut settings = keyboard.just_pressed(KeyCode::KeyT);
    let mut back =
        keyboard.just_pressed(KeyCode::Escape) || pad_pressed(world, gilrs::Button::East);
    let mut open_play = false;
    let mut story = false;
    let mut campaign = false;
    let mut levels = false;
    let mut random = false;
    let mut endless = false;
    let mut quit = false;

    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.play_button {
            open_play = true;
        } else if entity == handles.gallery_button {
            gallery = true;
        } else if entity == handles.editor_button {
            editor = true;
        } else if entity == handles.settings_button {
            settings = true;
        } else if entity == handles.quit_button {
            quit = true;
        } else if entity == handles.story_button {
            story = true;
        } else if entity == handles.campaign_button {
            campaign = true;
        } else if entity == handles.levels_button {
            levels = true;
        } else if entity == handles.random_button {
            random = true;
        } else if entity == handles.endless_button {
            endless = true;
        } else if entity == handles.play_back_button {
            back = true;
        }
    }

    // Opening and closing the second menu is the only thing on this screen that
    // is not a change of screen, so it is settled first and everything below it
    // is about leaving.
    if open_play {
        game.title_menu = TitleMenu::Play;
    } else if back {
        game.title_menu = TitleMenu::Root;
    }
    if open_play || back {
        update_menu(game, world);
        world.res_mut::<RetainedUiInteraction>().focused_entity = Some(menu_focus(game));
        return;
    }

    if campaign {
        game.pending = Some(MapRequest {
            map: load_map(game.selected_map),
            origin: MapOrigin::Campaign(game.selected_map),
        });
        next_state(world, Screen::InGame);
    }
    if levels {
        next_state(world, Screen::LevelSelect);
    }
    // A board rather than a screen of dials to describe one. What comes out is
    // rolled across every mechanic, every shape and every party the game has,
    // and the screen it goes to is the board itself.
    if random {
        work::make(game, Making::Single);
    }
    if gallery {
        next_state(world, Screen::Gallery);
    }
    if editor {
        next_state(world, Screen::Editor);
    }
    if settings {
        next_state(world, Screen::Settings);
    }
    if endless {
        work::make(game, Making::RunStart);
    }
    if story {
        next_state(world, Screen::Story);
    }
    if quit {
        world.res_mut::<Window>().should_exit = true;
    }
}
