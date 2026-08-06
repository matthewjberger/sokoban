use crate::ecs::{Brush, EditorHandles, EditorOverlay, SokobanResources};
use crate::schema::{RULE_SWITCHES, map_floor_index, map_slot_for, map_tile, summarize};
use crate::systems::screens::widgets;
use crate::theme::*;
use nightshade::prelude::*;
use nightshade::ui::widgets::world_state::{ui_checkbox_value, widget};

const BRUSH_SIZE: Vec2 = Vec2::new(150.0, 27.0);
const ACTION_SIZE: Vec2 = Vec2::new(170.0, 27.0);
const SMALL: Vec2 = Vec2::new(38.0, 26.0);
/// What the columns leave for the header above them and the status bar below,
/// so they shrink with the window instead of growing into either.
const COLUMN_MARGIN: f32 = 190.0;

pub fn build(tree: &mut UiTreeBuilder) -> EditorHandles {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .entity();

    let mut handles = EditorHandles {
        root,
        ..Default::default()
    };

    tree.in_parent(root, |tree| {
        build_brush_column(tree, &mut handles);
        build_map_column(tree, &mut handles);
        build_header(tree, &mut handles);
        build_footer(tree, &mut handles);
        build_rules_panel(tree, &mut handles);
        build_schema_panel(tree, &mut handles);
        build_confirm_panel(tree, &mut handles);
    });

    handles
}

