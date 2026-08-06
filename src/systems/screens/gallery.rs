//! The mechanics gallery: one board per rule with nothing else on it, and the
//! words to go with it. The boards are ordinary maps, so everything the game
//! can do to a map it can do here, which is the point. Play with one, undo,
//! reset, and move on.

use crate::ecs::{GalleryHandles, MapOrigin, MapRequest, Screen, SokobanResources};
use crate::gallery::{Topic, lesson, lessons};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::systems::world::playback;
use crate::theme::*;
use nightshade::prelude::*;
use nightshade::ui::widgets::world_state::widget;

const RAIL_WIDTH: f32 = 250.0;
const ITEM_SIZE: Vec2 = Vec2::new(214.0, 42.0);
const CARD_WIDTH: f32 = 720.0;
const CARD_HEIGHT: f32 = 208.0;
/// Where the list of mechanics starts, and how much of the rail it has to leave
/// for the heading above it and the key legend below it. The list scrolls, so
/// this decides what is on screen rather than what fits.
const LIST_TOP: f32 = 82.0;
const LIST_INSET: f32 = LIST_TOP - CONTROLS_TOP + 12.0;

/// Where the key legend starts, measured up from the foot of the rail. The rows
/// stack down from there, so the block grows with the list rather than needing
/// its offsets kept in step by hand.
const CONTROLS_TOP: f32 = -(CONTROLS.len() as f32) * 22.0 - 20.0;

/// The keys, spelled out once. They live at the foot of the rail because they
/// are true of every board, unlike anything on the card.
const CONTROLS: [(&str, &str); 5] = [
    ("WASD", "move and push"),
    ("SHIFT", "drag, where allowed"),
    ("SPACE", "lift or seat a gem"),
    ("Z", "undo a move"),
    ("ESC", "back to the menu"),
];

pub fn build(tree: &mut UiTreeBuilder) -> GalleryHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.22)
        .entity();

    let mut handles = GalleryHandles {
        root,
        ..Default::default()
    };

    tree.in_parent(root, |tree| {
        build_rail(tree, &mut handles);
        build_card(tree, &mut handles);
    });

    handles
}

