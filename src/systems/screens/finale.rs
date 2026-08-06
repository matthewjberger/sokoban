//! The screen that closes the campaign out, and the buttons that leave it.

use crate::ecs::{FinaleHandles, Screen, SokobanResources};
use crate::maps::map_count;
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::theme::*;
use nightshade::prelude::*;

pub fn build_finale(tree: &mut UiTreeBuilder) -> FinaleHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.35)
        .entity();

    let mut stats_label = Entity::default();
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
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -160.0)),
                Ab(vec2(900.0, 74.0)),
                Anchor::Center,
            )
            .with_text("WAREHOUSE CLEARED", 52.0)
            .text_center()
            .color_raw::<UiBase>(WHITE)
            .with_text_outline(ACCENT, 2.0)
            .entity();

        stats_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -100.0)),
                Ab(vec2(820.0, 26.0)),
                Anchor::Center,
            )
            .with_text("", 17.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        let column = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -56.0)),
                Ab(vec2(MENU_BUTTON_SIZE.x, 120.0)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, 10.0)
            .entity();
        tree.in_parent(column, |tree| {
            menu_button_entity = widgets::build(tree, "MAIN MENU");
            quit_button = widgets::build(tree, "QUIT");
        });
    });

    FinaleHandles {
        root,
        stats_label,
        menu_button: menu_button_entity,
        quit_button,
    }
}

pub fn populate_finale(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.finale;
    ui_set_text(
        world,
        handles.stats_label,
        &format!(
            "ALL {} MAPS SOLVED   ·   {} TOTAL MOVES",
            map_count(),
            game.total_moves
        ),
    );
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if in_state(world, Screen::CampaignComplete) {
        let handles = &game.ui.finale;
        let mut menu = world.res::<Input>().keyboard.just_pressed(KeyCode::Enter)
            || pad_pressed(world, gilrs::Button::East);
        let mut quit = false;
        for entity in ui_button_clicks(world) {
            if entity == handles.menu_button {
                menu = true;
            } else if entity == handles.quit_button {
                quit = true;
            }
        }
        if menu {
            game.selected_map = 0;
            next_state(world, Screen::Title);
        } else if quit {
            world.res_mut::<Window>().should_exit = true;
        }
    }
}
