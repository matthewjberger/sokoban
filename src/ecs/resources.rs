use crate::ecs::UiHandles;
use crate::rules::{BeamSegment, Direction, MapState, Step};
use crate::schema::{GemColor, Map, Position, Slot, map_blank};
use crate::systems::world::work::Work;
use nalgebra_glm::{Vec2, Vec3};
use nightshade::prelude::Entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Title,
    LevelSelect,
    Story,
    Settings,
    Gallery,
    InGame,
    Paused,
    MapComplete,
    CampaignComplete,
    Editor,
}

/// Which of the front screen's two menus is up.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TitleMenu {
    #[default]
    Root,
    Play,
}

/// Where a map in play came from. Progression, the map counter, and what
/// happens when it is solved all read this instead of guessing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MapOrigin {
    Campaign(usize),
    Random,
    /// One board of an endless run. Finishing it produces the next rather than
    /// a screen saying it was finished.
    Endless,
    /// The overworld. It is a map like any other and is played like one, and
    /// the only difference is that some of its squares are doors.
    Overworld,
    /// A campaign puzzle entered through a door in the overworld, which is
    /// where finishing it goes back to.
    Story(usize),
    /// A gallery board. It is played like any other map, but solving one is not
    /// an achievement to announce, because the point was to watch the rule work.
    Lesson,
    #[default]
    Authored,
}

/// How fast a run is being watched. An endless run mostly runs itself, and
/// watching one at the speed a person plays at is watching a clock, so this is
/// the dial that says how much of it to sit through. It moves the game's own
/// clock rather than any one timer, so the moves, the pauses between them and
/// the wait between boards all come along together.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RunSpeed {
    #[default]
    Normal,
    Fast,
    Faster,
}

impl RunSpeed {
    pub fn factor(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Fast => 3.0,
            Self::Faster => 8.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "SPEED  1x",
            Self::Fast => "SPEED  3x",
            Self::Faster => "SPEED  8x",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Fast,
            Self::Fast => Self::Faster,
            Self::Faster => Self::Normal,
        }
    }
}

/// What a board being generated is for. A board asked for on its own and the
/// first of a run are both something to go to once they are ready. The next
/// board of a run already going replaces the one under the player without a
/// screen in between.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Making {
    Single,
    RunStart,
    RunNext,
}

/// A map queued for the world to build, together with its provenance.
#[derive(Clone, Debug, Default)]
pub struct MapRequest {
    pub map: Map,
    pub origin: MapOrigin,
}

/// Every entity the current map owns. Crates and the player carry their parts
/// so a storey change can retag the whole actor at once.
#[derive(Default)]
pub struct MapEntities {
    pub spawned: Vec<Entity>,
    pub crates: Vec<Entity>,
    pub crate_parts: Vec<[Entity; 2]>,
    pub goal_markers: Vec<Entity>,
    pub goal_covered: Vec<bool>,
    pub crate_covered: Vec<bool>,
    /// One crystal per gem the map lists, in the map's order.
    pub gems: Vec<Entity>,
    /// One post per watcher, in the order the state holds them.
    pub watchers: Vec<Entity>,
    /// One body per member of the party, in the order the map lists them.
    pub members: Vec<Entity>,
    /// The pieces each member is built from, so the whole of one can be
    /// outlined rather than the block at the middle of it.
    pub member_parts: Vec<Vec<Entity>>,
    /// What each member's body is giving off, so the material behind it is
    /// only written when the answer has actually changed.
    pub member_glow: Vec<[f32; 3]>,
    pub player_parts: Vec<Entity>,
    pub prompt: Entity,
    /// The burst left where somebody went under. One per board, moved and fired
    /// again, since a death can happen as often as the player likes.
    pub splash: Entity,
    pub layer: i32,
}

pub struct CameraRig {
    pub entity: Entity,
    pub focus: Vec3,
    /// Slides the camera without turning it, for screens whose panels own part
    /// of the view and would otherwise sit on top of the board.
    pub shift: Vec2,
    pub extent: Vec2,
    pub distance: f32,
    pub pitch: f32,
    pub settled: bool,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            entity: Entity::default(),
            focus: Vec3::new(0.0, 0.0, 0.0),
            shift: Vec2::new(0.0, 0.0),
            extent: Vec2::new(9.0, 7.0),
            distance: 12.0,
            pitch: 0.98,
            settled: false,
        }
    }
}

