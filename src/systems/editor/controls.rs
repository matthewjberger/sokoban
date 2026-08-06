//! Everything that drives the editor from outside: the panel buttons, the
//! keyboard, the pointer painting on the ground plane, and the gamepad.

use crate::ecs::{Brush, EditorOverlay, Screen, SokobanResources};
use crate::schema::{Position, RULE_SWITCHES, map_blank, map_slot_for};
use crate::storage;
use crate::systems::editor::floors::{add_floor, change_layer, remove_floor, resize};
use crate::systems::editor::paint::paint;
use crate::systems::editor::{
    analyze, load, push_text_fields, randomize, revalidate, save, test_play,
};
use crate::systems::input::pad_pressed;

use crate::systems::world::build;
use nightshade::prelude::*;
use nightshade::ui::widgets::world_state::{ui_checkbox_changed, ui_text_input_changed};

/// A rule flipped in the panel changes the map value, and changing a rule can
/// change whether the map is still winnable, so the structural pass runs again.
pub fn read_rule_switches(game: &mut SokobanResources, world: &mut World) {
    if game.editor.overlay != EditorOverlay::Rules {
        return;
    }
    let boxes = game.ui.editor.rule_boxes.clone();
    for (index, entity) in boxes.iter().enumerate() {
        let Some(value) = ui_checkbox_changed(world, *entity) else {
            continue;
        };
        let Some((label, access)) = RULE_SWITCHES.get(index) else {
            continue;
        };
        *access(&mut game.editor.map.rules) = value;
        game.editor.status = format!("{label}: {}", if value { "yes" } else { "no" });
        revalidate(game);
    }
}

pub fn read_text_fields(game: &mut SokobanResources, world: &mut World) {
    if let Some(name) = ui_text_input_changed(world, game.ui.editor.name_input).map(str::to_string)
    {
        game.editor.map.name = name;
    }
    if let Some(hint) = ui_text_input_changed(world, game.ui.editor.hint_input).map(str::to_string)
    {
        game.editor.map.hint = hint;
    }
}

pub fn handle_buttons(game: &mut SokobanResources, world: &mut World) {
    let handles = game.ui.editor.clone();
    let clicks: Vec<Entity> = ui_button_clicks(world).collect();
    for entity in clicks {
        if let Some(index) = handles
            .brush_buttons
            .iter()
            .position(|button| *button == entity)
        {
            game.editor.brush = Brush::ALL[index];
            continue;
        }
        if entity == handles.group_button {
            game.editor.group = (game.editor.group + 1) % crate::rules::GATE_GROUPS as u8;
        } else if entity == handles.width_minus {
            resize(game, -1, 0);
        } else if entity == handles.width_plus {
            resize(game, 1, 0);
        } else if entity == handles.height_minus {
            resize(game, 0, -1);
        } else if entity == handles.height_plus {
            resize(game, 0, 1);
        } else if entity == handles.rules_button {
            game.editor.overlay = EditorOverlay::Rules;
        } else if entity == handles.schema_button {
            game.editor.overlay = EditorOverlay::Schema;
        } else if entity == handles.rules_close || entity == handles.schema_close {
            game.editor.overlay = EditorOverlay::None;
        } else if entity == handles.randomize_button {
            game.editor.overlay = EditorOverlay::Confirm;
        } else if entity == handles.confirm_no {
            game.editor.overlay = EditorOverlay::None;
        } else if entity == handles.confirm_yes {
            game.editor.overlay = EditorOverlay::None;
            randomize(game, world);
        } else if entity == handles.character_button {
            game.editor.map.character = game.editor.map.next_free_character();
            game.editor.needs_rebuild = true;
            game.editor.status = format!(
                "{} {}",
                game.editor.map.character.label(),
                game.editor.map.character.blurb()
            );
        } else if entity == handles.skin_button {
            game.editor.map.skin = game.editor.map.skin.next();
            game.editor.needs_rebuild = true;
            game.editor.status = format!("skin is {}", game.editor.map.skin.label());
        } else if entity == handles.win_button {
            game.editor.map.rules.win = game.editor.map.rules.win.next();
            game.editor.status = format!("win by {}", game.editor.map.rules.win.label());
        } else if entity == handles.layer_down {
            change_layer(game, -1);
        } else if entity == handles.layer_up {
            change_layer(game, 1);
        } else if entity == handles.add_floor {
            add_floor(game);
        } else if entity == handles.remove_floor {
            remove_floor(game);
        } else if entity == handles.previous_slot {
            game.editor.slot_index = game.editor.slot_index.saturating_sub(1);
        } else if entity == handles.next_slot {
            let limit = game.editor.slots.len().saturating_sub(1);
            game.editor.slot_index = (game.editor.slot_index + 1).min(limit);
        } else if entity == handles.new_button {
            game.editor.map = map_blank(11, 9);
            game.editor.cursor = Position::new(0, (1, 1));
            game.editor.needs_rebuild = true;
            game.editor.status = "new map".to_string();
            push_text_fields(game, world);
        } else if entity == handles.analyze_button {
            analyze(game);
        } else if entity == handles.test_button {
            test_play(game, world);
            return;
        } else if entity == handles.save_button {
            save(game);
        } else if entity == handles.load_button {
            load(game, world);
        } else if entity == handles.copy_button {
            game.editor.status = match storage::copy_to_clipboard(&game.editor.map) {
                Ok(message) => message,
                Err(message) => message,
            };
        } else if entity == handles.back_button {
            next_state(world, Screen::Title);
            return;
        }
    }
}

