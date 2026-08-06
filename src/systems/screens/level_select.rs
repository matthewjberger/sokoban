//! The level picker. Every board in the campaign, in one list, grouped by the
//! area it belongs to and playable from a cold start. Nothing here is gated:
//! story mode is where the order is the point, and this is where the boards are
//! just boards.

use crate::ecs::{LevelSelectHandles, MapOrigin, MapRequest, Screen, SokobanResources};
use crate::maps::{load_map, map_count};
use crate::schema::{describe, mechanics};
use crate::story::{area_of, areas};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::theme::*;
use nightshade::prelude::*;

const PANEL_WIDTH: f32 = 620.0;
const ROW_HEIGHT: f32 = 42.0;
const ROW_GAP: f32 = 5.0;
const HEADING_HEIGHT: f32 = 26.0;
/// What the panel leaves at the top for its own title and at the bottom for the
/// way out, so the list gets the rest of the height whatever the window is.
const LIST_TOP: f32 = 92.0;
const LIST_FOOT: f32 = 78.0;

pub fn build(tree: &mut UiTreeBuilder) -> LevelSelectHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.2)
        .entity();

    let mut handles = LevelSelectHandles {
        root,
        ..Default::default()
    };

    tree.in_parent(root, |tree| {
        tree.add_node()
            .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(BACKDROP)
            .entity();

        let panel = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)),
                Ab(vec2(PANEL_WIDTH, 0.0)) + Rl(vec2(0.0, 88.0)),
                Anchor::Center,
            )
            .with_rect(5.0, 1.0, PANEL_BORDER)
            .color_raw::<UiBase>(PANEL_BG_DEEP)
            .entity();

        tree.in_parent(panel, |tree| {
            tree.add_node()
                .window(
                    Ab(vec2(0.0, 24.0)) + Rl(vec2(50.0, 0.0)),
                    Ab(vec2(PANEL_WIDTH - 48.0, 34.0)),
                    Anchor::TopCenter,
                )
                .with_text("PICK A BOARD", 26.0)
                .text_center()
                .with_text_outline(ACCENT, 1.4)
                .color_raw::<UiBase>(WHITE)
                .entity();

            tree.add_node()
                .window(
                    Ab(vec2(0.0, 58.0)) + Rl(vec2(50.0, 0.0)),
                    Ab(vec2(PANEL_WIDTH - 48.0, 18.0)),
                    Anchor::TopCenter,
                )
                .with_text("all of them, in any order", 12.0)
                .text_center()
                .color_raw::<UiBase>(TEXT_FAINT)
                .entity();

            let list = tree
                .add_node()
                .window(
                    Ab(vec2(24.0, LIST_TOP)),
                    Rl(vec2(0.0, 100.0)) + Ab(vec2(PANEL_WIDTH - 48.0, -(LIST_TOP + LIST_FOOT))),
                    Anchor::TopLeft,
                )
                .flow(FlowDirection::Vertical, 0.0, ROW_GAP)
                .entity();
            tree.in_parent(list, |tree| {
                let scroll = tree.add_scroll_area_fill(0.0, ROW_GAP);
                let content = widget::<UiScrollAreaData>(tree.world_mut(), scroll)
                    .map(|data| data.content_entity)
                    .unwrap_or(scroll);
                tree.in_parent(content, |tree| {
                    let mut area = usize::MAX;
                    for level in 0..map_count() {
                        if area_of(level) != area {
                            area = area_of(level);
                            build_heading(tree, &areas()[area].name);
                        }
                        handles.items.push(build_row(tree, level));
                    }
                });
            });

            let footer = tree
                .add_node()
                .window(
                    Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -60.0)),
                    Ab(vec2(MENU_BUTTON_SIZE.x, 52.0)),
                    Anchor::TopCenter,
                )
                .flow(FlowDirection::Vertical, 0.0, 8.0)
                .entity();
            tree.in_parent(footer, |tree| {
                handles.back_button = widgets::build(tree, "BACK");
            });
        });
    });

    handles
}

fn build_heading(tree: &mut UiTreeBuilder, name: &str) {
    let heading = tree
        .add_node()
        .flow_child(Ab(vec2(PANEL_WIDTH - 68.0, HEADING_HEIGHT)))
        .entity();
    tree.in_parent(heading, |tree| {
        tree.add_node()
            .window(
                Ab(vec2(4.0, 6.0)),
                Ab(vec2(PANEL_WIDTH - 80.0, 18.0)),
                Anchor::TopLeft,
            )
            .with_text(name, 13.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();
    });
}

/// One board: what number it is, what it is called, and what it is made of. The
/// whole row is the button, so there is no small target to find.
fn build_row(tree: &mut UiTreeBuilder, level: usize) -> Entity {
    let map = load_map(level);
    let row = tree
        .add_node()
        .flow_child(Ab(vec2(PANEL_WIDTH - 68.0, ROW_HEIGHT)))
        .with_rect(4.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG)
        .color_raw::<UiHover>(PANEL_HOVER)
        .color_raw::<UiPressed>(PANEL_PRESSED)
        .with_transition::<UiHover>(14.0, 8.0)
        .with_transition::<UiPressed>(20.0, 12.0)
        .with_interaction()
        .with_cursor_icon(winit::window::CursorIcon::Pointer)
        .entity();

    tree.in_parent(row, |tree| {
        tree.add_node()
            .window(Ab(vec2(14.0, 11.0)), Ab(vec2(34.0, 20.0)), Anchor::TopLeft)
            .with_text(&format!("{}", level + 1), 16.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        tree.add_node()
            .window(Ab(vec2(54.0, 6.0)), Ab(vec2(280.0, 20.0)), Anchor::TopLeft)
            .with_text(&map.name, 16.0)
            .text_left()
            .color_raw::<UiBase>(WHITE)
            .entity();

        tree.add_node()
            .window(Ab(vec2(54.0, 24.0)), Ab(vec2(480.0, 16.0)), Anchor::TopLeft)
            .with_text(&describe(&mechanics(&map)), 11.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    });

    row
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::LevelSelect) {
        return;
    }

    let handles = game.ui.levels.clone();
    let mut back = world.res::<Input>().keyboard.just_pressed(KeyCode::Escape)
        || pad_pressed(world, gilrs::Button::East);
    let mut chosen = None;

    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.back_button {
            back = true;
        } else if let Some(level) = handles.items.iter().position(|item| *item == entity) {
            chosen = Some(level);
        }
    }

    if let Some(level) = chosen {
        game.selected_map = level;
        game.pending = Some(MapRequest {
            map: load_map(level),
            origin: MapOrigin::Campaign(level),
        });
        next_state(world, Screen::InGame);
        return;
    }
    if back {
        next_state(world, Screen::Title);
    }
}
