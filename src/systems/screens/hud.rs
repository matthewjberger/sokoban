use crate::ecs::{HudHandles, MapOrigin, RunSpeed, Screen, SokobanResources};
use crate::maps::map_count;
use crate::rules::{beam_field, carried_by, goals_covered, seated_at};
use crate::schema::{GemColor, WinCondition, map_layers};
use crate::systems::input::pad_pressed;
use crate::systems::screens::widgets;
use crate::theme::*;
use nightshade::prelude::*;

const SPEED_BUTTON_SIZE: Vec2 = Vec2::new(130.0, 28.0);

/// What the controls line says on a board being played, and on one being
/// watched. A run is mostly not played, so the line that tells you which keys
/// move a body is the wrong line to be reading.
const PLAYING: &str =
    "WASD / D-PAD MOVE   ·   Q E RIDE   ·   Z UNDO   ·   R RESTART   ·   ESC PAUSE";
/// The same line on a board with gems on it, where one more key matters.
const CARRYING: &str = "WASD / D-PAD MOVE   ·   SPACE LIFT OR SEAT   ·   Q E RIDE   ·   Z UNDO   ·   R RESTART   ·   ESC PAUSE";
const WATCHING: &str =
    "WASD / D-PAD TAKE OVER   ·   SPEED OR BACK CHANGES THE PACE   ·   ESC PAUSE";

pub fn build(tree: &mut UiTreeBuilder) -> HudHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .entity();

    let mut handles = HudHandles {
        root,
        ..Default::default()
    };

    tree.in_parent(root, |tree| {
        let banner = tree
            .add_node()
            .window(Ab(vec2(20.0, 18.0)), Ab(vec2(520.0, 74.0)), Anchor::TopLeft)
            .with_rect(4.0, 1.0, PANEL_BORDER)
            .color_raw::<UiBase>(PANEL_BG_DEEP)
            .entity();
        tree.in_parent(banner, |tree| {
            handles.map_label = tree
                .add_node()
                .window(Ab(vec2(18.0, 12.0)), Ab(vec2(484.0, 28.0)), Anchor::TopLeft)
                .with_text("MAP 1  ·  FIRST PUSH", 21.0)
                .text_left()
                .color_raw::<UiBase>(ACCENT)
                .with_text_outline(OUTLINE, 1.6)
                .entity();
            handles.hint_label = tree
                .add_node()
                .window(Ab(vec2(18.0, 42.0)), Ab(vec2(484.0, 22.0)), Anchor::TopLeft)
                .with_text("", 13.0)
                .text_left()
                .color_raw::<UiBase>(TEXT_FAINT)
                .entity();
        });

        let counters = tree
            .add_node()
            .window(
                Rl(vec2(100.0, 0.0)) + Ab(vec2(-20.0, 18.0)),
                Ab(vec2(260.0, 128.0)),
                Anchor::TopRight,
            )
            .with_rect(4.0, 1.0, PANEL_BORDER)
            .color_raw::<UiBase>(PANEL_BG_DEEP)
            .entity();
        tree.in_parent(counters, |tree| {
            handles.goal_label = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 12.0)),
                    Ab(vec2(224.0, 26.0)),
                    Anchor::TopRight,
                )
                .with_text("CRATES  0 / 1", 19.0)
                .text_right()
                .color_raw::<UiBase>(SUCCESS)
                .entity();
            handles.move_label = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 42.0)),
                    Ab(vec2(224.0, 20.0)),
                    Anchor::TopRight,
                )
                .with_text("MOVES  0", 14.0)
                .text_right()
                .color_raw::<UiBase>(TEXT_DIM)
                .entity();
            handles.push_label = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 62.0)),
                    Ab(vec2(224.0, 20.0)),
                    Anchor::TopRight,
                )
                .with_text("PUSHES  0", 14.0)
                .text_right()
                .color_raw::<UiBase>(TEXT_DIM)
                .entity();
            handles.par_label = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 82.0)),
                    Ab(vec2(224.0, 20.0)),
                    Anchor::TopRight,
                )
                .with_text("PAR  1", 14.0)
                .text_right()
                .color_raw::<UiBase>(TEXT_FAINT)
                .entity();
            handles.layer_label = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 102.0)),
                    Ab(vec2(224.0, 20.0)),
                    Anchor::TopRight,
                )
                .with_text("", 14.0)
                .text_right()
                .color_raw::<UiBase>(ACCENT)
                .entity();
        });

        handles.gem_label = tree
            .add_node()
            .window(
                Rl(vec2(100.0, 0.0)) + Ab(vec2(-20.0, 156.0)),
                Ab(vec2(420.0, 20.0)),
                Anchor::TopRight,
            )
            .with_text("", 14.0)
            .text_right()
            .color_raw::<UiBase>(ACCENT)
            .with_text_outline(OUTLINE, 1.2)
            .entity();

        // A run mostly runs itself, so the one control worth having on a board
        // in play is how fast to watch it. It sits under the banner rather than
        // among the counters, because it is something to press rather than
        // something to read.
        let speed = tree
            .add_node()
            .window(
                Ab(vec2(20.0, 100.0)),
                Ab(SPEED_BUTTON_SIZE),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Vertical, 0.0, 0.0)
            .entity();
        tree.in_parent(speed, |tree| {
            (handles.speed_button, handles.speed_label) =
                widgets::build_sized(tree, RunSpeed::default().label(), SPEED_BUTTON_SIZE, 14.0);
        });

        handles.fps_label = tree
            .add_node()
            .window(
                Rl(vec2(100.0, 100.0)) + Ab(vec2(-20.0, -16.0)),
                Ab(vec2(140.0, 18.0)),
                Anchor::BottomRight,
            )
            .with_text("0 fps", 12.0)
            .text_right()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();

        handles.footer_label = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -18.0)),
                Ab(vec2(1000.0, 18.0)),
                Anchor::BottomCenter,
            )
            .with_text(PLAYING, 12.0)
            .text_center()
            .color_raw::<UiBase>(TEXT_FAINT)
            .with_text_outline(OUTLINE, 1.2)
            .entity();
    });

    handles
}

