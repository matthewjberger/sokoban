//! The settings screen. Every row is one switch on [`Settings`], listed from
//! the same table the systems read, so a setting added there appears here
//! without this knowing what it was.

use crate::ecs::{SETTING_SWITCHES, Screen, SettingsHandles, SokobanResources};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::systems::world::effects;
use crate::theme::*;
use nightshade::prelude::*;

const ROW_WIDTH: f32 = 560.0;
const ROW_HEIGHT: f32 = 46.0;
const ROW_SIZE: Vec2 = Vec2::new(ROW_WIDTH, ROW_HEIGHT);
const ROW_GAP: f32 = 8.0;
/// The rows and the gaps between them. The column has to say how tall it really
/// is or it runs over whatever sits below it.
const COLUMN_HEIGHT: f32 =
    ROW_HEIGHT * SETTING_SWITCHES.len() as f32 + ROW_GAP * (SETTING_SWITCHES.len() as f32 - 1.0);

pub fn build(tree: &mut UiTreeBuilder) -> SettingsHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.2)
        .entity();

    let mut handles = SettingsHandles {
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
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -(COLUMN_HEIGHT * 0.5) - 96.0)),
                Ab(vec2(760.0, 56.0)),
                Anchor::Center,
            )
            .with_text("SETTINGS", 42.0)
            .text_center()
            .with_text_outline(ACCENT, 1.8)
            .color_raw::<UiBase>(WHITE)
            .entity();

        let column = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -(COLUMN_HEIGHT * 0.5))),
                Ab(vec2(ROW_WIDTH, COLUMN_HEIGHT)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, ROW_GAP)
            .entity();
        tree.in_parent(column, |tree| {
            for (name, blurb, _) in SETTING_SWITCHES {
                let (row, value) = build_row(tree, name, blurb);
                handles.rows.push(row);
                handles.values.push(value);
            }
        });

        let menu = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, COLUMN_HEIGHT * 0.5 + 24.0)),
                Ab(vec2(MENU_BUTTON_SIZE.x, 52.0)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, 10.0)
            .entity();
        tree.in_parent(menu, |tree| {
            handles.back_button = widgets::build(tree, "BACK");
        });
    });

    handles
}

/// A row: what the setting is called, what it does in a few words, and whether
/// it is on. The whole row is the button, so there is no small target to find.
fn build_row(tree: &mut UiTreeBuilder, name: &str, blurb: &str) -> (Entity, Entity) {
    let row = tree
        .add_node()
        .flow_child(Ab(ROW_SIZE))
        .with_rect(5.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG)
        .color_raw::<UiHover>(PANEL_HOVER)
        .color_raw::<UiPressed>(PANEL_PRESSED)
        .with_transition::<UiHover>(14.0, 8.0)
        .with_transition::<UiPressed>(20.0, 12.0)
        .with_interaction()
        .with_cursor_icon(winit::window::CursorIcon::Pointer)
        .entity();

    let mut value = Entity::default();
    tree.in_parent(row, |tree| {
        tree.add_node()
            .window(Ab(vec2(18.0, 7.0)), Ab(vec2(400.0, 20.0)), Anchor::TopLeft)
            .with_text(name, 15.0)
            .text_left()
            .color_raw::<UiBase>(WHITE)
            .entity();

        tree.add_node()
            .window(Ab(vec2(18.0, 26.0)), Ab(vec2(400.0, 16.0)), Anchor::TopLeft)
            .with_text(blurb, 11.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        value = tree
            .add_node()
            .window(
                Rl(vec2(100.0, 50.0)) + Ab(vec2(-18.0, 0.0)),
                Ab(vec2(80.0, 24.0)),
                Anchor::CenterRight,
            )
            .with_text("", 15.0)
            .text_right()
            .color_raw::<UiBase>(ACCENT)
            .entity();
    });

    (row, value)
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Settings) {
        return;
    }

    let handles = game.ui.settings.clone();
    let mut back = world.res::<Input>().keyboard.just_pressed(KeyCode::Escape)
        || pad_pressed(world, gilrs::Button::East);
    let mut toggled = None;

    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.back_button {
            back = true;
        } else if let Some(index) = handles.rows.iter().position(|row| *row == entity) {
            toggled = Some(index);
        }
    }

    if let Some(index) = toggled
        && let Some((_, _, access)) = SETTING_SWITCHES.get(index)
    {
        let flag = access(&mut game.settings);
        *flag = !*flag;
        effects::apply(&game.settings, world);
    }

    update_labels(game, world);

    if back {
        next_state(world, Screen::Title);
    }
}

pub fn update_labels(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.settings;
    let mut settings = game.settings;
    for (index, (_, _, access)) in SETTING_SWITCHES.iter().enumerate() {
        let on = *access(&mut settings);
        let Some(value) = handles.values.get(index).copied() else {
            continue;
        };
        ui_set_text(world, value, if on { "ON" } else { "OFF" });
        if let Some(color) = world.get_mut::<UiNodeColor>(value) {
            color.colors[UiBase::INDEX] = Some(if on { ACCENT } else { TEXT_FAINT });
        }
    }
}
