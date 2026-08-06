//! The board's jobs, on screen. What is asked for and what waits on what comes
//! from the board itself, so this only has to draw a list and tick it, and a
//! board with a switch feeding a door feeding a marker reads as three lines with
//! the last two indented under the first.
//!
//! The rows are built once and reused. A board with more jobs than there are
//! rows shows the first of them, which is a display running out of room rather
//! than the board being misread.

use crate::ecs::{ObjectiveHandles, Screen, SokobanResources};
use crate::objectives::{Job, done};
use crate::theme::*;
use nightshade::prelude::*;

/// How many jobs the panel will show. The busiest board in the campaign asks for
/// six, so this is that with room over.
const ROWS: usize = 12;
const ROW_HEIGHT: f32 = 26.0;
const PANEL_WIDTH: f32 = 268.0;
/// What a job that waits on another is prefixed with, which is how a list shows
/// a graph without drawing lines.
const INDENT: &str = "  ";

pub fn build(tree: &mut UiTreeBuilder) -> ObjectiveHandles {
    let mut handles = ObjectiveHandles::default();

    let root = tree
        .add_node()
        .window(
            Rl(vec2(100.0, 0.0)) + Ab(vec2(-20.0, 108.0)),
            Ab(vec2(PANEL_WIDTH, ROW_HEIGHT * ROWS as f32 + 46.0)),
            Anchor::TopRight,
        )
        .with_rect(4.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .with_visible(false)
        .entity();
    handles.root = root;

    tree.in_parent(root, |tree| {
        tree.add_node()
            .window(
                Ab(vec2(16.0, 11.0)),
                Ab(vec2(PANEL_WIDTH - 32.0, 20.0)),
                Anchor::TopLeft,
            )
            .with_text("WHAT THIS BOARD WANTS", 13.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        for index in 0..ROWS {
            let top = 38.0 + index as f32 * ROW_HEIGHT;
            let mark = tree
                .add_node()
                .window(Ab(vec2(16.0, top)), Ab(vec2(18.0, 20.0)), Anchor::TopLeft)
                .with_text("", 14.0)
                .text_left()
                .color_raw::<UiBase>(TEXT_FAINT)
                .entity();
            let label = tree
                .add_node()
                .window(
                    Ab(vec2(36.0, top)),
                    Ab(vec2(PANEL_WIDTH - 52.0, 20.0)),
                    Anchor::TopLeft,
                )
                .with_text("", 13.0)
                .text_left()
                .color_raw::<UiBase>(TEXT_COLOR)
                .entity();
            handles.marks.push(mark);
            handles.labels.push(label);
        }
    });

    handles
}

/// Redraws the list against the position on the board. Everything shown is read
/// from the state rather than remembered, so undo and restart are already
/// handled by having nothing to unwind.
pub fn update(mut game: ResMut<SokobanResources>, world: &mut World) {
    let game = &mut *game;
    let handles = game.ui.objectives.clone();
    if !world.is_alive(handles.root) {
        return;
    }

    let showing = game.settings.show_objectives
        && (in_state(world, Screen::InGame) || in_state(world, Screen::Story))
        && !game.objectives.nodes.is_empty();
    if world
        .get::<Visibility>(handles.root)
        .is_some_and(|visibility| visibility.visible != showing)
    {
        world.set(handles.root, Visibility { visible: showing });
    }
    if !showing {
        return;
    }

    let finished = done(&game.map, &game.state, &game.objectives);
    for index in 0..handles.labels.len() {
        let Some(node) = game.objectives.nodes.get(index) else {
            ui_set_text(world, handles.labels[index], "");
            ui_set_text(world, handles.marks[index], "");
            continue;
        };
        let complete = finished.get(index).copied().unwrap_or(false);
        // A job that waits on something is set in under it, which is the whole
        // of the graph a list can show without drawing lines.
        let indent = if node.needs.is_empty() { "" } else { INDENT };
        ui_set_text(
            world,
            handles.marks[index],
            if complete { "X" } else { "-" },
        );
        ui_set_text(
            world,
            handles.labels[index],
            &format!("{indent}{}", node.job.label()),
        );

        let tone = match (complete, node.job) {
            (true, _) => SUCCESS,
            (false, Job::Deliver { .. }) => TEXT_COLOR,
            (false, _) => TEXT_FAINT,
        };
        if let Some(color) = world.get_mut::<UiNodeColor>(handles.labels[index]) {
            color.colors[UiBase::INDEX] = Some(tone);
        }
        if let Some(color) = world.get_mut::<UiNodeColor>(handles.marks[index]) {
            color.colors[UiBase::INDEX] = Some(if complete { SUCCESS } else { TEXT_FAINT });
        }
    }
}