/// Where the story has got to. The cleared list is the whole of the
/// progression, and everything else here is only about putting the player back
/// where they were.
#[derive(Default)]
pub struct StoryProgress {
    pub cleared: Vec<bool>,
    /// The depot as the player left it. It is a board being played, so going
    /// into a room and coming back has to find it as it was, with the crate
    /// still on the plate and the gate it holds open still open.
    pub depot: Option<MapState>,
    pub area: usize,
    pub at_door: Option<usize>,
    /// An area whose opening scene has not been played yet.
    pub pending_scene: Option<usize>,
    pub opening_seen: bool,
    pub ending_seen: bool,
}

/// What the player has turned on. Plain data with a switch each, so the screen
/// that edits them and the systems that read them share one list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub auto_solve: bool,
    pub rewind_effect: bool,
    pub bloom: bool,
    pub reflections: bool,
    pub ambient_occlusion: bool,
    pub water: bool,
    pub show_objectives: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_solve: false,
            rewind_effect: true,
            bloom: true,
            reflections: true,
            ambient_occlusion: true,
            water: true,
            // Asked for by name rather than shown by default. It is a way to see
            // the board as a dependency, which is worth having and not worth
            // putting between the player and the board every time.
            show_objectives: false,
        }
    }
}

/// One switch on [`Settings`]: what to call it, what it does, and how to reach
/// it. Naming them once means the screen and the systems cannot drift apart.
pub type SettingSwitch = (&'static str, &'static str, fn(&mut Settings) -> &mut bool);

pub const SETTING_SWITCHES: [SettingSwitch; 7] = [
    (
        "AUTO SOLVE",
        "an endless run solves itself and moves on",
        |settings| &mut settings.auto_solve,
    ),
    (
        "REWIND EFFECT",
        "the picture pulls back through itself on undo",
        |settings| &mut settings.rewind_effect,
    ),
    ("BLOOM", "light bleeds past bright edges", |settings| {
        &mut settings.bloom
    }),
    (
        "REFLECTIONS",
        "ice and water pick up what is around them",
        |settings| &mut settings.reflections,
    ),
    (
        "AMBIENT OCCLUSION",
        "corners and contacts gather shadow",
        |settings| &mut settings.ambient_occlusion,
    ),
    (
        "WATER SURFACE",
        "flooded squares get waves and foam",
        |settings| &mut settings.water,
    ),
    (
        "OBJECTIVES",
        "list what the board wants and tick it off",
        |settings| &mut settings.show_objectives,
    ),
];

/// What is pooling on a square. A lamp throws warm light that lends nothing and
/// answers to sensors; a seated gem throws a colour that lends what the colour
/// is for. The board draws both, because both are rules about which squares you
/// are standing on and neither is something a rendered light can say.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightPool {
    Lamp,
    Aura(GemColor),
}

/// Where a run of moves has got to, whether it is a worked example in the
/// gallery or a solution playing itself out. While one is running the player
/// watches rather than plays, which is the only time the game takes the
/// controls.
#[derive(Default)]
pub struct Playback {
    /// The moves to play, in order. Owned rather than borrowed from whoever
    /// asked, so a solution and a lesson are the same kind of thing here.
    pub script: Vec<Step>,
    /// Whether reaching the end starts it over.
    pub looping: bool,
    pub playing: bool,
    pub step: usize,
    pub timer: f32,
}

#[derive(Default)]
pub struct InputRepeat {
    pub held: Option<Direction>,
    pub stick: Option<Direction>,
    pub timer: f32,
}

/// What the editor paints into a square.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Brush {
    /// Paints nothing. Clicking a square reports what is on it, which is the
    /// only way to read a board without changing it.
    #[default]
    Select,
    Wall,
    Floor,
    Ice,
    Pit,
    Plate,
    Gate,
    Portal,
    Elevator,
    OneWay,
    Conveyor,
    Fragile,
    Switch,
    Brittle,
    Water,
    Emitter,
    Mirror,
    Tower,
    Sensor,
    Socket,
    Glass,
    Prism,
    Splitter,
    Spike,
    Incinerator,
    Shutter,
    Lock,
    Watcher,
    Gem,
    Orb,
    Lamp,
    Stone,
    PalletMirror,
    Goal,
    Crate,
    Player,
    Erase,
}

