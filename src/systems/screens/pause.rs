use crate::ecs::{MapOrigin, PauseHandles, Screen, SokobanResources};
use crate::systems::input::{pad_pressed, restart};
use crate::systems::screens::widgets;
use crate::systems::world::work;
use crate::theme::*;
use nightshade::prelude::*;

/// The buttons in the pause column, which decide how tall it is. Deriving the
/// height from the count means adding one never runs it off the screen.
const PAUSE_BUTTONS: f32 = 5.0;
const PAUSE_GAP: f32 = 10.0;
const PAUSE_COLUMN_HEIGHT: f32 =
    MENU_BUTTON_HEIGHT * PAUSE_BUTTONS + PAUSE_GAP * (PAUSE_BUTTONS - 1.0);

pub fn build(tree: &mut UiTreeBuilder) -> PauseHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.18)
        .entity();

    let mut resume_button = Entity::default();
    let mut restart_button = Entity::default();
    let mut solve_button = Entity::default();
    let mut menu_button_entity = Entity::default();
    let mut quit_button = Entity::default();

    tree.in_parent(root, |tree| {
        tree.add_node()
            .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(BACKDROP)
            .entity();

        tree.add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -170.0)),
                Ab(vec2(600.0, 60.0)),
                Anchor::Center,
            )
            .with_text("PAUSED", 46.0)
            .text_center()
            .color_raw::<UiBase>(WHITE)
            .with_text_outline(ACCENT, 1.8)
            .entity();

        let column = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -(PAUSE_COLUMN_HEIGHT * 0.5))),
                Ab(vec2(MENU_BUTTON_SIZE.x, PAUSE_COLUMN_HEIGHT)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, PAUSE_GAP)
            .entity();
        tree.in_parent(column, |tree| {
            resume_button = widgets::build(tree, "RESUME");
            restart_button = widgets::build(tree, "RESTART LEVEL");
            solve_button = widgets::build(tree, "SOLVE IT");
            menu_button_entity = widgets::build(tree, "MAIN MENU");
            quit_button = widgets::build(tree, "QUIT");
        });
    });

    PauseHandles {
        root,
        resume_button,
        restart_button,
        solve_button,
        menu_button: menu_button_entity,
        quit_button,
    }
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Paused) {
        return;
    }

    let handles = &game.ui.pause;
    let mut resume = pad_pressed(world, gilrs::Button::East);
    let mut retry = false;
    let mut solve = false;
    let mut menu = false;
    let mut quit = false;
    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.resume_button {
            resume = true;
        } else if entity == handles.restart_button {
            retry = true;
        } else if entity == handles.solve_button {
            solve = true;
        } else if entity == handles.menu_button {
            menu = true;
        } else if entity == handles.quit_button {
            quit = true;
        }
    }

    if solve {
        // The board is handed to the search and the answer plays itself out
        // from the start, at the speed a person can follow, on whatever frame
        // the search finishes. Reaching for the controls takes it back, exactly
        // as it does during a worked example.
        work::solve(game);
        next_state(world, Screen::InGame);
    } else if retry {
        restart(game, world);
        next_state(world, Screen::InGame);
    } else if resume {
        next_state(world, Screen::InGame);
    } else if menu {
        if let MapOrigin::Campaign(index) = game.origin {
            game.selected_map = index;
        }
        next_state(world, Screen::Title);
    } else if quit {
        world.res_mut::<Window>().should_exit = true;
    }
}