/// Keeps an authored name inside the banner. The field it comes from accepts
/// anything, and the banner is a fixed width.
fn shorten(name: &str) -> String {
    const LIMIT: usize = 22;
    let upper = name.to_uppercase();
    if upper.chars().count() <= LIMIT {
        return upper;
    }
    upper.chars().take(LIMIT - 2).collect::<String>() + ".."
}

/// The one thing on a board in play that can be pressed. A run watched at the
/// speed a person plays at is a clock being watched, so the control that says
/// otherwise belongs on the board rather than behind the pause screen.
pub fn handle_input(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    if !in_state(world, Screen::InGame) || !matches!(game.origin, MapOrigin::Endless) {
        return;
    }
    let button = game.ui.hud.speed_button;
    // A pad cannot reach the button, because the board owns the stick while it
    // is up and the retained interface takes no focus here. The pad gets the same
    // control on a spare face button instead.
    if ui_button_clicks(world).any(|entity| entity == button)
        || pad_pressed(world, gilrs::Button::Select)
    {
        game.run_speed = game.run_speed.next();
    }
}

/// What is in your hands and what the light you are standing in is lending you.
/// Both are things the board is already saying in colour, and this is the line
/// that says them in words, which is what turns a rule somebody noticed into a
/// rule they can rely on.
fn carrying(game: &SokobanResources) -> String {
    if game.map.gems.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    if let Some(index) = carried_by(&game.state, game.state.active)
        && let Some(gem) = game.map.gems.get(index)
    {
        parts.push(format!("CARRYING  {}", gem.color.label()));
    }
    if game.map.rules.gem_light_grants_powers {
        let field = beam_field(&game.map, &game.state);
        let standing = game.state.player();
        for color in GemColor::ALL {
            if field
                .aura
                .iter()
                .any(|(at, tint)| *at == standing && *tint == color)
            {
                parts.push(format!("{}  ·  {}", color.label(), color.blurb()));
            }
        }
    }
    parts.join("   ·   ")
}

pub fn update(game: Res<SokobanResources>, world: &mut World) {
    let handles = &game.ui.hud;
    let fps = world.res::<Time>().frames_per_second;

    // The banner stays up over the pause and completion screens, and a control
    // that cannot be pressed has no business being under a panel that covers
    // it. It shows exactly where it works.
    let running = matches!(game.origin, MapOrigin::Endless) && in_state(world, Screen::InGame);
    ui_set_visible(world, handles.speed_button, running);
    ui_set_text(world, handles.speed_label, game.run_speed.label());
    ui_set_text(
        world,
        handles.footer_label,
        match (running, game.map.gems.is_empty()) {
            (true, _) => WATCHING,
            (false, true) => PLAYING,
            (false, false) => CARRYING,
        },
    );

    ui_set_text(
        world,
        handles.map_label,
        &match game.origin {
            MapOrigin::Campaign(index) => format!(
                "MAP {} / {}  ·  {}",
                index + 1,
                map_count(),
                shorten(&game.map.name)
            ),
            MapOrigin::Random => format!("RANDOM  ·  {}", shorten(&game.map.name)),
            MapOrigin::Overworld => format!("THE DEPOT  ·  {}", shorten(&game.map.name)),
            MapOrigin::Story(level) => format!(
                "ROOM {} / {}  ·  {}",
                level + 1,
                map_count(),
                shorten(&game.map.name)
            ),
            MapOrigin::Endless => format!(
                "ENDLESS  ·  BOARD {}  ·  {}",
                game.endless_cleared + 1,
                shorten(&game.map.name)
            ),
            MapOrigin::Lesson => format!("LESSON  ·  {}", shorten(&game.map.name)),
            MapOrigin::Authored => format!("CUSTOM  ·  {}", shorten(&game.map.name)),
        },
    );
    let hint = if game.notice.is_empty() {
        &game.map.hint
    } else {
        &game.notice
    };
    ui_set_text(world, handles.hint_label, hint);
    // What the board is counting depends on what it is asking for, and a board
    // won by seating gems is not counting crates.
    ui_set_text(
        world,
        handles.goal_label,
        &if game.map.rules.win == WinCondition::SocketsFilled {
            format!(
                "GEMS  {} / {}",
                game.map
                    .sockets
                    .iter()
                    .filter(|(at, _)| seated_at(&game.state, *at).is_some())
                    .count(),
                game.map.sockets.len()
            )
        } else {
            format!(
                "CRATES  {} / {}",
                goals_covered(&game.map, &game.state),
                game.map.goals.len()
            )
        },
    );
    ui_set_text(world, handles.gem_label, &carrying(&game));
    ui_set_text(
        world,
        handles.move_label,
        &format!("MOVES  {}", game.state.moves),
    );
    ui_set_text(
        world,
        handles.push_label,
        &format!("PUSHES  {}", game.state.pushes),
    );
    ui_set_text(world, handles.par_label, &format!("PAR  {}", game.map.par));

    let storeys = map_layers(&game.map).len();
    ui_set_text(
        world,
        handles.layer_label,
        &if storeys > 1 {
            format!("STOREY  {} / {}", game.state.player().layer + 1, storeys)
        } else {
            String::new()
        },
    );
    ui_set_text(world, handles.fps_label, &format!("{fps:.0} fps"));
}
