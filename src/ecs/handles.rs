//! Handles onto the retained interface. A screen builds its tree once and
//! keeps the entities it will speak to later. This is where those live so the
//! resource struct stays about the game rather than about the interface.

use nightshade::prelude::Entity;

/// The level picker: one button per board, in campaign order.
#[derive(Clone, Debug, Default)]
pub struct LevelSelectHandles {
    pub root: Entity,
    pub items: Vec<Entity>,
    pub back_button: Entity,
}

#[derive(Clone, Default)]
pub struct TitleHandles {
    pub root: Entity,
    /// The two menus. One is up at a time and the other is hidden, so which one
    /// is showing is the whole of the state this screen carries.
    pub root_column: Entity,
    pub play_column: Entity,
    pub play_button: Entity,
    pub gallery_button: Entity,
    pub editor_button: Entity,
    pub settings_button: Entity,
    pub quit_button: Entity,
    pub story_button: Entity,
    pub campaign_button: Entity,
    pub levels_button: Entity,
    pub random_button: Entity,
    pub endless_button: Entity,
    pub play_back_button: Entity,
    /// What the generator is doing, for the wait between pressing the button
    /// and a board arriving. There is no screen in between any more, so this is
    /// where that wait is shown.
    pub status_label: Entity,
}

#[derive(Default, Clone)]
pub struct SettingsHandles {
    pub root: Entity,
    pub rows: Vec<Entity>,
    pub values: Vec<Entity>,
    pub back_button: Entity,
}

#[derive(Default)]
pub struct HudHandles {
    pub root: Entity,
    pub map_label: Entity,
    pub hint_label: Entity,
    pub goal_label: Entity,
    pub move_label: Entity,
    pub push_label: Entity,
    pub par_label: Entity,
    pub layer_label: Entity,
    /// What is in your hands and what the light you are standing in is lending
    /// you, on the boards that have either.
    pub gem_label: Entity,
    pub fps_label: Entity,
    /// The run speed control and the face of it, shown only while a run is on.
    pub speed_button: Entity,
    pub speed_label: Entity,
    /// The line of controls along the bottom, which says something different
    /// while a run is on.
    pub footer_label: Entity,
}

#[derive(Default)]
pub struct PauseHandles {
    pub root: Entity,
    pub resume_button: Entity,
    pub restart_button: Entity,
    pub solve_button: Entity,
    pub menu_button: Entity,
    pub quit_button: Entity,
}

#[derive(Default)]
pub struct CompleteHandles {
    pub root: Entity,
    pub title_label: Entity,
    pub stats_label: Entity,
    pub next_button: Entity,
    pub retry_button: Entity,
    pub menu_button: Entity,
}

#[derive(Default)]
pub struct FinaleHandles {
    pub root: Entity,
    pub stats_label: Entity,
    pub menu_button: Entity,
    pub quit_button: Entity,
}

/// The mechanics gallery: a rail of lessons and the card that explains the one
/// on screen.
#[derive(Clone, Default)]
pub struct GalleryHandles {
    pub root: Entity,
    pub items: Vec<Entity>,
    pub item_bars: Vec<Entity>,
    pub item_labels: Vec<Entity>,
    /// Which lesson each row stands for, since the rows are grouped by topic and
    /// no longer run in the order the lessons are declared.
    pub item_lessons: Vec<usize>,
    pub title_label: Entity,
    pub counter_label: Entity,
    pub blurb_label: Entity,
    pub practice_label: Entity,
    pub who_label: Entity,
    pub play_button: Entity,
    pub play_label: Entity,
    pub previous_button: Entity,
    pub next_button: Entity,
    pub reset_button: Entity,
    pub back_button: Entity,
}

#[derive(Clone, Default)]
pub struct EditorHandles {
    pub root: Entity,
    pub brush_buttons: Vec<Entity>,
    pub group_button: Entity,
    pub group_label: Entity,
    pub name_input: Entity,
    pub hint_input: Entity,
    pub size_label: Entity,
    pub width_minus: Entity,
    pub width_plus: Entity,
    pub height_minus: Entity,
    pub height_plus: Entity,
    pub character_button: Entity,
    pub character_label: Entity,
    pub skin_button: Entity,
    pub skin_label: Entity,
    pub win_button: Entity,
    pub win_label: Entity,
    pub layer_label: Entity,
    pub layer_down: Entity,
    pub layer_up: Entity,
    pub add_floor: Entity,
    pub remove_floor: Entity,
    pub slot_label: Entity,
    pub previous_slot: Entity,
    pub next_slot: Entity,
    pub new_button: Entity,
    pub randomize_button: Entity,
    pub analyze_button: Entity,
    pub test_button: Entity,
    pub save_button: Entity,
    pub load_button: Entity,
    pub copy_button: Entity,
    pub back_button: Entity,
    pub rules_button: Entity,
    pub schema_button: Entity,
    pub status_label: Entity,
    pub issue_label: Entity,
    pub rules_panel: Entity,
    pub rules_close: Entity,
    pub rule_boxes: Vec<Entity>,
    pub schema_panel: Entity,
    pub schema_close: Entity,
    pub confirm_panel: Entity,
    pub confirm_yes: Entity,
    pub confirm_no: Entity,
    pub schema_lines: Vec<Entity>,
}

/// The objectives panel: one mark and one label per row, reused every frame.
#[derive(Clone, Debug, Default)]
pub struct ObjectiveHandles {
    pub root: Entity,
    pub marks: Vec<Entity>,
    pub labels: Vec<Entity>,
}

#[derive(Default)]
pub struct UiHandles {
    /// The tree the panels hang off, kept so the whole interface can be scaled
    /// to a window smaller than the one it was laid out for.
    pub root: Entity,
    pub title: TitleHandles,
    pub levels: LevelSelectHandles,
    pub settings: SettingsHandles,
    pub hud: HudHandles,
    pub pause: PauseHandles,
    pub complete: CompleteHandles,
    pub finale: FinaleHandles,
    pub gallery: GalleryHandles,
    pub editor: EditorHandles,
    pub objectives: ObjectiveHandles,
}
