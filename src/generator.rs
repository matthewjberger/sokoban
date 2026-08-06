use crate::rules::GATE_GROUPS;
use crate::schema::{
    Character, Direction, Gem, GemColor, MAX_EXTENT, MIN_EXTENT, Map, Member, Position, Rules,
    Skin, Slant, Slot, Tile, complexity, map_add_floor, map_positions, map_reachable, map_relink,
    map_set_tile, map_slot_for, map_tile, validate,
};
use crate::shortcut::skipped;
use crate::solver::{Progress, Search};
use nightshade::prelude::{Rng, rand};
use serde::{Deserialize, Serialize};

/// Every dial the generator exposes. It is plain serializable data, so a
/// preset can be saved beside a map, tuned in the menu, or handed to a batch
/// run without any of them knowing about each other.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
pub struct Recipe {
    pub floor_width: i32,
    pub floor_height: i32,
    /// Stacked storeys, joined by elevator pairs.
    pub layers: i32,
    /// Extra floors placed beside the first one on the ground storey, which
    /// the player walks onto without any transition.
    pub wings: i32,
    pub crates: usize,
    pub wall_fraction: f32,
    pub ice_patches: usize,
    pub pits: usize,
    pub portal_pairs: usize,
    pub one_way_arrows: usize,
    pub belts: usize,
    pub fragile_squares: usize,
    /// Each one scatters a switch and the gate it throws, since neither half is
    /// worth anything on its own.
    pub switch_gates: usize,
    pub brittle_walls: usize,
    pub water_squares: usize,
    /// Burners, which take a crate and keep it and are ordinary ground to a
    /// body. A hole only one of the two can be lost down is a different
    /// constraint from a hole neither can cross.
    pub incinerators: usize,
    /// Each one scatters a bed of spikes and the switch that raises it, since
    /// neither half is worth anything on its own.
    pub spike_beds: usize,
    /// Panes, which stop a body and pass light.
    pub glass_panes: usize,
    /// Watchers, which kill whatever ends up beside them.
    pub watchers: usize,
    /// Boulders, which nothing shoves and only a pair of hands answers.
    pub boulders: usize,
    /// Mirrors on pallets, which shove like crates and turn light like mirrors.
    pub pallet_mirrors: usize,
    /// Each one scatters a gem and the socket it can be seated in, since a gem
    /// with nowhere to go is an ornament and a socket with no gem is a plinth.
    pub gem_sockets: usize,
    /// Each one scatters a gem and a lock of its colour, for the same reason.
    pub gem_locks: usize,
    /// How many of the party there are. Every one of them is a different class,
    /// because two members of one class are one member with two bodies.
    pub party: usize,
    /// How hard the board should be, as a step on the complexity table.
    pub complexity: u8,
    /// Rejects boards the solver finishes too quickly, which is what keeps a
    /// generated puzzle from being a two move walk.
    pub minimum_moves: usize,
    /// Rejects boards that read as slighter than this, by the schema's own
    /// measure. A run that keeps handing out boards of the same weight is a
    /// demonstration rather than a run, and this is the floor that climbs under
    /// it.
    pub minimum_complexity: u32,
    pub attempts: usize,
    pub solver_budget: usize,
    pub skin: Skin,
    pub character: Character,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            floor_width: 8,
            floor_height: 7,
            layers: 1,
            wings: 0,
            crates: 2,
            wall_fraction: 0.1,
            ice_patches: MIXED.ice_patches,
            pits: MIXED.pits,
            portal_pairs: MIXED.portal_pairs,
            one_way_arrows: MIXED.one_way_arrows,
            belts: MIXED.belts,
            fragile_squares: MIXED.fragile_squares,
            switch_gates: MIXED.switch_gates,
            brittle_walls: MIXED.brittle_walls,
            water_squares: MIXED.water_squares,
            incinerators: MIXED.incinerators,
            spike_beds: MIXED.spike_beds,
            glass_panes: 0,
            watchers: 0,
            boulders: 0,
            pallet_mirrors: 0,
            gem_sockets: 0,
            gem_locks: 0,
            party: 1,
            complexity: 2,
            minimum_moves: 8,
            minimum_complexity: 0,
            attempts: 2000,
            solver_budget: 120_000,
            skin: Skin::Warehouse,
            character: Character::Pusher,
        }
    }
}

/// One setting of the hazard dial: what to call it and how much of each thing
/// to scatter. Keeping the dial as a table rather than a chain of branches
/// means teaching it a new mechanic is a line rather than a rewrite.
pub struct HazardStage {
    pub name: &'static str,
    pub ice_patches: usize,
    pub pits: usize,
    pub portal_pairs: usize,
    pub one_way_arrows: usize,
    pub belts: usize,
    pub fragile_squares: usize,
    pub switch_gates: usize,
    pub brittle_walls: usize,
    pub water_squares: usize,
    pub incinerators: usize,
    pub spike_beds: usize,
    pub glass_panes: usize,
    pub watchers: usize,
    pub boulders: usize,
    pub pallet_mirrors: usize,
    pub gem_sockets: usize,
    pub gem_locks: usize,
    pub party: usize,
}