impl Brush {
    pub const ALL: [Brush; 37] = [
        Brush::Select,
        Brush::Wall,
        Brush::Floor,
        Brush::Ice,
        Brush::Pit,
        Brush::Plate,
        Brush::Gate,
        Brush::Switch,
        Brush::Portal,
        Brush::Elevator,
        Brush::OneWay,
        Brush::Conveyor,
        Brush::Fragile,
        Brush::Brittle,
        Brush::Water,
        Brush::Emitter,
        Brush::Mirror,
        Brush::Tower,
        Brush::Sensor,
        Brush::Socket,
        Brush::Glass,
        Brush::Prism,
        Brush::Splitter,
        Brush::Spike,
        Brush::Incinerator,
        Brush::Shutter,
        Brush::Lock,
        Brush::Watcher,
        Brush::Gem,
        Brush::Orb,
        Brush::Lamp,
        Brush::Stone,
        Brush::PalletMirror,
        Brush::Goal,
        Brush::Crate,
        Brush::Player,
        Brush::Erase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::Wall => "WALL",
            Self::Floor => "FLOOR",
            Self::Ice => "ICE",
            Self::Pit => "PIT",
            Self::Plate => "PLATE",
            Self::Gate => "GATE",
            Self::Portal => "PORTAL",
            Self::Elevator => "ELEVATOR",
            Self::OneWay => "ONE WAY",
            Self::Conveyor => "BELT",
            Self::Fragile => "FRAGILE",
            Self::Switch => "SWITCH",
            Self::Brittle => "BRITTLE",
            Self::Water => "WATER",
            Self::Emitter => "EMITTER",
            Self::Mirror => "MIRROR",
            Self::Tower => "TOWER",
            Self::Sensor => "SENSOR",
            Self::Socket => "SOCKET",
            Self::Glass => "GLASS",
            Self::Prism => "PRISM",
            Self::Splitter => "SPLITTER",
            Self::Spike => "SPIKES",
            Self::Incinerator => "INCINERATOR",
            Self::Shutter => "SHUTTER",
            Self::Lock => "LOCK",
            Self::Watcher => "WATCHER",
            Self::Gem => "GEM",
            Self::Orb => "ORB",
            Self::Lamp => "LAMP",
            Self::Stone => "BOULDER",
            Self::PalletMirror => "PALLET MIRROR",
            Self::Goal => "GOAL",
            Self::Crate => "CRATE",
            Self::Player => "PLAYER",
            Self::Erase => "ERASE",
        }
    }
}

/// Which of the editor's two detail panels is open, if either. They cover the
/// board, so only one shows at a time.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorOverlay {
    #[default]
    None,
    Rules,
    Schema,
    /// Asking before doing something that throws the map away.
    Confirm,
}

pub struct EditorState {
    pub map: Map,
    pub overlay: EditorOverlay,
    pub brush: Brush,
    pub group: u8,
    pub cursor: Position,
    /// The square the select brush last reported on, drawn with a marker so the
    /// answer in the status line has something to point at.
    pub selected: Option<Position>,
    pub slot: Slot,
    pub painting: bool,
    pub cursor_entity: Entity,
    pub marker_entity: Entity,
    pub slots: Vec<String>,
    pub slot_index: usize,
    pub status: String,
    pub issues: String,
    pub needs_rebuild: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            map: map_blank(11, 9),
            overlay: EditorOverlay::None,
            brush: Brush::Select,
            group: 0,
            cursor: Position::new(0, (1, 1)),
            selected: None,
            slot: Slot::default(),
            painting: false,
            cursor_entity: Entity::default(),
            marker_entity: Entity::default(),
            slots: Vec::new(),
            slot_index: 0,
            status: String::new(),
            issues: String::new(),
            needs_rebuild: false,
        }
    }
}

