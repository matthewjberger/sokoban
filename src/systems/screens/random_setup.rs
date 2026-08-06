//! The random puzzle setup screen. Every control here edits one dial on the
//! recipe, so what the screen shows is the value the generator is handed, and
//! generating is a button rather than a hidden consequence of a menu choice.

use crate::ecs::{Making, RandomHandles, Screen, SokobanResources};
use crate::generator::{
    COMPLEXITY, HAZARD_STAGES, PRESET_COUNT, apply_complexity, apply_hazards, hazard_stage_of,
    preset, preset_name,
};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::systems::world::work;
use crate::theme::*;
use nightshade::prelude::*;
use nightshade::ui::widgets::world_state::{
    ui_checkbox_changed, ui_checkbox_value, ui_slider_set_value, ui_slider_value,
    ui_slider_value_changed,
};

const DIAL_HEIGHT: f32 = 34.0;
const DIAL_SIZE: Vec2 = Vec2::new(300.0, DIAL_HEIGHT);
const DIAL_GAP: f32 = 8.0;
/// The dials and the gaps between them. The column has to say how tall it
/// really is or it runs over whatever sits below it, and deriving that from the
/// count means adding a dial moves the screen rather than breaking it.
const DIALS: f32 = 10.0;
const DIAL_COLUMN_HEIGHT: f32 = DIAL_HEIGHT * DIALS + DIAL_GAP * (DIALS - 1.0);
/// Where the dials start, measured from the middle of the screen, and where the
/// buttons under them start as a result. Deriving the second from the first
/// means a dial added above never lands on a button below.
const DIAL_COLUMN_TOP: f32 = -(DIAL_COLUMN_HEIGHT * 0.5) - 24.0;
const MENU_BUTTONS: f32 = 4.0;
const MENU_GAP: f32 = 10.0;
const MENU_COLUMN_HEIGHT: f32 = MENU_BUTTON_HEIGHT * MENU_BUTTONS + MENU_GAP * (MENU_BUTTONS - 1.0);
const MENU_COLUMN_TOP: f32 = DIAL_COLUMN_TOP + DIAL_COLUMN_HEIGHT + 20.0;

pub fn build(tree: &mut UiTreeBuilder) -> RandomHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.22)
        .entity();

    let mut handles = RandomHandles {
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
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, DIAL_COLUMN_TOP - 72.0)),
                Ab(vec2(760.0, 56.0)),
                Anchor::Center,
            )
            .with_text("RANDOM PUZZLE", 42.0)
            .text_center()
            .with_text_outline(ACCENT, 1.8)
            .color_raw::<UiBase>(WHITE)
            .entity();

        handles.summary_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, DIAL_COLUMN_TOP - 28.0)),
                Ab(vec2(760.0, 22.0)),
                Anchor::Center,
            )
            .with_text("", 14.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        let dials = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, DIAL_COLUMN_TOP)),
                Ab(vec2(DIAL_SIZE.x, DIAL_COLUMN_HEIGHT)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, DIAL_GAP)
            .entity();
        tree.in_parent(dials, |tree| {
            (handles.preset_button, handles.preset_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.size_button, handles.size_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.layers_button, handles.layers_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.wings_button, handles.wings_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.crates_button, handles.crates_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.mechanics_button, handles.mechanics_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            (handles.character_button, handles.character_label) =
                widgets::build_sized(tree, "", DIAL_SIZE, 15.0);
            handles.complexity_label = tree
                .add_node()
                .flow_child(Ab(DIAL_SIZE))
                .with_text("", 15.0)
                .text_center()
                .color_raw::<UiBase>(TEXT_DIM)
                .entity();
            handles.complexity_slider = tree.add_slider(1.0, COMPLEXITY.len() as f32, 2.0);
            handles.auto_box = tree.add_checkbox("SOLVE EACH BOARD AND MOVE ON", false);
        });

        let menu = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, MENU_COLUMN_TOP)),
                Ab(vec2(MENU_BUTTON_SIZE.x, MENU_COLUMN_HEIGHT)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, MENU_GAP)
            .entity();
        tree.in_parent(menu, |tree| {
            handles.generate_button = widgets::build(tree, "GENERATE");
            handles.solve_button = widgets::build(tree, "GENERATE AND SOLVE");
            handles.endless_button = widgets::build(tree, "ENDLESS RUN");
            handles.back_button = widgets::build(tree, "BACK");
        });

        tree.add_node()
            .window(
                Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -34.0)),
                Ab(vec2(900.0, 18.0)),
                Anchor::TopCenter,
            )
            .with_text(
                "EVERY BOARD IS SOLVED BY THE SOLVER BEFORE YOU SEE IT   ·   ENTER GENERATES   ·   ESC GOES BACK",
                12.0,
            )
            .text_center()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    });

    handles
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::RandomSetup) {
        return;
    }

    let handles = game.ui.random.clone();
    // The checkbox owns the switch while this screen is up, so what it is set
    // to is what an endless run does.
    if let Some(value) = ui_checkbox_changed(world, handles.auto_box) {
        game.settings.auto_solve = value;
    }
    if let Some(value) = ui_slider_value_changed(world, handles.complexity_slider) {
        apply_complexity(&mut game.recipe, value.round() as u8);
    }
    let keyboard = &world.res::<Input>().keyboard;
    let mut generate_now =
        keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space);
    let mut back = keyboard.just_pressed(KeyCode::Escape);
    let mut solve_now = false;
    let mut endless = false;
    generate_now = generate_now || pad_pressed(world, gilrs::Button::North);
    back = back || pad_pressed(world, gilrs::Button::East);

    for entity in ui_button_clicks(world).collect::<Vec<Entity>>() {
        if entity == handles.generate_button {
            generate_now = true;
        } else if entity == handles.back_button {
            back = true;
        } else if entity == handles.preset_button {
            game.preset_index = (game.preset_index + 1) % PRESET_COUNT;
            game.recipe = preset(game.preset_index);
        } else if entity == handles.size_button {
            cycle_size(game);
        } else if entity == handles.layers_button {
            game.recipe.layers = if game.recipe.layers >= 3 {
                1
            } else {
                game.recipe.layers + 1
            };
        } else if entity == handles.wings_button {
            game.recipe.wings = if game.recipe.wings >= 2 {
                0
            } else {
                game.recipe.wings + 1
            };
        } else if entity == handles.crates_button {
            game.recipe.crates = if game.recipe.crates >= 4 {
                1
            } else {
                game.recipe.crates + 1
            };
        } else if entity == handles.mechanics_button {
            cycle_mechanics(game);
        } else if entity == handles.character_button {
            game.recipe.character = game.recipe.character.next();
        } else if entity == handles.solve_button {
            solve_now = true;
        } else if entity == handles.endless_button {
            endless = true;
        }
    }

    update_labels(game, world);

    if endless {
        work::make(game, Making::RunStart);
    } else if solve_now {
        // A run that solves itself. One board solved and stopped was a demo of
        // the recipe rather than of the generator, and the thing worth watching
        // is board after board coming out of it, so this is an endless run with
        // the switch thrown rather than a single map.
        game.settings.auto_solve = true;
        work::make(game, Making::RunStart);
    } else if generate_now {
        work::make(game, Making::Single);
    } else if back {
        next_state(world, Screen::Title);
    }
}