const NOTHING: HazardStage = HazardStage {
    name: "PLAIN",
    ice_patches: 0,
    pits: 0,
    portal_pairs: 0,
    one_way_arrows: 0,
    belts: 0,
    fragile_squares: 0,
    switch_gates: 0,
    brittle_walls: 0,
    water_squares: 0,
    incinerators: 0,
    spike_beds: 0,
    glass_panes: 0,
    watchers: 0,
    boulders: 0,
    pallet_mirrors: 0,
    gem_sockets: 0,
    gem_locks: 0,
    party: 1,
};

pub const HAZARD_STAGES: [HazardStage; 19] = [
    NOTHING,
    HazardStage {
        name: "ICE",
        ice_patches: 3,
        ..NOTHING
    },
    HazardStage {
        name: "ICE AND PITS",
        ice_patches: 3,
        pits: 1,
        ..NOTHING
    },
    HazardStage {
        name: "PADS",
        portal_pairs: 1,
        ..NOTHING
    },
    HazardStage {
        name: "ARROWS",
        one_way_arrows: 4,
        ..NOTHING
    },
    HazardStage {
        name: "BELTS",
        belts: 3,
        ..NOTHING
    },
    HazardStage {
        name: "FRAGILE",
        fragile_squares: 4,
        ..NOTHING
    },
    HazardStage {
        name: "BRITTLE",
        brittle_walls: 1,
        ..NOTHING
    },
    HazardStage {
        name: "WATER",
        water_squares: 5,
        ..NOTHING
    },
    HazardStage {
        name: "INCINERATORS",
        incinerators: 1,
        ..NOTHING
    },
    HazardStage {
        name: "SPIKES",
        spike_beds: 1,
        ..NOTHING
    },
    HazardStage {
        name: "GLASS",
        glass_panes: 3,
        ..NOTHING
    },
    HazardStage {
        name: "WATCHERS",
        watchers: 1,
        ..NOTHING
    },
    HazardStage {
        name: "BOULDERS",
        boulders: 2,
        ..NOTHING
    },
    HazardStage {
        name: "PALLET MIRRORS",
        pallet_mirrors: 1,
        ..NOTHING
    },
    HazardStage {
        name: "GEMS",
        gem_sockets: 1,
        ..NOTHING
    },
    HazardStage {
        name: "LOCKS",
        gem_locks: 1,
        ..NOTHING
    },
    HazardStage {
        name: "A PARTY",
        party: 2,
        ..NOTHING
    },
    MIXED,
];

/// A mix rather than the lot, and what a board comes with unless something says
/// otherwise. A generator that hands out bare rooms by default is a generator
/// whose whole point is off until somebody finds the dial.
///
/// Two things have to be worth their place in the solution here, the ice and
/// the hole, and the water is the ground they stand in. That is as many as a
/// board can be asked to make load bearing at once: every extra one multiplies
/// how many layouts get thrown away, and past two the dial stops answering on
/// a board with storeys or a wing on it.
const MIXED: HazardStage = HazardStage {
    name: "MIXED",
    ice_patches: 2,
    pits: 1,
    water_squares: 2,
    ..NOTHING
};

/// Lays one stage's scatter on top of whatever the recipe already asks for,
/// which is what lets a rolled board wear several mechanics at once. The table
/// stays the one list of what a mechanic is worth, so teaching the generator a
/// new one is still a line in it rather than a second place to add it.
fn add_hazards(recipe: &mut Recipe, index: usize) {
    let stage = &HAZARD_STAGES[index % HAZARD_STAGES.len()];
    recipe.ice_patches += stage.ice_patches;
    recipe.pits += stage.pits;
    recipe.portal_pairs += stage.portal_pairs;
    recipe.one_way_arrows += stage.one_way_arrows;
    recipe.belts += stage.belts;
    recipe.fragile_squares += stage.fragile_squares;
    recipe.switch_gates += stage.switch_gates;
    recipe.brittle_walls += stage.brittle_walls;
    recipe.water_squares += stage.water_squares;
    recipe.incinerators += stage.incinerators;
    recipe.spike_beds += stage.spike_beds;
    recipe.glass_panes += stage.glass_panes;
    recipe.watchers += stage.watchers;
    recipe.boulders += stage.boulders;
    recipe.pallet_mirrors += stage.pallet_mirrors;
    recipe.gem_sockets += stage.gem_sockets;
    recipe.gem_locks += stage.gem_locks;
    recipe.party = recipe.party.max(stage.party);
}

