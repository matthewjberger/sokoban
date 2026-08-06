//! The screen shown when a map is solved, and the buttons that leave it.

use crate::ecs::{CompleteHandles, MapOrigin, MapRequest, Screen, SokobanResources};
use crate::maps::{load_map, map_count};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::systems::world::work;
use crate::theme::*;
use nightshade::prelude::*;

pub fn build_panel(tree: &mut UiTreeBuilder) -> CompleteHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.25)
        .entity();

    let mut title_label = Entity::default();
    let mut stats_label = Entity::default();
    let mut next_button = Entity::default();
    let mut retry_button = Entity::default();
    let mut menu_button_entity = Entity::default();

    tree.in_parent(root, |tree| {
        tree.add_node()
            .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(BACKDROP)
            .entity();

        title_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -168.0)),
                Ab(vec2(760.0, 60.0)),
                Anchor::Center,
            )
            .with_text("LEVEL SOLVED", 44.0)
            .text_center()
            .color_raw::<UiBase>(SUCCESS)
            .with_text_outline(OUTLINE, 2.0)
            .entity();

        stats_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -112.0)),
                Ab(vec2(760.0, 26.0)),
                Anchor::Center,
            )
            .with_text("", 17.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        let column = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, -70.0)),
                Ab(vec2(MENU_BUTTON_SIZE.x, 176.0)),
                Anchor::TopCenter,
            )
            .flow(FlowDirection::Vertical, 0.0, 10.0)
            .entity();
        tree.in_parent(column, |tree| {
            next_button = widgets::build(tree, "NEXT LEVEL");
            retry_button = widgets::build(tree, "REPLAY LEVEL");
            menu_button_entity = widgets::build(tree, "MAIN MENU");
        });

        tree.add_node()
            .window(
                Rl(vec2(50.0, 50.0)) + Ab(vec2(0.0, 130.0)),
                Ab(vec2(760.0, 18.0)),
                Anchor::Center,
            )
            .with_text("ENTER OR A CONTINUES   ·   B RETURNS TO THE MENU", 12.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    });

    CompleteHandles {
        root,
        title_label,
        stats_label,
        next_button,
        retry_button,
        menu_button: menu_button_entity,
    }
}

pub fn populate(game: &mut SokobanResources, world: &mut World) {
    // The line below is about to be replaced, so what was last written to it is
    // no longer what is on it.
    game.notice_shown.clear();
    let handles = &game.ui.complete;
    ui_set_text(
        world,
        handles.title_label,
        &format!("{} SOLVED", game.map.name.to_uppercase()),
    );
    ui_set_text(
        world,
        handles.stats_label,
        &format!(
            "MOVES {}   ·   PUSHES {}   ·   PAR {}",
            game.state.moves, game.state.pushes, game.map.par
        ),
    );
}

pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;

    if in_state(world, Screen::MapComplete) {
        let handles = &game.ui.complete;
        // A random board is made after this screen asks for it rather than
        // before, so the screen says what is happening in the line that was
        // reporting the board just finished.
        //
        // Only when it changes. Writing the line every frame lays the panel out
        // every frame, and a button laid out again mid animation reads as one
        // flashing at whoever just pressed it.
        if work::making(game) && game.notice != game.notice_shown {
            game.notice_shown = game.notice.clone();
            let line = game.notice.clone();
            ui_set_text(world, handles.stats_label, &line);
        }
        let mut advance = world.res::<Input>().keyboard.just_pressed(KeyCode::Enter)
            || world.res::<Input>().keyboard.just_pressed(KeyCode::Space);
        let mut retry = world.res::<Input>().keyboard.just_pressed(KeyCode::KeyR)
            || pad_pressed(world, gilrs::Button::West);
        let mut menu = pad_pressed(world, gilrs::Button::East);
        for entity in ui_button_clicks(world) {
            if entity == handles.next_button {
                advance = true;
            } else if entity == handles.retry_button {
                retry = true;
            } else if entity == handles.menu_button {
                menu = true;
            }
        }

        if advance {
            match game.origin {
                MapOrigin::Random => work::make(game, crate::ecs::Making::Single),
                // Neither an endless run nor a story room reaches this screen:
                // each hands over somewhere else the moment it is finished.
                MapOrigin::Endless | MapOrigin::Overworld | MapOrigin::Story(_) => {}
                MapOrigin::Campaign(index) => {
                    let next = (index + 1).min(map_count() - 1);
                    game.selected_map = next;
                    game.pending = Some(MapRequest {
                        map: load_map(next),
                        origin: MapOrigin::Campaign(next),
                    });
                    next_state(world, Screen::InGame);
                }
                MapOrigin::Lesson => next_state(world, Screen::Gallery),
                MapOrigin::Authored => next_state(world, Screen::Editor),
            }
        } else if retry {
            game.pending = Some(MapRequest {
                map: game.map.clone(),
                origin: game.origin,
            });
            next_state(world, Screen::InGame);
        } else if menu {
            next_state(world, Screen::Title);
        }
    }
}