/// The list of mechanics down the left, in the order they build on each other.
fn build_rail(tree: &mut UiTreeBuilder, handles: &mut GalleryHandles) {
    let panel = tree
        .add_node()
        .window(
            Ab(vec2(20.0, 20.0)),
            Rl(vec2(0.0, 100.0)) + Ab(vec2(RAIL_WIDTH, -40.0)),
            Anchor::TopLeft,
        )
        .with_rect(7.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .with_intro(UiAnimationType::SlideRight, 0.28)
        .entity();

    tree.in_parent(panel, |tree| {
        tree.add_node()
            .window(Ab(vec2(18.0, 18.0)), Ab(vec2(210.0, 26.0)), Anchor::TopLeft)
            .with_text("MECHANICS", 19.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        tree.add_node()
            .window(Ab(vec2(18.0, 44.0)), Ab(vec2(210.0, 18.0)), Anchor::TopLeft)
            .with_text("one board each, nothing else on it", 11.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        tree.add_node()
            .window(Ab(vec2(18.0, 68.0)), Ab(vec2(214.0, 1.0)), Anchor::TopLeft)
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(PANEL_BORDER)
            .entity();

        let list = tree
            .add_node()
            .window(
                Ab(vec2(18.0, LIST_TOP)),
                Rl(vec2(0.0, 100.0)) + Ab(vec2(ITEM_SIZE.x, -LIST_INSET)),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Vertical, 0.0, 6.0)
            .entity();
        tree.in_parent(list, |tree| {
            let scroll = tree.add_scroll_area_fill(0.0, 6.0);
            let content = widget::<UiScrollAreaData>(tree.world_mut(), scroll)
                .map(|data| data.content_entity)
                .unwrap_or(scroll);
            tree.in_parent(content, |tree| {
                // Who you are and what the board does are different questions,
                // so the list asks them in separate breaths.
                for topic in Topic::ALL {
                    build_section(tree, topic.label());
                    for (index, entry) in lessons()
                        .iter()
                        .enumerate()
                        .filter(|(_, entry)| entry.topic == topic)
                    {
                        let number = handles.item_lessons.len() + 1;
                        let (button, bar, label) = build_item(tree, number, &entry.name);
                        handles.items.push(button);
                        handles.item_bars.push(bar);
                        handles.item_labels.push(label);
                        handles.item_lessons.push(index);
                    }
                }
            });
        });

        build_controls(tree);
    });
}

/// The key legend, parked at the foot of the rail where there is nothing else.
fn build_controls(tree: &mut UiTreeBuilder) {
    tree.add_node()
        .window(
            Ab(vec2(18.0, 0.0)) + Rl(vec2(0.0, 100.0)) + Ab(vec2(0.0, CONTROLS_TOP)),
            Ab(vec2(214.0, 1.0)),
            Anchor::BottomLeft,
        )
        .with_rect(0.0, 0.0, TRANSPARENT)
        .color_raw::<UiBase>(PANEL_BORDER)
        .entity();

    for (index, (key, meaning)) in CONTROLS.iter().enumerate() {
        let offset = CONTROLS_TOP + 24.0 + index as f32 * 22.0;
        tree.add_node()
            .window(
                Ab(vec2(18.0, 0.0)) + Rl(vec2(0.0, 100.0)) + Ab(vec2(0.0, offset)),
                Ab(vec2(46.0, 18.0)),
                Anchor::BottomLeft,
            )
            .with_text(key, 12.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        tree.add_node()
            .window(
                Ab(vec2(70.0, 0.0)) + Rl(vec2(0.0, 100.0)) + Ab(vec2(0.0, offset)),
                Ab(vec2(162.0, 18.0)),
                Anchor::BottomLeft,
            )
            .with_text(meaning, 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    }
}

/// A heading in the list, which is the whole of what separates one kind of
/// board from another.
fn build_section(tree: &mut UiTreeBuilder, name: &str) {
    let row = tree
        .add_node()
        .flow_child(Ab(vec2(ITEM_SIZE.x, 26.0)))
        .entity();
    tree.in_parent(row, |tree| {
        tree.add_node()
            .window(
                Ab(vec2(10.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(ITEM_SIZE.x - 20.0, 18.0)),
                Anchor::CenterLeft,
            )
            .with_text(name, 12.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();
    });
}

/// A list row: a number that stays quiet, the name, and a bar down the left
/// edge that lights when the row is the one being shown.
fn build_item(tree: &mut UiTreeBuilder, number: usize, name: &str) -> (Entity, Entity, Entity) {
    let button = tree
        .add_node()
        .flow_child(Ab(ITEM_SIZE))
        .with_rect(4.0, 1.0, TRANSPARENT)
        .color_raw::<UiBase>(PANEL_BG)
        .color_raw::<UiHover>(PANEL_HOVER)
        .color_raw::<UiPressed>(PANEL_PRESSED)
        .with_transition::<UiHover>(14.0, 8.0)
        .with_transition::<UiPressed>(20.0, 12.0)
        .with_interaction()
        .with_cursor_icon(winit::window::CursorIcon::Pointer)
        .entity();

    let mut bar = Entity::default();
    let mut label = Entity::default();
    tree.in_parent(button, |tree| {
        bar = tree
            .add_node()
            .window(
                Ab(vec2(10.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(3.0, ITEM_SIZE.y - 16.0)),
                Anchor::CenterLeft,
            )
            .with_rect(2.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(PANEL_BORDER)
            .entity();

        tree.add_node()
            .window(
                Ab(vec2(24.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(26.0, 20.0)),
                Anchor::CenterLeft,
            )
            .with_text(&format!("{number:02}"), 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        label = tree
            .add_node()
            .window(
                Ab(vec2(52.0, 0.0)) + Rl(vec2(0.0, 50.0)),
                Ab(vec2(ITEM_SIZE.x - 64.0, 22.0)),
                Anchor::CenterLeft,
            )
            .with_text(name, 14.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();
    });

    (button, bar, label)
}

/// The card under the board: what this mechanic is, what to try, and the way
/// out.
fn build_card(tree: &mut UiTreeBuilder, handles: &mut GalleryHandles) {
    let card = tree
        .add_node()
        .window(
            Rl(vec2(50.0, 100.0)) + Ab(vec2(RAIL_WIDTH * 0.5, -20.0)),
            Ab(vec2(CARD_WIDTH, CARD_HEIGHT)),
            Anchor::BottomCenter,
        )
        .with_rect(7.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .with_intro(UiAnimationType::SlideUp, 0.28)
        .entity();

    tree.in_parent(card, |tree| {
        handles.title_label = tree
            .add_node()
            .window(Ab(vec2(26.0, 18.0)), Ab(vec2(520.0, 30.0)), Anchor::TopLeft)
            .with_text("", 24.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        handles.counter_label = tree
            .add_node()
            .window(
                Rl(vec2(100.0, 0.0)) + Ab(vec2(-26.0, 24.0)),
                Ab(vec2(120.0, 20.0)),
                Anchor::TopRight,
            )
            .with_text("", 13.0)
            .text_right()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        tree.add_node()
            .window(
                Ab(vec2(26.0, 52.0)),
                Ab(vec2(CARD_WIDTH - 52.0, 1.0)),
                Anchor::TopLeft,
            )
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(PANEL_BORDER)
            .entity();

        handles.blurb_label = tree
            .add_node()
            .window(
                Ab(vec2(26.0, 64.0)),
                Ab(vec2(CARD_WIDTH - 52.0, 44.0)),
                Anchor::TopLeft,
            )
            .with_text("", 15.0)
            .with_text_wrap()
            .text_left()
            .color_raw::<UiBase>(TEXT_COLOR)
            .entity();

        handles.practice_label = tree
            .add_node()
            .window(
                Ab(vec2(26.0, 112.0)),
                Ab(vec2(CARD_WIDTH - 52.0, 20.0)),
                Anchor::TopLeft,
            )
            .with_text("", 13.0)
            .with_text_wrap()
            .text_left()
            .color_raw::<UiBase>(SUCCESS)
            .entity();

        // Who is doing the pushing is a property of the board, and on a board
        // built to show a character off it is the whole point of it.
        handles.who_label = tree
            .add_node()
            .window(
                Ab(vec2(26.0, 134.0)),
                Ab(vec2(CARD_WIDTH - 52.0, 18.0)),
                Anchor::TopLeft,
            )
            .with_text("", 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        let buttons = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -16.0)),
                Ab(vec2(556.0, 30.0)),
                Anchor::BottomCenter,
            )
            .flow(FlowDirection::Horizontal, 0.0, 8.0)
            .entity();
        tree.in_parent(buttons, |tree| {
            (handles.play_button, handles.play_label) =
                widgets::build_sized(tree, "PLAY EXAMPLE", Vec2::new(104.0, 30.0), 12.0);
            handles.previous_button =
                widgets::build_sized(tree, "PREVIOUS", Vec2::new(104.0, 30.0), 12.0).0;
            handles.next_button =
                widgets::build_sized(tree, "NEXT", Vec2::new(104.0, 30.0), 12.0).0;
            handles.reset_button =
                widgets::build_sized(tree, "RESET BOARD", Vec2::new(104.0, 30.0), 12.0).0;
            handles.back_button =
                widgets::build_sized(tree, "MAIN MENU", Vec2::new(104.0, 30.0), 12.0).0;
        });
    });
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::Gallery) {
        return;
    }

    let handles = game.ui.gallery.clone();
    let keyboard = &world.res::<Input>().keyboard;
    let mut step: i32 = 0;
    if keyboard.just_pressed(KeyCode::BracketLeft) || keyboard.just_pressed(KeyCode::PageUp) {
        step -= 1;
    }
    if keyboard.just_pressed(KeyCode::BracketRight) || keyboard.just_pressed(KeyCode::PageDown) {
        step += 1;
    }
    let mut back = keyboard.just_pressed(KeyCode::Escape);
    let mut chosen = None;
    let mut play = keyboard.just_pressed(KeyCode::Enter);
    let mut reset = false;

    back = back || pad_pressed(world, gilrs::Button::East);
    // The bumpers walk the party on a board that has one, because a lesson with
    // two of you on it is a lesson about swapping between them and there is no
    // other pair of buttons for it. A lesson with one body has nobody to walk,
    // so on those they browse the rail instead.
    if game.map.party_size() == 1 {
        if pad_pressed(world, gilrs::Button::LeftTrigger) {
            step -= 1;
        }
        if pad_pressed(world, gilrs::Button::RightTrigger) {
            step += 1;
        }
    }

    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.previous_button {
            step -= 1;
        } else if entity == handles.next_button {
            step += 1;
        } else if entity == handles.play_button {
            play = true;
        } else if entity == handles.reset_button {
            reset = true;
        } else if entity == handles.back_button {
            back = true;
        } else if let Some(row) = handles.items.iter().position(|item| *item == entity) {
            chosen = handles.item_lessons.get(row).copied();
        }
    }

    if step != 0 {
        let count = lessons().len() as i32;
        let selected = (game.selected_lesson as i32 + step).rem_euclid(count);
        chosen = Some(selected as usize);
    }

    if let Some(index) = chosen {
        show(game, world, index);
    } else if reset {
        // Putting the board back means something different depending on who is
        // driving. It starts the example again, or hands the player a clean
        // board to keep working on.
        if game.playback.playing {
            replay(game, world);
        } else {
            crate::systems::input::restart(game, world);
        }
    } else if play {
        // The one button covers both halves of the same idea. Stop watching and
        // take the board, or hand it back and watch again.
        if game.playback.playing {
            playback::stop(game);
        } else {
            replay(game, world);
        }
    }
    update_labels(game, world);

    if back {
        next_state(world, Screen::Title);
    }
}

/// Starts this lesson's worked example from a clean board.
fn replay(game: &mut SokobanResources, world: &mut World) {
    let script = lesson(game.selected_lesson).demo.to_vec();
    playback::start(game, world, script, true);
}

/// Loads a lesson's board. Nothing about it is special, because it goes through
/// the same request the campaign uses.
pub fn show(game: &mut SokobanResources, world: &mut World, index: usize) {
    game.selected_lesson = index;

    let map = crate::gallery::lesson_map(index);
    crate::systems::world::build::start_map(
        game,
        world,
        MapRequest {
            map,
            origin: MapOrigin::Lesson,
        },
    );
    // A lesson opens by demonstrating itself. Nobody should have to press
    // anything to find out what the board in front of them is for.
    replay(game, world);
    // The rail and the card own the left and the bottom of the screen, so the
    // board sits up and to the right of centre rather than under them.
    game.camera.shift = Vec2::new(-1.6, 1.4);
}

pub fn update_labels(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.gallery;
    let entry = lesson(game.selected_lesson);

    ui_set_text(world, handles.title_label, &entry.name);
    ui_set_text(
        world,
        handles.counter_label,
        &format!("{} of {}", game.selected_lesson + 1, lessons().len()),
    );
    ui_set_text(world, handles.blurb_label, &entry.blurb);
    ui_set_text(
        world,
        handles.practice_label,
        &format!("TRY THIS   {}", entry.practice),
    );
    let character = game.map.character;
    ui_set_text(
        world,
        handles.who_label,
        &format!("PLAYING   {}, {}", character.label(), character.blurb()),
    );
    if let Some(color) = world.get_mut::<UiNodeColor>(handles.who_label) {
        let body = crate::palette::character_body(character);
        color.colors[UiBase::INDEX] = Some(Vec4::new(body[0], body[1], body[2], 1.0));
    }

    let watching = game.playback.playing;
    ui_set_text(
        world,
        handles.play_label,
        if watching {
            "TAKE OVER"
        } else {
            "PLAY EXAMPLE"
        },
    );
    if let Some(color) = world.get_mut::<UiNodeColor>(handles.play_button) {
        color.colors[UiBase::INDEX] = Some(if watching { PANEL_BG } else { ACCENT_DIM });
    }
    if let Some(color) = world.get_mut::<UiNodeColor>(handles.play_label) {
        color.colors[UiBase::INDEX] = Some(if watching { TEXT_DIM } else { WHITE });
    }

    for (row, item) in handles.items.iter().enumerate() {
        let selected = handles.item_lessons.get(row).copied() == Some(game.selected_lesson);
        let background = if selected { ACCENT_DIM } else { PANEL_BG };
        if let Some(color) = world.get_mut::<UiNodeColor>(*item) {
            color.colors[UiBase::INDEX] = Some(background);
        }
        if let Some(bar) = handles.item_bars.get(row)
            && let Some(color) = world.get_mut::<UiNodeColor>(*bar)
        {
            color.colors[UiBase::INDEX] = Some(if selected { ACCENT } else { PANEL_BORDER });
        }
        if let Some(label) = handles.item_labels.get(row)
            && let Some(color) = world.get_mut::<UiNodeColor>(*label)
        {
            color.colors[UiBase::INDEX] = Some(if selected { WHITE } else { TEXT_DIM });
        }
    }
}