pub fn apply_hazards(recipe: &mut Recipe, index: usize) {
    let stage = &HAZARD_STAGES[index % HAZARD_STAGES.len()];
    recipe.ice_patches = stage.ice_patches;
    recipe.pits = stage.pits;
    recipe.portal_pairs = stage.portal_pairs;
    recipe.one_way_arrows = stage.one_way_arrows;
    recipe.belts = stage.belts;
    recipe.fragile_squares = stage.fragile_squares;
    recipe.switch_gates = stage.switch_gates;
    recipe.brittle_walls = stage.brittle_walls;
    recipe.water_squares = stage.water_squares;
    recipe.incinerators = stage.incinerators;
    recipe.spike_beds = stage.spike_beds;
    recipe.glass_panes = stage.glass_panes;
    recipe.watchers = stage.watchers;
    recipe.boulders = stage.boulders;
    recipe.pallet_mirrors = stage.pallet_mirrors;
    recipe.gem_sockets = stage.gem_sockets;
    recipe.gem_locks = stage.gem_locks;
    recipe.party = stage.party.max(1);
}

/// How many starting points there are. The menu cycles through them, so it asks
/// here rather than carrying its own count that could fall behind.
pub const PRESET_COUNT: usize = 5;

/// One notch of the complexity dial. Everything that makes a board harder moves
/// together, so one control covers what would otherwise be four that have to be
/// kept in step by hand.
pub struct ComplexityStep {
    pub crates: usize,
    pub wall_fraction: f32,
    pub minimum_moves: usize,
    pub attempts: usize,
    pub solver_budget: usize,
}

pub const COMPLEXITY: [ComplexityStep; 5] = [
    ComplexityStep {
        crates: 1,
        wall_fraction: 0.06,
        minimum_moves: 4,
        attempts: 1200,
        solver_budget: 60_000,
    },
    ComplexityStep {
        crates: 2,
        wall_fraction: 0.1,
        minimum_moves: 8,
        attempts: 2000,
        solver_budget: 120_000,
    },
    ComplexityStep {
        crates: 2,
        wall_fraction: 0.14,
        minimum_moves: 16,
        attempts: 3000,
        solver_budget: 300_000,
    },
    ComplexityStep {
        crates: 3,
        wall_fraction: 0.18,
        minimum_moves: 24,
        attempts: 4000,
        solver_budget: 600_000,
    },
    ComplexityStep {
        crates: 4,
        wall_fraction: 0.2,
        minimum_moves: 32,
        attempts: 6000,
        solver_budget: 1_200_000,
    },
];

/// Sets everything one notch of the dial decides. The crate count has a dial of
/// its own, so the notch is a floor under it rather than a replacement for it:
/// asking for more crates than the notch wants is allowed, asking for fewer
/// than it needs is not.
pub fn apply_complexity(recipe: &mut Recipe, notch: u8) {
    let index = (notch.max(1) as usize - 1).min(COMPLEXITY.len() - 1);
    let step = &COMPLEXITY[index];
    recipe.complexity = index as u8 + 1;
    recipe.crates = recipe.crates.max(step.crates);
    recipe.wall_fraction = step.wall_fraction;
    recipe.minimum_moves = step.minimum_moves;
    recipe.attempts = step.attempts;
    recipe.solver_budget = step.solver_budget;
}

/// What a board has to be worth to be handed out, which is the whole of what a
/// run carries from one board to the next. The shape of the next board is
/// rolled rather than remembered, so this is deliberately not a recipe: it says
/// what to insist on, never what to build.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Demand {
    pub minimum_moves: usize,
    pub minimum_complexity: u32,
}

impl Default for Demand {
    fn default() -> Self {
        Self {
            minimum_moves: OPENING_MOVES,
            minimum_complexity: 0,
        }
    }
}

/// The fewest moves a board handed out at random is allowed to take. Below this
/// what comes out is a walk rather than a puzzle.
const OPENING_MOVES: usize = 10;

/// How much longer than its opening a run will ask a board to take. Past this
/// the search is being asked for a needle rather than a puzzle.
const MOST_EXTRA_MOVES: usize = 40;

/// What a run asks of its next board. A run that keeps handing out the same
/// weight of board is a demonstration rather than a run, so every board cleared
/// raises the floor under the one after it, in how long it takes and in what
/// the schema makes of what it is built from. The shape stays rolled either
/// way, because what makes the next board different should be the whole board
/// and not one dial nudged along.
pub fn demand(cleared: usize, reached: u32) -> Demand {
    Demand {
        minimum_moves: (OPENING_MOVES + cleared * 2).min(OPENING_MOVES + MOST_EXTRA_MOVES),
        // Near the weight of the board just cleared rather than at it. What a
        // rolled board is built from is not something the roll can be told to
        // repeat, so insisting on matching the last one is insisting on a floor
        // most rolls fall under, and a run that asks for that is a run that
        // stops at the first heavy board it happens to hand out.
        minimum_complexity: reached * 3 / 4,
    }
}

/// How many shapes a run rolls through before it is asking for nothing but a
/// board. Every one that comes to nothing gives a little of the demand up, so a
/// floor no roll can reach becomes one some roll can rather than one the run
/// stops on.
const EASE_STEPS: usize = 12;