/// The floor size dial walks a few useful shapes rather than exposing width
/// and height separately, which keeps the screen to one control per idea.
fn cycle_size(game: &mut SokobanResources) {
    const SIZES: [(i32, i32); 7] = [
        (7, 7),
        (8, 7),
        (9, 8),
        (10, 9),
        (13, 10),
        (16, 12),
        (20, 14),
    ];
    let current = SIZES
        .iter()
        .position(|(width, height)| {
            *width == game.recipe.floor_width && *height == game.recipe.floor_height
        })
        .unwrap_or(0);
    let (width, height) = SIZES[(current + 1) % SIZES.len()];
    game.recipe.floor_width = width;
    game.recipe.floor_height = height;
}

/// The hazard dial walks the table of stages the generator publishes, so a
/// mechanic added there appears here without this knowing what it was.
fn cycle_mechanics(game: &mut SokobanResources) {
    let next = hazard_stage_of(&game.recipe) + 1;
    apply_hazards(&mut game.recipe, next);
}

pub fn update_labels(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.random;
    let recipe = &game.recipe;

    ui_set_text(
        world,
        handles.summary_label,
        &if game.random_status.is_empty() {
            "pick the shape of the puzzle, then generate".to_string()
        } else {
            game.random_status.clone()
        },
    );
    ui_set_text(
        world,
        handles.preset_label,
        &format!("PRESET  {}", preset_name(game.preset_index)),
    );
    ui_set_text(
        world,
        handles.size_label,
        &format!("FLOOR  {} x {}", recipe.floor_width, recipe.floor_height),
    );
    ui_set_text(
        world,
        handles.layers_label,
        &format!("STOREYS  {}", recipe.layers),
    );
    ui_set_text(
        world,
        handles.wings_label,
        &format!("SIDE FLOORS  {}", recipe.wings),
    );
    ui_set_text(
        world,
        handles.crates_label,
        &format!("CRATES  {}", recipe.crates),
    );
    ui_set_text(
        world,
        handles.complexity_label,
        &format!(
            "COMPLEXITY  {}  ·  {} crates, {} moves at least",
            recipe.complexity, recipe.crates, recipe.minimum_moves
        ),
    );
    if ui_slider_value(world, handles.complexity_slider).map(|value| value.round() as u8)
        != Some(recipe.complexity)
    {
        ui_slider_set_value(
            &mut world.ecs,
            handles.complexity_slider,
            recipe.complexity as f32,
        );
    }
    ui_set_text(
        world,
        handles.character_label,
        &format!("WHO  {}", recipe.character.label()),
    );
    if ui_checkbox_value(world, handles.auto_box) != Some(game.settings.auto_solve)
        && let Some(data) = world.get_mut::<UiCheckboxData>(handles.auto_box)
    {
        data.value = game.settings.auto_solve;
    }
    let mechanics = HAZARD_STAGES[hazard_stage_of(recipe)].name;
    ui_set_text(
        world,
        handles.mechanics_label,
        &format!("HAZARDS  {mechanics}"),
    );
}