pub fn handle_keys(game: &mut SokobanResources, world: &mut World) {
    let keyboard = &world.res::<Input>().keyboard;
    let brush = BRUSH_KEYS
        .iter()
        .position(|key| keyboard.just_pressed(*key))
        .and_then(|index| Brush::ALL.get(index).copied());
    let test = keyboard.just_pressed(KeyCode::Enter);
    let analyze_now = keyboard.just_pressed(KeyCode::KeyV);
    let back = keyboard.just_pressed(KeyCode::Escape);
    let layer_up = keyboard.just_pressed(KeyCode::PageUp);
    let layer_down = keyboard.just_pressed(KeyCode::PageDown);

    if let Some(brush) = brush {
        game.editor.brush = brush;
    }
    if layer_up {
        change_layer(game, 1);
    }
    if layer_down {
        change_layer(game, -1);
    }
    if analyze_now {
        analyze(game);
    }
    if test {
        test_play(game, world);
        return;
    }
    if back {
        // A panel is covering the board, so the first way out is closing it.
        if game.editor.overlay == EditorOverlay::None {
            next_state(world, Screen::Title);
        } else {
            game.editor.overlay = EditorOverlay::None;
        }
    }
}

pub fn handle_pointer(game: &mut SokobanResources, world: &mut World) {
    if game.editor.overlay != EditorOverlay::None || nightshade::ui::ui_wants_pointer(world) {
        game.editor.painting = false;
        return;
    }

    let state = world.res::<Input>().mouse.state;
    let screen = world.res::<Input>().mouse.position;
    let plane = build::layer_height(game.editor.cursor.layer);
    let Some(ground) = get_ground_position_from_screen(world, vec2(screen.x, screen.y), plane)
    else {
        return;
    };
    let at = Position::new(
        game.editor.cursor.layer,
        (ground.x.round() as i32, ground.z.round() as i32),
    );
    let moved = at != game.editor.cursor;
    game.editor.cursor = at;
    game.editor.slot = map_slot_for(&game.editor.map, at).0;

    let left = state.contains(MouseState::LEFT_CLICKED);
    let right = state.contains(MouseState::RIGHT_CLICKED);
    let started = state.contains(MouseState::LEFT_JUST_PRESSED)
        || state.contains(MouseState::RIGHT_JUST_PRESSED);

    if (left || right) && (started || moved || !game.editor.painting) {
        let brush = if right {
            Brush::Erase
        } else {
            game.editor.brush
        };
        paint(game, at, brush);
        game.editor.painting = true;
    }
    if !left && !right {
        game.editor.painting = false;
    }
}

pub fn handle_pad(game: &mut SokobanResources, world: &mut World) {
    if game.editor.overlay != EditorOverlay::None {
        return;
    }
    let mut cell = game.editor.cursor.cell;
    if pad_pressed(world, gilrs::Button::DPadUp) {
        cell.1 -= 1;
    }
    if pad_pressed(world, gilrs::Button::DPadDown) {
        cell.1 += 1;
    }
    if pad_pressed(world, gilrs::Button::DPadLeft) {
        cell.0 -= 1;
    }
    if pad_pressed(world, gilrs::Button::DPadRight) {
        cell.0 += 1;
    }
    game.editor.cursor.cell = cell;
    game.editor.slot = map_slot_for(&game.editor.map, game.editor.cursor).0;

    if pad_pressed(world, gilrs::Button::LeftTrigger) {
        change_layer(game, -1);
    }
    if pad_pressed(world, gilrs::Button::RightTrigger) {
        change_layer(game, 1);
    }
    if pad_pressed(world, gilrs::Button::South) {
        let at = game.editor.cursor;
        let brush = game.editor.brush;
        paint(game, at, brush);
    }
    if pad_pressed(world, gilrs::Button::East) {
        let at = game.editor.cursor;
        paint(game, at, Brush::Erase);
    }
    if pad_pressed(world, gilrs::Button::West) {
        let index = Brush::ALL
            .iter()
            .position(|brush| *brush == game.editor.brush)
            .unwrap_or(0);
        game.editor.brush = Brush::ALL[(index + 1) % Brush::ALL.len()];
    }
    if pad_pressed(world, gilrs::Button::North) {
        test_play(game, world);
    }
}

const BRUSH_KEYS: [KeyCode; 12] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::Digit0,
    KeyCode::Minus,
    KeyCode::Backspace,
];