/// What a run still insists on, having rolled this many shapes without a board.
fn eased(demand: Demand, rolled: usize) -> Demand {
    let held = 1.0 - rolled.min(EASE_STEPS) as f32 / EASE_STEPS as f32;
    Demand {
        minimum_moves: OPENING_MOVES
            + (demand.minimum_moves.saturating_sub(OPENING_MOVES) as f32 * held) as usize,
        minimum_complexity: (demand.minimum_complexity as f32 * held) as u32,
    }
}

/// How many mechanics one rolled board may be asked to carry. Every one of them
/// multiplies the layouts that get thrown away, and a run that rolls a fresh
/// board the moment one is slow to come out would otherwise spend all its time
/// on shapes nobody ever sees.
const MOST_ROLLED_STAGES: usize = 3;

/// The shapes a rolled board is built on. Small enough to prove, and varied
/// enough that two boards in a row rarely look alike.
const ROLLED_SHAPES: [(i32, i32); 6] = [(7, 7), (8, 7), (9, 8), (10, 9), (12, 9), (14, 11)];

/// A whole board asked for at random: its floor, its storeys, who is playing,
/// how many of them there are, and whichever mechanics it happens to draw.
///
/// Nothing here is a setting. A generator behind a screen of dials is a
/// generator most people never turn on, so the one control is the button, and
/// what comes out is meant to surprise the person who pressed it.
pub fn roll(rng: &mut impl Rng, demand: Demand) -> Recipe {
    let (floor_width, floor_height) = ROLLED_SHAPES[rng.random_range(0..ROLLED_SHAPES.len())];
    let mut recipe = Recipe {
        floor_width,
        floor_height,
        // A storey or a side floor multiplies what the search has to walk
        // without deepening the puzzle, so both are the exception rather than
        // the rule.
        layers: if rng.random_bool(0.18) { 2 } else { 1 },
        wings: if rng.random_bool(0.12) { 1 } else { 0 },
        skin: Skin::ALL[rng.random_range(0..Skin::ALL.len())],
        character: Character::ALL[rng.random_range(0..Character::ALL.len())],
        ..Default::default()
    };
    // From a bare room, so what the board wears is what the roll below puts on
    // it rather than that plus whatever the default carried.
    apply_hazards(&mut recipe, 0);
    apply_complexity(&mut recipe, rng.random_range(1..=4));

    let mut taken: Vec<usize> = Vec::new();
    while taken.len() < rng.random_range(1..=MOST_ROLLED_STAGES) {
        let index = rng.random_range(1..HAZARD_STAGES.len());
        if !taken.contains(&index) {
            taken.push(index);
            add_hazards(&mut recipe, index);
        }
    }

    // A party is rolled on its own rather than left to the one stage that names
    // one, because who is playing is as much of the puzzle as what is on the
    // floor. Every member is a different class, so this is bounded by how many
    // classes there are.
    recipe.party = recipe
        .party
        .max(match rng.random_range(0..10) {
            0..=5 => 1,
            6..=8 => 2,
            _ => 3,
        })
        .min(Character::ALL.len());
    recipe.minimum_moves = recipe.minimum_moves.max(demand.minimum_moves);
    recipe.minimum_complexity = demand.minimum_complexity;
    recipe
}

/// The named starting points the menu cycles through. Each one is only a
/// [`Recipe`], so the dials stay editable from there.
pub fn preset(index: usize) -> Recipe {
    // A preset that names a setting of the hazard dial gets that setting and
    // nothing else. Leaving the fields to the default would quietly stir the
    // mix into a preset whose whole idea is one mechanic.
    let (mut recipe, stage) = match index % PRESET_COUNT {
        0 => (Recipe::default(), "MIXED"),
        1 => (
            Recipe {
                floor_width: 9,
                floor_height: 8,
                minimum_moves: 10,
                skin: Skin::Glacier,
                ..Default::default()
            },
            "ICE",
        ),
        2 => (
            Recipe {
                floor_width: 9,
                floor_height: 8,
                crates: 3,
                minimum_moves: 12,
                skin: Skin::Vault,
                ..Default::default()
            },
            "ICE AND PITS",
        ),
        3 => (
            Recipe {
                floor_width: 7,
                floor_height: 7,
                layers: 2,
                crates: 2,
                wall_fraction: 0.08,
                minimum_moves: 12,
                skin: Skin::Vault,
                ..Default::default()
            },
            "PLAIN",
        ),
        // A big board is cheap to search while it stays sparse, so the wide
        // preset spends its room on distance and keeps the crate count down.
        _ => (
            Recipe {
                floor_width: 18,
                floor_height: 13,
                crates: 2,
                wall_fraction: 0.16,
                minimum_moves: 20,
                attempts: 3000,
                solver_budget: 900_000,
                skin: Skin::Warehouse,
                ..Default::default()
            },
            "ICE",
        ),
    };
    apply_hazards(&mut recipe, hazard_stage_named(stage));
    recipe
}