#[derive(Default)]
pub struct SokobanResources {
    /// What this board is asking for and what waits on what, read off the map
    /// when it is laid out.
    pub objectives: crate::objectives::Objectives,
    pub origin: MapOrigin,
    pub title_menu: TitleMenu,
    pub selected_map: usize,
    pub selected_lesson: usize,
    pub playback: Playback,
    /// Something the game wants to say about this board, shown in place of the
    /// map's own hint until the next board replaces it.
    pub notice: String,
    pub settings: Settings,
    /// How much of the rewind is left to play out, counting down from one. It
    /// is a plain number rather than a flag so the effect can ease off rather
    /// than switch off.
    pub rewind: f32,
    /// How long is left of a death before the board is put back. The body is
    /// going down while this runs, so the move that killed is not taken back
    /// the instant it is made.
    pub dying: f32,
    /// How many boards an endless run has finished.
    pub endless_cleared: usize,
    /// How heavy the last board a run handed out read, by the schema's own
    /// measure. The next board is asked to be no lighter, which is what makes a
    /// run climb rather than wander.
    pub endless_weight: u32,
    /// How fast a run is being watched.
    pub run_speed: RunSpeed,
    pub story: StoryProgress,
    /// The beam entities on screen, and the shape they were drawn for. Keeping
    /// the shape means the light is only rebuilt when the light has changed.
    pub beams: Vec<Entity>,
    pub beam_shape: Vec<BeamSegment>,
    /// The pools of light on the floor, and the squares they were drawn for.
    /// What a lamp reaches and what a colour lends are rules about squares, so
    /// the squares themselves are what the board shows.
    pub pools: Vec<Entity>,
    pub pool_shape: Vec<(Position, LightPool)>,
    /// The marks on the floor showing what the watchers can reach, and the
    /// squares they were drawn for.
    pub reach: Vec<Entity>,
    pub reach_shape: Vec<Position>,
    /// The marks on the storey below showing what is standing over it, and the
    /// squares they were drawn for.
    pub footprints: Vec<Entity>,
    pub footprint_shape: Vec<Position>,
    /// The signs over the shafts, and what they were drawn for: the square, the
    /// way it goes, and whether the far end is taken.
    pub signposts: Vec<Entity>,
    pub signpost_shape: Vec<(Position, i32, bool)>,
    pub map: Map,
    pub state: MapState,
    pub undo_stack: Vec<MapState>,
    /// Moves taken back, in the order they were taken back, so a hand that
    /// slipped on the undo can be put right. Any new move throws it away,
    /// because a board that went somewhere else has nothing to redo.
    pub redo_stack: Vec<MapState>,
    pub entities: MapEntities,
    pub ui: UiHandles,
    pub camera: CameraRig,
    pub repeat: InputRepeat,
    pub editor: EditorState,
    pub elapsed: f32,
    pub total_moves: u32,
    pub pending: Option<MapRequest>,
    /// The long search the game is running, if it is running one. It is walked
    /// over as many frames as it takes, so whatever asked for it stays up and
    /// stays alive while it works.
    pub work: Option<Work>,
    pub random_status: String,
    /// The last of those actually written to the screen, so a line that has not
    /// changed is not laid out again on the strength of being read again.
    pub random_status_shown: String,
    /// The same, for the running commentary the completion screen shows while
    /// the board after this one is being made.
    pub notice_shown: String,
    /// When the running commentary on a search was last written. A search takes
    /// a slice of every frame, and a line that says how far it has got is a
    /// line that would otherwise be re-laid out sixty times a second.
    pub work_said_at: f32,
    pub solved_announced: bool,
    pub solved_delay: f32,
}

impl SokobanResources {
    /// How fast the game is being asked to run. Only a run is ever watched at
    /// speed, so this is one everywhere else, and everything that has to keep
    /// up with a run asks here rather than deciding for itself, so the clock
    /// and the searches feeding it cannot disagree about how fast it is going.
    pub fn pace(&self) -> f32 {
        match self.origin {
            MapOrigin::Endless => self.run_speed.factor(),
            _ => 1.0,
        }
    }

    /// The body of the member being played, which is the one the controls and
    /// the camera are pointed at.
    pub fn active_body(&self) -> Entity {
        self.entities
            .members
            .get(self.state.active)
            .copied()
            .unwrap_or_default()
    }
}