fn build_brush_column(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let panel = tree
        .add_node()
        .window(
            Ab(vec2(18.0, 92.0)),
            Ab(vec2(174.0, 424.0)),
            Anchor::TopLeft,
        )
        .with_rect(5.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .entity();

    tree.in_parent(panel, |tree| {
        tree.add_node()
            .window(Ab(vec2(12.0, 8.0)), Ab(vec2(150.0, 18.0)), Anchor::TopLeft)
            .with_text("BRUSH", 13.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        let column = tree
            .add_node()
            .window(
                Ab(vec2(12.0, 28.0)),
                Rl(vec2(0.0, 100.0)) + Ab(vec2(BRUSH_SIZE.x, -36.0)),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Vertical, 0.0, 3.0)
            .entity();
        tree.in_parent(column, |tree| {
            let scroll = tree.add_scroll_area_fill(0.0, 3.0);
            let content = widget::<UiScrollAreaData>(tree.world_mut(), scroll)
                .map(|data| data.content_entity)
                .unwrap_or(scroll);
            tree.in_parent(content, |tree| {
                for brush in Brush::ALL {
                    let (button, _) = widgets::build_sized(tree, brush.label(), BRUSH_SIZE, 12.0);
                    handles.brush_buttons.push(button);
                }
                (handles.group_button, handles.group_label) =
                    widgets::build_sized(tree, "GROUP 1", BRUSH_SIZE, 12.0);
            });
        });
    });
}

fn build_map_column(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let panel = tree
        .add_node()
        .window(
            Rl(vec2(100.0, 0.0)) + Ab(vec2(-18.0, 92.0)),
            Rl(vec2(0.0, 100.0)) + Ab(vec2(194.0, -COLUMN_MARGIN)),
            Anchor::TopRight,
        )
        .with_rect(5.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .entity();

    tree.in_parent(panel, |tree| {
        tree.add_node()
            .window(Ab(vec2(12.0, 8.0)), Ab(vec2(170.0, 18.0)), Anchor::TopLeft)
            .with_text("MAP", 13.0)
            .text_left()
            .color_raw::<UiBase>(ACCENT)
            .entity();

        handles.size_label = tree
            .add_node()
            .window(Ab(vec2(12.0, 26.0)), Ab(vec2(170.0, 46.0)), Anchor::TopLeft)
            .with_text("", 11.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        let size_row = tree
            .add_node()
            .window(Ab(vec2(12.0, 72.0)), Ab(vec2(170.0, 28.0)), Anchor::TopLeft)
            .flow(FlowDirection::Horizontal, 0.0, 4.0)
            .entity();
        tree.in_parent(size_row, |tree| {
            handles.width_minus = widgets::build_sized(tree, "W-", SMALL, 12.0).0;
            handles.width_plus = widgets::build_sized(tree, "W+", SMALL, 12.0).0;
            handles.height_minus = widgets::build_sized(tree, "H-", SMALL, 12.0).0;
            handles.height_plus = widgets::build_sized(tree, "H+", SMALL, 12.0).0;
        });

        handles.layer_label = tree
            .add_node()
            .window(
                Ab(vec2(12.0, 104.0)),
                Ab(vec2(170.0, 18.0)),
                Anchor::TopLeft,
            )
            .with_text("", 11.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();

        let layer_row = tree
            .add_node()
            .window(
                Ab(vec2(12.0, 122.0)),
                Ab(vec2(170.0, 28.0)),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Horizontal, 0.0, 4.0)
            .entity();
        tree.in_parent(layer_row, |tree| {
            handles.layer_down = widgets::build_sized(tree, "DN", SMALL, 12.0).0;
            handles.layer_up = widgets::build_sized(tree, "UP", SMALL, 12.0).0;
            handles.add_floor = widgets::build_sized(tree, "+FL", SMALL, 12.0).0;
            handles.remove_floor = widgets::build_sized(tree, "-FL", SMALL, 12.0).0;
        });

        let slot_row = tree
            .add_node()
            .window(
                Ab(vec2(12.0, 156.0)),
                Ab(vec2(170.0, 28.0)),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Horizontal, 0.0, 4.0)
            .entity();
        tree.in_parent(slot_row, |tree| {
            handles.previous_slot = widgets::build_sized(tree, "<", Vec2::new(28.0, 26.0), 12.0).0;
            handles.slot_label = tree
                .add_node()
                .flow_child(Ab(vec2(106.0, 26.0)))
                .with_rect(3.0, 1.0, PANEL_BORDER)
                .color_raw::<UiBase>(PANEL_BG)
                .with_text("no saves", 12.0)
                .text_center()
                .color_raw::<UiBase>(TEXT_DIM)
                .entity();
            handles.next_slot = widgets::build_sized(tree, ">", Vec2::new(28.0, 26.0), 12.0).0;
        });

        let actions = tree
            .add_node()
            .window(
                Ab(vec2(12.0, 222.0)),
                Rl(vec2(0.0, 100.0)) + Ab(vec2(ACTION_SIZE.x, -230.0)),
                Anchor::TopLeft,
            )
            .flow(FlowDirection::Vertical, 0.0, 5.0)
            .entity();
        let actions = {
            let scroll = tree.in_parent(actions, |tree| tree.add_scroll_area_fill(0.0, 5.0));
            widget::<UiScrollAreaData>(tree.world_mut(), scroll)
                .map(|data| data.content_entity)
                .unwrap_or(scroll)
        };
        tree.in_parent(actions, |tree| {
            handles.new_button = widgets::build_sized(tree, "NEW", ACTION_SIZE, 12.0).0;
            handles.randomize_button = widgets::build_sized(tree, "RANDOMIZE", ACTION_SIZE, 12.0).0;
            (handles.character_button, handles.character_label) =
                widgets::build_sized(tree, "", ACTION_SIZE, 12.0);
            (handles.skin_button, handles.skin_label) =
                widgets::build_sized(tree, "", ACTION_SIZE, 12.0);
            (handles.win_button, handles.win_label) =
                widgets::build_sized(tree, "", ACTION_SIZE, 12.0);
            handles.rules_button = widgets::build_sized(tree, "RULES", ACTION_SIZE, 12.0).0;
            handles.schema_button = widgets::build_sized(tree, "SCHEMA", ACTION_SIZE, 12.0).0;
            handles.analyze_button = widgets::build_sized(tree, "ANALYZE", ACTION_SIZE, 12.0).0;
            handles.test_button = widgets::build_sized(tree, "TEST PLAY", ACTION_SIZE, 12.0).0;
            handles.save_button = widgets::build_sized(tree, "SAVE", ACTION_SIZE, 12.0).0;
            handles.load_button = widgets::build_sized(tree, "LOAD SLOT", ACTION_SIZE, 12.0).0;
            handles.copy_button = widgets::build_sized(tree, "COPY JSON", ACTION_SIZE, 12.0).0;
            handles.back_button = widgets::build_sized(tree, "MAIN MENU", ACTION_SIZE, 12.0).0;
        });
    });
}

fn build_header(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let header = tree
        .add_node()
        .window(
            Rl(vec2(50.0, 0.0)) + Ab(vec2(0.0, 16.0)),
            Ab(vec2(520.0, 82.0)),
            Anchor::TopCenter,
        )
        .with_rect(5.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .entity();

    tree.in_parent(header, |tree| {
        tree.add_node()
            .window(Ab(vec2(14.0, 15.0)), Ab(vec2(46.0, 18.0)), Anchor::TopLeft)
            .with_text("NAME", 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
        let name_row = tree
            .add_node()
            .window(Ab(vec2(64.0, 10.0)), Ab(vec2(440.0, 28.0)), Anchor::TopLeft)
            .flow(FlowDirection::Horizontal, 0.0, 0.0)
            .entity();
        tree.in_parent(name_row, |tree| {
            handles.name_input = tree.add_text_input_with_value("map name", "Untitled");
        });

        tree.add_node()
            .window(Ab(vec2(14.0, 51.0)), Ab(vec2(46.0, 18.0)), Anchor::TopLeft)
            .with_text("HINT", 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
        let hint_row = tree
            .add_node()
            .window(Ab(vec2(64.0, 46.0)), Ab(vec2(440.0, 28.0)), Anchor::TopLeft)
            .flow(FlowDirection::Horizontal, 0.0, 0.0)
            .entity();
        tree.in_parent(hint_row, |tree| {
            handles.hint_input = tree.add_text_input_with_value("shown while playing", "");
        });
    });
}

fn build_footer(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let footer = tree
        .add_node()
        .window(
            Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -18.0)),
            Ab(vec2(940.0, 74.0)),
            Anchor::BottomCenter,
        )
        .with_rect(5.0, 1.0, PANEL_BORDER)
        .color_raw::<UiBase>(PANEL_BG_DEEP)
        .entity();

    tree.in_parent(footer, |tree| {
        handles.status_label = tree
            .add_node()
            .window(Ab(vec2(16.0, 8.0)), Ab(vec2(900.0, 22.0)), Anchor::TopLeft)
            .with_text("", 15.0)
            .text_left()
            .color_raw::<UiBase>(SUCCESS)
            .entity();
        handles.issue_label = tree
            .add_node()
            .window(Ab(vec2(16.0, 30.0)), Ab(vec2(900.0, 20.0)), Anchor::TopLeft)
            .with_text("", 12.0)
            .text_left()
            .color_raw::<UiBase>(TEXT_DIM)
            .entity();
        tree.add_node()
            .window(Ab(vec2(16.0, 50.0)), Ab(vec2(900.0, 18.0)), Anchor::TopLeft)
            .with_text(
                "LEFT DRAG PAINTS   ·   RIGHT DRAG ERASES   ·   1-0 PICK BRUSH   ·   PAGE UP DOWN CHANGE STOREY   ·   V ANALYZE   ·   ENTER TEST   ·   ESC MENU",
                11.0,
            )
            .text_left()
            .color_raw::<UiBase>(TEXT_FAINT)
            .entity();
    });
}

/// How many lines the schema view can show before it starts leaving some out.
/// The panel scrolls, so this only has to cover the largest map worth reading.
const SCHEMA_LINES: usize = 72;

/// A panel that covers the board while it is open, for the parts of a map that
/// are not painted onto it.
fn build_overlay(tree: &mut UiTreeBuilder, title: &str, height: f32) -> (Entity, Entity, Entity) {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.16)
        .entity();
    let mut body = Entity::default();
    let mut close = Entity::default();

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
                Ab(vec2(560.0, height)),
                Anchor::Center,
            )
            .with_rect(6.0, 1.0, PANEL_BORDER)
            .color_raw::<UiBase>(PANEL_BG_DEEP)
            .entity();
        tree.in_parent(panel, |tree| {
            tree.add_node()
                .window(Ab(vec2(20.0, 14.0)), Ab(vec2(400.0, 24.0)), Anchor::TopLeft)
                .with_text(title, 20.0)
                .text_left()
                .color_raw::<UiBase>(ACCENT)
                .entity();

            let closer = tree
                .add_node()
                .window(
                    Rl(vec2(100.0, 0.0)) + Ab(vec2(-16.0, 12.0)),
                    Ab(vec2(110.0, 28.0)),
                    Anchor::TopRight,
                )
                .flow(FlowDirection::Horizontal, 0.0, 0.0)
                .entity();
            tree.in_parent(closer, |tree| {
                close = widgets::build_sized(tree, "CLOSE", Vec2::new(110.0, 28.0), 12.0).0;
            });

            let frame = tree
                .add_node()
                .window(
                    Ab(vec2(20.0, 50.0)),
                    Ab(vec2(520.0, height - 70.0)),
                    Anchor::TopLeft,
                )
                .flow(FlowDirection::Vertical, 0.0, 0.0)
                .entity();
            body = tree.in_parent(frame, |tree| {
                let scroll = tree.add_scroll_area_fill(0.0, 4.0);
                widget::<UiScrollAreaData>(tree.world_mut(), scroll)
                    .map(|data| data.content_entity)
                    .unwrap_or(scroll)
            });
        });
    });

    (root, body, close)
}

fn build_rules_panel(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let (root, body, close) = build_overlay(tree, "RULES", 540.0);
    handles.rules_panel = root;
    handles.rules_close = close;
    tree.in_parent(body, |tree| {
        for (label, _) in RULE_SWITCHES {
            handles.rule_boxes.push(tree.add_checkbox(label, true));
        }
    });
}

/// The one question the editor asks before it does something that cannot be
/// undone. It covers the board like the other panels, so the answer is the only
/// thing to give.
fn build_confirm_panel(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let root = tree
        .add_node()
        .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
        .with_visible(false)
        .with_intro(UiAnimationType::Fade, 0.14)
        .entity();
    handles.confirm_panel = root;

    tree.in_parent(root, |tree| {
        tree.add_node()
            .boundary(Rl(vec2(0.0, 0.0)), Rl(vec2(100.0, 100.0)))
            .with_rect(0.0, 0.0, TRANSPARENT)
            .color_raw::<UiBase>(BACKDROP)
            .entity();

        let frame = tree
            .add_node()
            .window(
                Rl(vec2(50.0, 50.0)),
                Ab(vec2(460.0, 190.0)),
                Anchor::Center,
            )
            .with_rect(7.0, 1.0, PANEL_BORDER)
            .color_raw::<UiBase>(PANEL_BG_DEEP)
            .with_intro(UiAnimationType::SlideUp, 0.16)
            .entity();

        tree.in_parent(frame, |tree| {
            tree.add_node()
                .window(Ab(vec2(28.0, 26.0)), Ab(vec2(404.0, 30.0)), Anchor::TopLeft)
                .with_text("REPLACE THIS MAP", 22.0)
                .text_left()
                .color_raw::<UiBase>(ACCENT)
                .entity();

            tree.add_node()
                .window(Ab(vec2(28.0, 66.0)), Ab(vec2(404.0, 44.0)), Anchor::TopLeft)
                .with_text(
                    "A generated map will take the place of the one you have open. Nothing here is saved yet.",
                    13.0,
                )
                .with_text_wrap()
                .text_left()
                .color_raw::<UiBase>(TEXT_DIM)
                .entity();

            let row = tree
                .add_node()
                .window(
                    Rl(vec2(50.0, 100.0)) + Ab(vec2(0.0, -22.0)),
                    Ab(vec2(316.0, 32.0)),
                    Anchor::BottomCenter,
                )
                .flow(FlowDirection::Horizontal, 0.0, 12.0)
                .entity();
            tree.in_parent(row, |tree| {
                handles.confirm_yes =
                    widgets::build_sized(tree, "REPLACE IT", Vec2::new(152.0, 32.0), 13.0).0;
                handles.confirm_no =
                    widgets::build_sized(tree, "KEEP MINE", Vec2::new(152.0, 32.0), 13.0).0;
            });
        });
    });
}

fn build_schema_panel(tree: &mut UiTreeBuilder, handles: &mut EditorHandles) {
    let (root, body, close) = build_overlay(tree, "SCHEMA", 520.0);
    handles.schema_panel = root;
    handles.schema_close = close;
    tree.in_parent(body, |tree| {
        for _ in 0..SCHEMA_LINES {
            handles.schema_lines.push(
                tree.add_node()
                    .flow_child(Ab(vec2(500.0, 17.0)))
                    .with_text("", 12.0)
                    .text_left()
                    .color_raw::<UiBase>(TEXT_DIM)
                    .entity(),
            );
        }
    });
}

pub fn update(game: Res<SokobanResources>, world: &mut World) {
    let handles = &game.ui.editor;
    let editor = &game.editor;
    let map = &editor.map;

    ui_set_text(
        world,
        handles.size_label,
        &format!(
            "floors {} x {}   ·   {} placed\n{} crates   ·   {} goals",
            map.floor_width,
            map.floor_height,
            map.floors.len(),
            map.crates.len(),
            map.goals.len()
        ),
    );

    let slot = map_slot_for(map, editor.cursor).0;
    let occupied = map_floor_index(map, slot).is_some();
    ui_set_text(
        world,
        handles.layer_label,
        &format!(
            "storey {}  ·  slot {},{}  ·  {}",
            editor.cursor.layer,
            slot.column,
            slot.row,
            if occupied {
                map_tile(map, editor.cursor).label()
            } else {
                "EMPTY"
            }
        ),
    );
    ui_set_text(
        world,
        handles.group_label,
        &format!("GROUP {}", editor.group + 1),
    );
    ui_set_text(
        world,
        handles.character_label,
        &format!("WHO  {}", map.character.label()),
    );
    ui_set_text(
        world,
        handles.skin_label,
        &format!("SKIN  {}", map.skin.label()),
    );
    ui_set_text(
        world,
        handles.win_label,
        &format!("WIN  {}", map.rules.win.label()),
    );
    let save_slot = editor
        .slots
        .get(editor.slot_index)
        .cloned()
        .unwrap_or_else(|| "no saves".to_string());
    ui_set_text(world, handles.slot_label, &save_slot);
    ui_set_text(world, handles.status_label, &editor.status);
    ui_set_text(world, handles.issue_label, &editor.issues);

    for (index, button) in handles.brush_buttons.iter().enumerate() {
        let selected = Brush::ALL.get(index).copied() == Some(editor.brush);
        let color = if selected { ACCENT_DIM } else { PANEL_BG };
        if let Some(node_color) = world.get_mut::<UiNodeColor>(*button) {
            node_color.colors[UiBase::INDEX] = Some(color);
        }
    }

    update_overlays(&game, world);
}

/// The detail panels follow the editor's state rather than holding their own,
/// so what they show is always the map in hand.
fn update_overlays(game: &SokobanResources, world: &mut World) {
    let handles = &game.ui.editor;
    let overlay = game.editor.overlay;
    ui_set_visible(world, handles.rules_panel, overlay == EditorOverlay::Rules);
    ui_set_visible(
        world,
        handles.schema_panel,
        overlay == EditorOverlay::Schema,
    );
    ui_set_visible(
        world,
        handles.confirm_panel,
        overlay == EditorOverlay::Confirm,
    );

    if overlay == EditorOverlay::Rules {
        let mut rules = game.editor.map.rules;
        for (index, entity) in handles.rule_boxes.iter().enumerate() {
            let Some((_, access)) = RULE_SWITCHES.get(index) else {
                continue;
            };
            let value = *access(&mut rules);
            if ui_checkbox_value(world, *entity) != Some(value)
                && let Some(data) = world.get_mut::<UiCheckboxData>(*entity)
            {
                data.value = value;
            }
        }
    }

    if overlay == EditorOverlay::Schema {
        let lines = summarize(&game.editor.map);
        for (index, entity) in handles.schema_lines.iter().enumerate() {
            match lines.get(index) {
                Some(line) => {
                    ui_set_text(world, *entity, &line.text);
                    let color = if line.heading { ACCENT } else { TEXT_DIM };
                    if let Some(node_color) = world.get_mut::<UiNodeColor>(*entity) {
                        node_color.colors[UiBase::INDEX] = Some(color);
                    }
                }
                None => ui_set_text(world, *entity, ""),
            }
        }
    }
}