/// A setting of the hazard dial by the name it shows, so a preset picks one by
/// what it is rather than by where it happens to sit in the table.
fn hazard_stage_named(name: &str) -> usize {
    HAZARD_STAGES
        .iter()
        .position(|stage| stage.name == name)
        .unwrap_or(0)
}

/// Roughly what laying a board out costs, measured in positions walked, so a
/// run that keeps rejecting layouts yields the frame as readily as one grinding
/// through a search.
const LAYOUT_COST: usize = 512;

/// The board the solver budgets in [`ComplexityStep`] are written for. Anything
/// larger is scaled from here.
const BASE_AREA: usize = 56;

/// How many positions a whole run will walk before it gives up. Generation runs
/// off the frame, so this is a wait rather than a freeze, but a wait has to end
/// somewhere, and a recipe that cannot be met inside it is one to say no to
/// rather than to keep grinding at.
const RUN_POSITIONS: usize = 40_000_000;

/// How many candidates a run is guaranteed a real try at, however large the
/// board. Laying boards out and throwing them away is how a generator finds
/// one, so a run that can afford a single candidate is a run that has stopped
/// looking.
const LEAST_ATTEMPTS: usize = 20;

/// What one rolled shape is given before another is rolled in its place. A
/// generator that grinds at a shape it cannot fill is the whole of what makes
/// one unusable, and the cheapest answer is not a longer wait but a different
/// board.
const PATIENCE: usize = 200_000;

/// How many layouts one rolled shape is worth trying. The dials a recipe
/// carries are written for somebody who asked for that recipe and will wait for
/// it; a rolled shape is one of many and is held to a shorter leash.
const MOST_ROLL_ATTEMPTS: usize = 400;

/// What one candidate of this shape is worth spending on. A storey or a side
/// floor multiplies the positions a search has to walk without making the
/// puzzle any deeper, so the budget the recipe names is for one floor of the
/// default size and is scaled from there. Without this a big board produces
/// nothing but candidates the search runs out of budget on, which reads as a
/// generator that has stopped rather than one that is being asked too much.
///
/// Scaled and then held under a share of what the whole run may walk. Scaling
/// one side of that and not the other buys a big board a deeper look at its
/// first candidate and pays for it with every candidate after, which is the
/// wrong way round: the larger the board, the more layouts have to be thrown
/// away before one is worth proving.
///
/// A board smaller than the one the budgets are written for is still worth the
/// whole of one, so the scale never falls below itself. Rounding it to nothing
/// hands the search a budget of no positions, and every candidate comes back
/// undecided on the position it started from.
fn budget_for(recipe: &Recipe) -> usize {
    let floors = (recipe.layers.max(1) + recipe.wings.max(0)) as usize;
    let area = (recipe.floor_width.clamp(MIN_EXTENT, MAX_EXTENT)
        * recipe.floor_height.clamp(MIN_EXTENT, MAX_EXTENT)) as usize;
    recipe
        .solver_budget
        .saturating_mul((floors * area / BASE_AREA).max(1))
        .min(RUN_POSITIONS / LEAST_ATTEMPTS)
}

/// Where a run has got to. A whole map is a large thing to carry beside two
/// empty answers, so it travels behind a pointer.
pub enum Outcome {
    /// Still going. Nothing has been decided and nothing has run out.
    Working,
    Ready(Box<Map>),
    /// Every attempt spent without a board worth playing.
    Barren,
}

/// A generation in progress. Laying a board out is the cheap half and proving
/// it is not, so a run holds the candidate it is currently proving and hands a
/// board back only once the search has finished with it. Nothing here runs
/// longer than it is told to, which is what keeps generating a board off the
/// frame that asked for one.
///
/// Being finishable is not enough. Scattering hazards across a board and
/// finding a route through it usually produces a route that goes round them,
/// which is a board wearing the mechanics the recipe asked for rather than one
/// using them. So the route is read back for what it skipped, and a board the
/// solution can short circuit is thrown away with the unsolvable ones.
pub struct Run {
    recipe: Recipe,
    /// What to insist on, for a run that rolls its own shapes. A run handed a
    /// recipe to fill has nothing to insist on beyond that recipe, and stops
    /// when the recipe runs dry rather than reaching for another one.
    demand: Option<Demand>,
    /// What one candidate of this shape is allowed to walk.
    budget: usize,
    attempts_left: usize,
    attempted: usize,
    rolled: usize,
    /// Positions walked across the whole run, which is what a run is really
    /// spending and the only bound that holds for every shape of board.
    spent: usize,
    /// The same, since the shape in hand was rolled. A shape that has had this
    /// much spent on it without giving anything up is a shape to leave.
    since_roll: usize,
    candidate: Option<(Map, Search)>,
}

impl Run {
    pub fn new(recipe: &Recipe) -> Self {
        Self {
            recipe: *recipe,
            demand: None,
            budget: budget_for(recipe),
            attempts_left: recipe.attempts.max(1),
            attempted: 0,
            rolled: 0,
            spent: 0,
            since_roll: 0,
            candidate: None,
        }
    }

    /// A run that asks for a board rather than for a particular board. It rolls
    /// a shape, gives it a fair try, and rolls another the moment that one
    /// stops looking promising, so what a player waits on is a board arriving
    /// rather than one shape being ground at until it gives in.
    pub fn rolling(demand: Demand) -> Self {
        let mut run = Self::new(&roll(&mut rand::rng(), demand));
        run.demand = Some(demand);
        run.attempts_left = run.attempts_left.clamp(1, MOST_ROLL_ATTEMPTS);
        run.rolled = 1;
        run
    }

    /// Throws the shape in hand away and rolls another, keeping what the run has
    /// spent and what it is holding out for.
    fn reroll(&mut self, demand: Demand) {
        self.recipe = roll(&mut rand::rng(), eased(demand, self.rolled));
        self.budget = budget_for(&self.recipe);
        self.attempts_left = self.recipe.attempts.clamp(1, MOST_ROLL_ATTEMPTS);
        self.since_roll = 0;
        self.rolled += 1;
        self.candidate = None;
    }

    /// How many boards have been laid out so far, and how far the search has got
    /// through the one it is on. Both are for the screen that has to say
    /// something while it waits.
    pub fn attempted(&self) -> usize {
        self.attempted
    }

    /// How many shapes have been tried, which is the other half of what a screen
    /// waiting on a board has to say.
    pub fn rolled(&self) -> usize {
        self.rolled
    }

    /// Spends about this many positions worth of work and says where that left
    /// it.
    pub fn advance(&mut self, slice: usize) -> Outcome {
        let mut spent = 0usize;
        while spent < slice {
            if self.candidate.is_none() {
                spent += LAYOUT_COST;
                if self.spent > RUN_POSITIONS {
                    return Outcome::Barren;
                }
                // A shape that has had its share and given nothing up is a
                // shape to leave rather than one to keep asking. Rolling
                // another costs nothing and is the whole of why waiting on this
                // ends.
                if self.attempts_left == 0 || self.since_roll > PATIENCE {
                    let Some(demand) = self.demand else {
                        return Outcome::Barren;
                    };
                    self.reroll(demand);
                }
                self.attempts_left -= 1;
                self.attempted += 1;
                let mut rng = rand::rng();
                let Some(map) = lay_out(&self.recipe, &mut rng) else {
                    continue;
                };
                if !validate(&map).is_empty() {
                    continue;
                }
                let search = Search::new(&map, self.budget);
                self.candidate = Some((map, search));
            }

            let Some((map, search)) = &mut self.candidate else {
                continue;
            };
            let before = search.explored();
            let progress = search.advance(map, slice.saturating_sub(spent));
            let walked = search.explored() - before;
            spent += walked;
            self.spent += walked;
            self.since_roll += walked;
            match progress {
                Progress::Running => {}
                Progress::Solved(route) => {
                    let Some((mut map, _)) = self.candidate.take() else {
                        continue;
                    };
                    let moves = route.len();
                    if moves >= self.recipe.minimum_moves
                        && complexity(&map) >= self.recipe.minimum_complexity
                        && skipped(&map, &route).is_empty()
                    {
                        map.par = moves as u32;
                        map.name = describe(&self.recipe);
                        map.hint =
                            format!("generated and solved in {moves} moves before you saw it");
                        return Outcome::Ready(Box::new(map));
                    }
                }
                Progress::Unsolvable | Progress::Exhausted => self.candidate = None,
            }
        }
        Outcome::Working
    }
}

/// A run taken to its end, for the batch tools that can afford to wait for the
/// answer. The game itself never calls this. It spends a slice a frame on a
/// [`Run`] instead, so the window keeps drawing while the search works.
pub fn generate(recipe: &Recipe) -> Option<Map> {
    let mut run = Run::new(recipe);
    loop {
        match run.advance(65_536) {
            Outcome::Working => {}
            Outcome::Ready(map) => return Some(*map),
            Outcome::Barren => return None,
        }
    }
}

/// The same for a run that rolls its own shapes, which is the one the game
/// itself asks for. The batch tools read it here so what they report on is what
/// the button does.
pub fn generate_rolling(demand: Demand) -> Option<Map> {
    let mut run = Run::rolling(demand);
    loop {
        match run.advance(65_536) {
            Outcome::Working => {}
            Outcome::Ready(map) => return Some(*map),
            Outcome::Barren => return None,
        }
    }
}

fn describe(recipe: &Recipe) -> String {
    let floors = recipe.layers.max(1) + recipe.wings.max(0);
    if floors > 1 {
        format!("Random {} Floor Map", floors)
    } else {
        "Random Map".to_string()
    }
}

fn lay_out(recipe: &Recipe, rng: &mut impl Rng) -> Option<Map> {
    let mut map = Map {
        floor_width: recipe.floor_width.clamp(5, 24),
        floor_height: recipe.floor_height.clamp(5, 24),
        rules: Rules::default(),
        skin: recipe.skin,
        character: recipe.character,
        ..Default::default()
    };

    for layer in 0..recipe.layers.max(1) {
        map_add_floor(
            &mut map,
            Slot {
                column: 0,
                row: 0,
                layer,
            },
        );
    }
    for column in 1..=recipe.wings.max(0) {
        map_add_floor(
            &mut map,
            Slot {
                column,
                row: 0,
                layer: 0,
            },
        );
    }

    for layer in 0..recipe.layers.max(1) - 1 {
        let shaft = (
            rng.random_range(1..map.floor_width - 1),
            rng.random_range(1..map.floor_height - 1),
        );
        map_set_tile(&mut map, Position::new(layer, shaft), Tile::Elevator);
        map_set_tile(&mut map, Position::new(layer + 1, shaft), Tile::Elevator);
    }

    let mut open: Vec<Position> = map_positions(&map)
        .into_iter()
        .filter(|at| map_tile(&map, *at) == Tile::Floor)
        .collect();
    shuffle(&mut open, rng);

    let mut cursor = 0;
    let wall_budget = (open.len() as f32 * recipe.wall_fraction.clamp(0.0, 0.4)) as usize;
    for _ in 0..wall_budget {
        let at = *open.get(cursor)?;
        cursor += 1;
        map_set_tile(&mut map, at, Tile::Wall);
    }

    // The player and what they are moving go down before the hazards do. A hole
    // or a patch of ice is only worth putting on a board if the answer has to
    // deal with it, and a board with storeys or a wing on it is mostly floor the
    // answer never walks, so scattering over all of it puts most of the hazards
    // where nothing will ever meet them.
    //
    // A hole and a cracked wall are each a job the board is asking for, and each
    // one is paid for with a crate that never reaches a marker, so the board is
    // handed one spare per hole and per wall.
    let standable = open.len() - cursor;
    let deliveries = deliverable(recipe.crates, standable);
    let crate_count = deliveries + recipe.pits + recipe.brittle_walls;
    if standable < crate_count + deliveries + 1 {
        return None;
    }

    map.player = *open.get(cursor)?;
    cursor += 1;
    while map.crates.len() < crate_count {
        let at = *open.get(cursor)?;
        cursor += 1;
        if wedged(&map, at) {
            continue;
        }
        map.crates.push(at);
    }
    while map.goals.len() < deliveries {
        let at = *open.get(cursor)?;
        cursor += 1;
        if map.crates.contains(&at) {
            continue;
        }
        map.goals.push(at);
    }

    // Whatever is left, on the floors the answer stands on. Anywhere else is a
    // room the solution has no reason to enter.
    let mut working: Vec<Slot> = Vec::new();
    for at in std::iter::once(map.player)
        .chain(map.crates.iter().copied())
        .chain(map.goals.iter().copied())
    {
        let slot = map_slot_for(&map, at).0;
        if !working.contains(&slot) {
            working.push(slot);
        }
    }
    let ground: Vec<Position> = open
        .iter()
        .copied()
        .skip(cursor)
        .filter(|at| working.contains(&map_slot_for(&map, *at).0))
        .collect();

    let mut next = 0;
    let take = |next: &mut usize| -> Option<Position> {
        let at = *ground.get(*next)?;
        *next += 1;
        Some(at)
    };
    for _ in 0..recipe.ice_patches {
        map_set_tile(&mut map, take(&mut next)?, Tile::Ice);
    }
    for _ in 0..recipe.pits {
        map_set_tile(&mut map, take(&mut next)?, Tile::Pit);
    }
    for _ in 0..recipe.portal_pairs {
        let first = take(&mut next)?;
        let second = take(&mut next)?;
        map_set_tile(&mut map, first, Tile::Portal);
        map_set_tile(&mut map, second, Tile::Portal);
        map.portals.push((first, second));
    }
    for _ in 0..recipe.one_way_arrows {
        map_set_tile(&mut map, take(&mut next)?, Tile::OneWay(any_way(rng)));
    }
    for _ in 0..recipe.belts {
        map_set_tile(&mut map, take(&mut next)?, Tile::Conveyor(any_way(rng)));
    }
    for _ in 0..recipe.water_squares {
        map_set_tile(&mut map, take(&mut next)?, Tile::Water);
    }
    for _ in 0..recipe.brittle_walls {
        map_set_tile(&mut map, take(&mut next)?, Tile::Brittle);
    }
    for _ in 0..recipe.fragile_squares {
        map_set_tile(&mut map, take(&mut next)?, Tile::Fragile);
    }
    for _ in 0..recipe.incinerators {
        map_set_tile(&mut map, take(&mut next)?, Tile::Incinerator);
    }
    for _ in 0..recipe.glass_panes {
        map_set_tile(&mut map, take(&mut next)?, Tile::Glass);
    }
    for _ in 0..recipe.watchers {
        map.watchers.push(take(&mut next)?);
    }
    for _ in 0..recipe.boulders {
        map.stones.push(take(&mut next)?);
    }
    for _ in 0..recipe.pallet_mirrors {
        let at = take(&mut next)?;
        let slant = if rng.random_bool(0.5) {
            Slant::Forward
        } else {
            Slant::Back
        };
        map.mirrors.push((at, slant));
    }
    // A gem and somewhere for it to go, scattered together. Either half alone
    // is furniture.
    for index in 0..recipe.gem_sockets {
        let socket = take(&mut next)?;
        let gem = take(&mut next)?;
        map_set_tile(&mut map, socket, Tile::Socket(any_way(rng)));
        map.gems.push(Gem {
            at: gem,
            color: GemColor::ALL[index % GemColor::ALL.len()],
        });
    }
    for index in 0..recipe.gem_locks {
        let colour = GemColor::ALL[index % GemColor::ALL.len()];
        let lock = take(&mut next)?;
        let gem = take(&mut next)?;
        map_set_tile(&mut map, lock, Tile::Lock(colour));
        map.gems.push(Gem {
            at: gem,
            color: colour,
        });
    }
    // A switch and the thing it answers go down together. A group with only one
    // half is a map the static pass would reject anyway, and the groups are
    // handed out from one counter so a door and a bed of spikes never end up
    // wired to each other by accident.
    let mut group = 0usize;
    for _ in 0..recipe.switch_gates.min(GATE_GROUPS) {
        let lever = take(&mut next)?;
        let gate = take(&mut next)?;
        map_set_tile(&mut map, lever, Tile::Switch(group as u8));
        map_set_tile(&mut map, gate, Tile::Gate(group as u8));
        group += 1;
    }
    for _ in 0..recipe.spike_beds {
        if group >= GATE_GROUPS {
            break;
        }
        let lever = take(&mut next)?;
        let bed = take(&mut next)?;
        map_set_tile(&mut map, lever, Tile::Switch(group as u8));
        map_set_tile(&mut map, bed, Tile::Spike(group as u8));
        group += 1;
    }

    // A board with a boulder on it needs hands that can break one, or the
    // boulder is a wall the static pass rejects the board for.
    if !map.stones.is_empty() && !map.character.abilities().smashes {
        map.character = Character::Breaker;
    }
    // Gems scattered as keys are keys. Leaving the light lending powers would
    // make every one of those boards a board about a power nothing on it needs.
    if recipe.gem_locks > 0 && recipe.gem_sockets == 0 {
        map.rules.gem_light_grants_powers = false;
    }

    // A party is a set of classes rather than a count of bodies, because two
    // members of one class are one member with two bodies.
    let mut classes: Vec<Character> = Character::ALL.to_vec();
    for index in (1..classes.len()).rev() {
        classes.swap(index, rng.random_range(0..=index));
    }
    classes.retain(|class| *class != map.character);
    for class in classes.into_iter().take(recipe.party.saturating_sub(1)) {
        let Some(at) = take(&mut next) else {
            break;
        };
        map.followers.push(Member {
            at,
            character: class,
        });
    }

    map_relink(&mut map);
    let reachable = map_reachable(&map);
    if !map
        .crates
        .iter()
        .chain(map.goals.iter())
        .all(|at| reachable.contains(at))
    {
        return None;
    }

    Some(map)
}

/// How many crates a board this size can be asked for and still be provable.
/// The search walks a position for every arrangement of the crates on it, so
/// one more crate costs the whole board again, and a big board spends its room
/// on distance instead. The number is calibrated against the boards that do
/// finish: fifteen by eleven carries four, and the widest shipped board carries
/// two.
fn deliverable(wanted: usize, squares: usize) -> usize {
    (SEARCHABLE_SQUARES / squares.max(1)).clamp(1, wanted)
}

/// Squares times crates, past which no budget worth waiting for decides the
/// board.
const SEARCHABLE_SQUARES: usize = 700;

fn any_way(rng: &mut impl Rng) -> Direction {
    Direction::ALL[rng.random_range(0..Direction::ALL.len())]
}

/// A crate against two perpendicular walls can never move again, so placing one
/// there only wastes the solver's time.
fn wedged(map: &Map, at: Position) -> bool {
    let blocked = |delta| map_tile(map, at.offset(delta)).blocks_walking();
    (blocked((1, 0)) || blocked((-1, 0))) && (blocked((0, 1)) || blocked((0, -1)))
}

fn shuffle(positions: &mut [Position], rng: &mut impl Rng) {
    for index in (1..positions.len()).rev() {
        positions.swap(index, rng.random_range(0..=index));
    }
}
