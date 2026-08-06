pub use crate::schema::Direction;
use crate::schema::{
    Abilities, Character, CrateKind, GemColor, Map, Position, Slant, Tile, WinCondition,
    map_elevator_drop, map_elevator_target, map_step, map_teleport_exit, map_tile,
};
use std::ops::ControlFlow;

/// How many separate wirings a board can have. A group is the whole of the
/// wiring, so everything naming one feeds every gate that names it, and this is
/// the count of doors a board can open independently of each other. The depot
/// needs one per seam between its wings.
pub const GATE_GROUPS: usize = 6;

/// One thing a player can do, named rather than performed. A worked example is
/// a list of these, and so is a solution the search found, which is why both
/// can be played back through exactly the same machinery.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Step {
    Go(Direction),
    Drag(Direction),
    Ride(i32),
    /// Point the controls at another member of the party. It costs a move like
    /// anything else, so a solution that swaps about pays for it.
    Take(usize),
    /// Do the one thing the square underfoot allows with what is in your hands:
    /// lift the gem lying there, seat the one you are carrying in the socket you
    /// are standing on, or take back the one already seated. Which of those it
    /// is never has to be chosen, because no square ever offers two.
    Handle,
}

/// How much of a move to work out. Playback needs every square everything
/// crossed so it can draw the trip. A search needs the board that came out and
/// nothing else, and it asks for tens of millions of them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    Full,
    /// The board only. `paths` says the board still reads what was crossed,
    /// which fragile floor and switches both do, so on a board carrying either
    /// the trail is kept whatever the caller wants.
    Position {
        paths: bool,
    },
}

impl Detail {
    fn keeps_paths(self) -> bool {
        match self {
            Self::Full => true,
            Self::Position { paths } => paths,
        }
    }
}

/// Every move available from a state, each labelled with the step that reaches
/// it. The search walks these and playback replays them, so neither can drift
/// from what the rules actually allow.
pub fn expansions(map: &Map, state: &MapState) -> Vec<(Step, MoveOutcome)> {
    let mut moves = Vec::new();
    expand(map, state, Detail::Full, |step, outcome| {
        moves.push((step, outcome));
        ControlFlow::Continue(())
    });
    moves
}

/// The same moves, handed over one at a time. A search holds one position at a
/// time and stops the moment one of its children finishes the board, so the
/// list [`expansions`] builds is a list nothing reads twice.
pub fn expand(
    map: &Map,
    state: &MapState,
    detail: Detail,
    mut visit: impl FnMut(Step, MoveOutcome) -> ControlFlow<()>,
) {
    let gates = gate_flags(map, state);
    // What the one being played can do here, asked once. It is the character
    // plus whatever light they happen to be standing in, and standing still
    // does not change it, so every move out of this position is worked out
    // against the same answer.
    let powers = active_abilities(map, state);
    for direction in Direction::ALL {
        if let Some(outcome) = attempt_move_with(map, state, direction, &gates, powers, detail)
            && visit(Step::Go(direction), outcome).is_break()
        {
            return;
        }
    }
    // A magnet has no separate drag to offer, because its ordinary step already
    // is one, so listing both would be the same move twice.
    if !powers.magnetic {
        for direction in Direction::ALL {
            if let Some(outcome) = attempt_pull_with(map, state, direction, &gates, powers, detail)
                && visit(Step::Drag(direction), outcome).is_break()
            {
                return;
            }
        }
    }
    if let Some(outcome) = attempt_handle(map, state)
        && visit(Step::Handle, outcome).is_break()
    {
        return;
    }
    for way in [-1, 1] {
        if let Some(outcome) = attempt_ride(map, state, way)
            && visit(Step::Ride(way), outcome).is_break()
        {
            return;
        }
    }
    for index in 0..state.members.len() {
        if let Some(outcome) = attempt_take(map, state, index)
            && visit(Step::Take(index), outcome).is_break()
        {
            return;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CrateState {
    pub at: Position,
    pub sunk: bool,
    /// What it is. Everything here shoves the same way, and the kind decides
    /// what it does to light.
    pub kind: CrateKind,
}

/// Where one gem is. A gem is small enough to carry, which is the whole reason
/// it is not a crate: it is somewhere on the floor, in somebody's hands, or
/// seated in a socket doing its work, and it moves between the three by being
/// handled rather than by being shoved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GemSpot {
    Loose(Position),
    /// Carried by the member at this index. Whoever holds it takes it with them,
    /// so where it is is wherever they are.
    Held(usize),
    Seated(Position),
}

impl Default for GemSpot {
    fn default() -> Self {
        Self::Loose(Position::default())
    }
}

impl GemSpot {
    /// The square it is on, for anything reading the board rather than the
    /// party. A gem in somebody's hands is on no square of its own.
    pub fn square(self) -> Option<Position> {
        match self {
            Self::Loose(at) | Self::Seated(at) => Some(at),
            Self::Held(_) => None,
        }
    }
}

/// Everything that changes while a map is played. The map itself never
/// mutates, so undo is a stack of these and nothing else.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapState {
    /// Where each member of the party is standing, in the order the map lists
    /// them, so which character is at which square never has to be guessed.
    pub members: Vec<Position>,
    /// Which of them the controls are pointed at.
    pub active: usize,
    pub facing: Direction,
    pub crates: Vec<CrateState>,
    /// Where each gem is, in the order the map lists them. A gem is never
    /// spent and never lost, so this is a run of the same length from the first
    /// move to the last.
    pub gems: Vec<GemSpot>,
    /// Where each watcher is standing. They never move on their own and there
    /// is exactly one way to move them, which is to trade places with one from
    /// far enough off to still be alive afterwards.
    pub watchers: Vec<Position>,
    pub pits_filled: Vec<Position>,
    /// Fragile squares that have already dropped away. Unlike a filled pit this
    /// is not implied by anything else on the board, so it is state in its own
    /// right and the search has to carry it.
    pub collapsed: Vec<Position>,
    /// Which gate groups a switch has flipped and left flipped.
    pub latched: [bool; GATE_GROUPS],
    /// Brittle walls a crate has been spent breaking.
    pub broken: Vec<Position>,
    pub moves: u32,
    pub pushes: u32,
}

impl MapState {
    /// Where the one being played is standing.
    pub fn player(&self) -> Position {
        self.members.get(self.active).copied().unwrap_or_default()
    }

    fn set_player(&mut self, at: Position) {
        if let Some(slot) = self.members.get_mut(self.active) {
            *slot = at;
        }
    }
}

pub fn initial_state(map: &Map) -> MapState {
    MapState {
        members: (0..map.party_size())
            .map(|index| map.member_start(index))
            .collect(),
        active: 0,
        facing: Direction::Down,
        // Orbs join the crates rather than living beside them, so every rule
        // that moves a crate moves an orb without being told about orbs.
        crates: map
            .crates
            .iter()
            .map(|at| (at, CrateKind::Box))
            .chain(map.orbs.iter().map(|at| (at, CrateKind::Orb)))
            .chain(map.lamps.iter().map(|at| (at, CrateKind::Lamp)))
            .chain(map.stones.iter().map(|at| (at, CrateKind::Stone)))
            .chain(
                map.mirrors
                    .iter()
                    .map(|(at, slant)| (at, CrateKind::Mirror(*slant))),
            )
            .map(|(at, kind)| CrateState {
                at: *at,
                sunk: false,
                kind,
            })
            .collect(),
        // A gem laid on a socket starts in it, so a board can open with its
        // light already on and the puzzle be about moving it somewhere else.
        gems: map
            .gems
            .iter()
            .map(|gem| match map_tile(map, gem.at) {
                Tile::Socket(_) => GemSpot::Seated(gem.at),
                _ => GemSpot::Loose(gem.at),
            })
            .collect(),
        watchers: map.watchers.clone(),
        pits_filled: Vec::new(),
        collapsed: Vec::new(),
        latched: [false; GATE_GROUPS],
        broken: Vec::new(),
        moves: 0,
        pushes: 0,
    }
}

/// Whether a square is within reach of a watcher, which is the four squares
/// around it and not the corners. Standing there is the end of the board, so
/// this is asked of every square anybody arrives on.
pub fn watched(state: &MapState, at: Position) -> bool {
    state.watchers.iter().any(|watcher| {
        watcher.layer == at.layer
            && (watcher.cell.0 - at.cell.0).abs() + (watcher.cell.1 - at.cell.1).abs() == 1
    })
}

/// Whether anybody in the party is carrying a gem of this colour, which is the
/// only thing a lock asks.
///
/// A reading taken of the board with nothing on it has nobody carrying
/// anything, and the graph the search prunes with is exactly that reading. A
/// door that answers to what is being carried has to read as open there, or the
/// graph reports a square as out of reach that a player walks to holding the
/// key.
fn holds_colour(map: &Map, state: &MapState, colour: GemColor) -> bool {
    if state.gems.is_empty() {
        return true;
    }
    map.gems.iter().enumerate().any(|(index, gem)| {
        gem.colour_matches(colour) && matches!(state.gems.get(index), Some(GemSpot::Held(_)))
    })
}

/// One step from a route, played. The search hands back steps rather than
/// positions, so replaying is how a route turns back into the boards it visited
/// and the moves it made getting there.
pub fn play(map: &Map, state: &MapState, step: Step) -> Option<MoveOutcome> {
    expansions(map, state)
        .into_iter()
        .find(|(candidate, _)| *candidate == step)
        .map(|(_, outcome)| outcome)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Actor {
    Player,
    Crate(usize),
}

pub struct MoveOutcome {
    pub state: MapState,
    /// Which member did the moving, since the one being played is no longer the
    /// only one on the board.
    pub mover: usize,
    pub player_path: Vec<Position>,
    pub crate_moves: Vec<(usize, Vec<Position>)>,
    pub sunk: Vec<usize>,
}

/// One straight run of a beam, from where it started or last turned to where it
/// stops or turns again.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BeamSegment {
    pub from: Position,
    pub to: Position,
    /// What colour it is, or nothing for the light an emitter throws. The
    /// colourless light is the light that burns, and every colour is a power
    /// somebody can stand in, so this one field is the difference between a
    /// beam that kills and a beam that lends.
    pub tint: Option<GemColor>,
}

/// Everything the light is doing right now: the runs it travels along, the
/// squares it covers, and the gate groups its towers are powering. It is
/// derived from the board and where the crates are standing, so it is recomputed
/// rather than remembered, and nothing about it belongs in a saved state.
#[derive(Clone, Default, Debug)]
pub struct BeamField {
    pub segments: Vec<BeamSegment>,
    /// Squares the colourless light covers, which are the squares that burn.
    pub covered: Vec<Position>,
    /// Squares coloured light covers, and what colour. One square can be in
    /// here more than once, because two colours crossing lend both.
    pub aura: Vec<(Position, GemColor)>,
    pub powered: [bool; GATE_GROUPS],
}

impl BeamField {
    pub fn burns(&self, at: Position) -> bool {
        self.covered.contains(&at)
    }

    /// What standing here lends. Coloured light is the only light that gives
    /// anything, and two colours reaching one square give both.
    pub fn lends(&self, at: Position) -> Abilities {
        self.aura
            .iter()
            .filter(|(square, _)| *square == at)
            .fold(Abilities::NONE, |powers, (_, color)| {
                powers.union(color.grants())
            })
    }
}

/// One square the light reaches, filed under what kind of light reached it.
fn touch(field: &mut BeamField, at: Position, tint: Option<GemColor>) {
    match tint {
        None => field.covered.push(at),
        Some(color) => field.aura.push((at, color)),
    }
}

/// Traces every beam on the board. A mirror turns one a quarter turn, a crate
/// or an orb stops it, a tower drinks it, and everything else it passes over.
///
/// A body stops it too. Whoever the light reaches is standing in it, so their
/// square is one the light covers and the run ends there. Being unharmed by it
/// is a separate question, asked of the character rather than of the board.
/// One who can stand in a beam is one who can put themselves between it and
/// somebody who cannot.
///
/// A ring of mirrors would send a beam round forever, so a run that arrives
/// somewhere it has already been on the same heading is a run that has closed
/// on itself and is done.
pub fn beam_field(map: &Map, state: &MapState) -> BeamField {
    let mut field = BeamField::default();
    let mut runs = sources(map, state);
    if runs.is_empty() {
        return field;
    }

    // A run is remembered by where it is, where it is going and what colour it
    // is. A ring of mirrors would otherwise send one round forever, and a
    // splitter feeding itself is the same loop with two ends.
    let mut visited: Vec<(Position, Direction, Option<GemColor>)> = Vec::new();
    while let Some((source, start_heading, start_tint)) = runs.pop() {
        let mut at = source;
        let mut heading = start_heading;
        let mut tint = start_tint;
        let mut run_start = at;
        loop {
            if visited.contains(&(at, heading, tint)) {
                break;
            }
            visited.push((at, heading, tint));

            let next = at.offset(heading.delta());
            let tile = map_tile(map, next);
            // The three machines that take light and pass it on are asked about
            // before anything else, because each of them both stops a run and
            // starts another, which nothing that merely blocks does.
            if let Tile::Mirror(slant) = tile {
                touch(&mut field, next, tint);
                field.segments.push(BeamSegment {
                    from: run_start,
                    to: next,
                    tint,
                });
                heading = slant.deflect(heading);
                run_start = next;
                at = next;
                continue;
            }
            if let Tile::Prism(color) = tile {
                touch(&mut field, next, tint);
                field.segments.push(BeamSegment {
                    from: run_start,
                    to: next,
                    tint,
                });
                tint = Some(color);
                run_start = next;
                at = next;
                continue;
            }
            if tile == Tile::Splitter {
                touch(&mut field, next, tint);
                field.segments.push(BeamSegment {
                    from: run_start,
                    to: next,
                    tint,
                });
                // Either side of the line it arrived on, which is both mirrors
                // at once and is why a wedge is drawn as one.
                for slant in [Slant::Forward, Slant::Back] {
                    runs.push((next, slant.deflect(heading), tint));
                }
                break;
            }
            // A mirror on a pallet turns the light exactly as one set into the
            // floor does, so it is asked about before anything that merely
            // blocks, and it is asked of the crates because that is where it is.
            if let Some(index) = crate_at(state, next)
                && let CrateKind::Mirror(slant) = state.crates[index].kind
            {
                touch(&mut field, next, tint);
                field.segments.push(BeamSegment {
                    from: run_start,
                    to: next,
                    tint,
                });
                heading = slant.deflect(heading);
                run_start = next;
                at = next;
                continue;
            }
            let stands = state.members.contains(&next);
            let blocked = tile.stops_light() || stands || crate_at(state, next).is_some();
            if blocked {
                // A tower drinks the light and a body takes it, so both are
                // squares the light reaches. A wall or a crate merely ends it
                // and there is nothing on that square to burn.
                if let Tile::Receiver(group) = tile
                    && let Some(flag) = field.powered.get_mut(group as usize)
                {
                    *flag = true;
                }
                if stands || matches!(tile, Tile::Receiver(_)) {
                    touch(&mut field, next, tint);
                }
                field.segments.push(BeamSegment {
                    from: run_start,
                    to: next,
                    tint,
                });
                break;
            }

            at = next;
            touch(&mut field, at, tint);
        }
    }
    field
}

/// Everywhere light is coming from: the emitters, which are part of the board
/// and always on, and the sockets, which are only sources while somebody has
/// carried a gem to them.
fn sources(map: &Map, state: &MapState) -> Vec<(Position, Direction, Option<GemColor>)> {
    let mut runs: Vec<(Position, Direction, Option<GemColor>)> = map
        .emitters
        .iter()
        .map(|(at, heading)| (*at, *heading, None))
        .collect();
    for (index, gem) in map.gems.iter().enumerate() {
        if let Some(GemSpot::Seated(at)) = state.gems.get(index).copied()
            && let Tile::Socket(heading) = map_tile(map, at)
        {
            runs.push((at, heading, Some(gem.color)));
        }
    }
    runs
}

/// Whether the light on this board is worth asking about. A board with no
/// source and no lens has nothing to trace, and most boards are that board.
fn lit_board(map: &Map) -> bool {
    !map.emitters.is_empty() || !map.gems.is_empty()
}

/// Whether standing in the light lends anything here. Colour is what lends, and
/// colour comes from a gem or from a lens with something to stain, so a board
/// with neither never has to trace a beam to find out what its player can do.
///
/// A board where it does is a board where what a body can do depends on where
/// it is standing, which is why the search has to be told as well: a walk that
/// changes what the walker can do is not a walk that can be priced after the
/// fact.
pub fn light_lends(map: &Map) -> bool {
    // A gem is a source and a colour at once. A lens is only a colour, so it
    // needs something to stain before it lends anything, and a board with a
    // lens standing in an unlit room is a board with an ornament in it.
    map.rules.gem_light_grants_powers
        && (!map.gems.is_empty() || !(map.prisms.is_empty() || map.emitters.is_empty()))
}

/// What the member being played can do right now: what they are by class, plus
/// whatever the light they are standing in lends them. Every rule that asks
/// what a body can do asks this, so a shover in ruby light phases through a
/// wall without a single rule knowing about gems.
pub fn active_abilities(map: &Map, state: &MapState) -> Abilities {
    member_abilities(map, state, state.active)
}

fn member_abilities(map: &Map, state: &MapState, index: usize) -> Abilities {
    let base = map.member_character(index).abilities();
    if !light_lends(map) {
        return base;
    }
    let Some(at) = state.members.get(index).copied() else {
        return base;
    };
    base.union(beam_field(map, state).lends(at))
}

/// Whether the board has killed the player. Standing in a beam does it, which
/// is the only way to die in this game and the reason undo exists twice over.
/// Every square a lamp reaches. A square is lit when a lamp is within range of
/// it and nothing solid stands on the line between them, so where the shadows
/// fall is worked out rather than looked at, and the lights the renderer puts
/// down are placed to agree with it.
pub fn lit_squares(map: &Map, state: &MapState) -> Vec<Position> {
    let range = map.rules.light_range as i32;
    let mut lit: Vec<Position> = Vec::new();
    if range <= 0 {
        return lit;
    }
    let lamps: Vec<Position> = state
        .crates
        .iter()
        .filter(|entry| !entry.sunk && entry.kind == CrateKind::Lamp)
        .map(|entry| entry.at)
        .collect();

    for lamp in &lamps {
        for offset_y in -range..=range {
            for offset_x in -range..=range {
                let at = lamp.offset((offset_x, offset_y));
                if at.layer != lamp.layer {
                    continue;
                }
                // Square range rather than round, because a board is squares.
                if offset_x.abs().max(offset_y.abs()) > range {
                    continue;
                }
                if light_reaches(map, state, *lamp, at) {
                    lit.push(at);
                }
            }
        }
    }
    // Two lamps reaching the same square is one lit square, and anything
    // counting these has to see it once.
    lit.sort_unstable();
    lit.dedup();
    lit
}

/// Whether light gets from one square to another without something standing in
/// the way. The line is walked a square at a time the way a straight line is
/// drawn on a grid, and everything but the two ends has to be clear.
fn light_reaches(map: &Map, state: &MapState, from: Position, to: Position) -> bool {
    let (mut x, mut y) = (from.cell.0, from.cell.1);
    let (target_x, target_y) = (to.cell.0, to.cell.1);
    let (step_x, step_y) = ((target_x - x).signum(), (target_y - y).signum());
    let (span_x, span_y) = ((target_x - x).abs(), (target_y - y).abs());
    let mut error = span_x - span_y;

    loop {
        if (x, y) == (target_x, target_y) {
            return true;
        }
        let doubled = error * 2;
        if doubled > -span_y {
            error -= span_y;
            x += step_x;
        }
        if doubled < span_x {
            error += span_x;
            y += step_y;
        }
        if (x, y) == (target_x, target_y) {
            return true;
        }
        let at = Position::new(from.layer, (x, y));
        // A mirror turns a beam rather than stopping it, but it is a thing
        // standing on its square, and a lamp cannot see past one.
        if map_tile(map, at).stops_light() || crate_at(state, at).is_some() {
            return false;
        }
    }
}

/// Whether the board has killed anybody. Standing in water without being
/// carried by it does it, and so does standing in a beam without being warded
/// against it. Everyone in the party counts, not only whoever is being played.
/// What the board did, for the half of the game that has to show it. The rules
/// only ever need to know whether somebody died, but a body going under water,
/// a body caught in a beam and a body on a bed of spikes are three different
/// things to watch, and the difference is here rather than guessed at by
/// looking at the square afterwards.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Death {
    Drowned,
    Burned,
    Impaled,
    /// Stood within reach of a watcher, which is the four squares around it.
    Watched,
}

impl Death {
    pub fn notice(self) -> &'static str {
        match self {
            Self::Drowned => "gone under",
            Self::Burned => "the beam caught you",
            Self::Impaled => "the spikes came up",
            Self::Watched => "it had you the moment you were beside it",
        }
    }
}

/// How the board killed whoever it killed, if it killed anybody. This is the
/// same reading [`lethal`] takes, kept in one place so what is shown and what
/// is decided cannot disagree.
pub fn killed_by(map: &Map, state: &MapState) -> Option<Death> {
    let field = lit_board(map).then(|| beam_field(map, state));
    let spikes =
        (!map.spikes.is_empty() && map.rules.spikes_impale).then(|| gate_flags(map, state));
    state.members.iter().enumerate().find_map(|(index, at)| {
        let mut abilities = map.member_character(index).abilities();
        if let Some(field) = field.as_ref()
            && map.rules.gem_light_grants_powers
        {
            abilities = abilities.union(field.lends(*at));
        }
        if !abilities.wades && map_tile(map, *at) == Tile::Water {
            return Some(Death::Drowned);
        }
        if !abilities.warded && field.as_ref().is_some_and(|field| field.burns(*at)) {
            return Some(Death::Burned);
        }
        let impaled = spikes.as_ref().is_some_and(|flags| {
            matches!(map_tile(map, *at), Tile::Spike(group)
                if flags.get(group as usize).copied().unwrap_or(false))
        });
        if impaled {
            return Some(Death::Impaled);
        }
        watched(state, *at).then_some(Death::Watched)
    })
}

pub fn lethal(map: &Map, state: &MapState) -> bool {
    let field = lit_board(map).then(|| beam_field(map, state));
    // Spikes answer to their group, and working out what every group is doing
    // costs a trace of the whole board, so a board without spikes never asks.
    let spikes =
        (!map.spikes.is_empty() && map.rules.spikes_impale).then(|| gate_flags(map, state));
    state.members.iter().enumerate().any(|(index, at)| {
        // Light that lends is light that saves. Somebody standing in jade walks
        // on the water they are standing in, which is the whole of why the
        // powers are worked out here rather than read off the class.
        let mut abilities = map.member_character(index).abilities();
        if let Some(field) = field.as_ref()
            && map.rules.gem_light_grants_powers
        {
            abilities = abilities.union(field.lends(*at));
        }
        let drowned = !abilities.wades && map_tile(map, *at) == Tile::Water;
        let burned = !abilities.warded && field.as_ref().is_some_and(|field| field.burns(*at));
        let impaled = spikes.as_ref().is_some_and(|flags| {
            matches!(map_tile(map, *at), Tile::Spike(group)
                if flags.get(group as usize).copied().unwrap_or(false))
        });
        drowned || burned || impaled || watched(state, *at)
    })
}

/// Who the controls are pointed at, and so whose qualities the rules are asking
/// about.
pub fn active_character(map: &Map, state: &MapState) -> Character {
    map.member_character(state.active)
}

/// Which gate groups are held open. A plate is pressed exactly when something
/// stands on it, so this asks what the few things standing are standing on
/// rather than walking the whole board looking for plates. It runs inside the
/// solver's inner loop, where that difference is the difference.
pub fn gate_flags(map: &Map, state: &MapState) -> [bool; GATE_GROUPS] {
    // A latch a switch threw holds its gate open on its own, so plates start
    // from what the switches already decided rather than from nothing.
    let mut flags = state.latched;
    let mut press = |at: Position| {
        if let Tile::Plate(group) = map_tile(map, at)
            && let Some(flag) = flags.get_mut(group as usize)
        {
            *flag = true;
        }
    };
    if map.rules.plates_sense_player {
        // Everyone standing on the board is standing on it, not only the one
        // being played.
        for at in &state.members {
            press(*at);
        }
    }
    if map.rules.plates_sense_crates {
        for entry in state.crates.iter().filter(|entry| !entry.sunk) {
            press(entry.at);
        }
    }
    // A sensor holds its gate for as long as a lamp is reaching it, which is
    // the same bargain a plate makes with a crate.
    if !map.lamps.is_empty() {
        for at in lit_squares(map, state) {
            if let Tile::Sensor(group) = map_tile(map, at)
                && let Some(flag) = flags.get_mut(group as usize)
            {
                *flag = true;
            }
        }
    }
    // A tower holds its gate open for exactly as long as a beam reaches it,
    // which is why the light is traced here rather than remembered anywhere.
    if lit_board(map) {
        let field = beam_field(map, state);
        for (group, powered) in field.powered.iter().enumerate() {
            if *powered && let Some(flag) = flags.get_mut(group) {
                *flag = true;
            }
        }
    }
    flags
}

pub fn pressed(map: &Map, state: &MapState, at: Position) -> bool {
    (map.rules.plates_sense_player && state.members.contains(&at))
        || (map.rules.plates_sense_crates && covered(state, at))
}

pub fn map_solved(map: &Map, state: &MapState) -> bool {
    match map.rules.win {
        WinCondition::GoalsCovered => {
            !map.goals.is_empty() && map.goals.iter().all(|goal| covered(state, *goal))
        }
        WinCondition::CratesOnGoals => {
            let survivors: Vec<&CrateState> =
                state.crates.iter().filter(|entry| !entry.sunk).collect();
            !survivors.is_empty() && survivors.iter().all(|entry| map.goals.contains(&entry.at))
        }
        WinCondition::SocketsFilled => {
            !map.sockets.is_empty()
                && map
                    .sockets
                    .iter()
                    .all(|(at, _)| seated_at(state, *at).is_some())
        }
    }
}

/// Which gem is seated in the socket on this square, if any.
pub fn seated_at(state: &MapState, at: Position) -> Option<usize> {
    state
        .gems
        .iter()
        .position(|spot| *spot == GemSpot::Seated(at))
}

/// Which gem this member is carrying. One pair of hands, one gem, which is what
/// makes putting one down to pick another up a move worth planning.
pub fn carried_by(state: &MapState, member: usize) -> Option<usize> {
    state
        .gems
        .iter()
        .position(|spot| *spot == GemSpot::Held(member))
}

pub fn covered(state: &MapState, at: Position) -> bool {
    state
        .crates
        .iter()
        .any(|entry| !entry.sunk && entry.at == at)
}

pub fn goals_covered(map: &Map, state: &MapState) -> usize {
    map.goals
        .iter()
        .filter(|goal| covered(state, **goal))
        .count()
}

fn crate_at(state: &MapState, at: Position) -> Option<usize> {
    state
        .crates
        .iter()
        .position(|entry| !entry.sunk && entry.at == at)
}

fn pit_filled(map: &Map, state: &MapState, at: Position) -> bool {
    map.rules.filled_pits_are_floor && state.pits_filled.contains(&at)
}

fn swallows(map: &Map, actor: Actor) -> bool {
    map.rules.pits_swallow_crates && matches!(actor, Actor::Crate(_))
}

fn slides_on_ice(map: &Map, actor: Actor) -> bool {
    match actor {
        Actor::Player => map.rules.ice_slides_player,
        Actor::Crate(_) => map.rules.ice_slides_crates,
    }
}

fn carried_by_portal(map: &Map, actor: Actor) -> bool {
    match actor {
        Actor::Player => map.rules.portals_carry_player,
        Actor::Crate(_) => map.rules.portals_carry_crates,
    }
}

fn carried_by_conveyor(map: &Map, actor: Actor) -> bool {
    match actor {
        Actor::Player => map.rules.conveyors_carry_player,
        Actor::Crate(_) => map.rules.conveyors_carry_crates,
    }
}

fn stopped_by_one_way(map: &Map, actor: Actor) -> bool {
    match actor {
        Actor::Player => map.rules.one_way_stops_player,
        Actor::Crate(_) => map.rules.one_way_stops_crates,
    }
}

fn collapsed(map: &Map, state: &MapState, at: Position) -> bool {
    map.rules.fragile_floor_collapses && state.collapsed.contains(&at)
}

fn broken_open(map: &Map, state: &MapState, at: Position) -> bool {
    map.rules.crates_break_brittle && state.broken.contains(&at)
}

fn breaks_through(map: &Map, actor: Actor) -> bool {
    map.rules.crates_break_brittle && matches!(actor, Actor::Crate(_))
}

fn drowns(map: &Map, actor: Actor) -> bool {
    map.rules.crates_sink_in_water && matches!(actor, Actor::Crate(_))
}

/// Whether this square would eat a crate arriving right now. A pit and a
/// dropped square swallow it into the floor, and a brittle wall spends it on
/// the way through. All three leave something anything can cross afterwards, so
/// they answer one question here rather than three at every call site.
fn swallowing_hole(map: &Map, state: &MapState, at: Position, actor: Actor) -> bool {
    match map_tile(map, at) {
        Tile::Pit => !pit_filled(map, state, at),
        Tile::Fragile => collapsed(map, state, at) && !pit_filled(map, state, at),
        Tile::Brittle => map.rules.crates_break_brittle && !broken_open(map, state, at),
        Tile::Water => map.rules.crates_sink_in_water,
        // A burner holds under a body and gives under a crate, so unlike every
        // other hole on the board it has to be asked who is standing on it.
        Tile::Incinerator => map.rules.incinerators_burn_crates && matches!(actor, Actor::Crate(_)),
        _ => false,
    }
}

/// Where the record of a spent crate belongs. A broken wall is not a filled
/// pit, and only one of them turns back into a square you can stand on.
fn record_spent(map: &Map, state: &mut MapState, at: Position) {
    // Water eats a crate and is exactly as wet afterwards. Which squares keep
    // a record is the schema's answer, so a new one that keeps none cannot end
    // up filed under filled pits.
    let tile = map_tile(map, at);
    if !tile.records_spending() {
        return;
    }
    match tile {
        Tile::Brittle => state.broken.push(at),
        _ => state.pits_filled.push(at),
    }
}

fn can_enter(
    map: &Map,
    state: &MapState,
    at: Position,
    gates: &[bool; GATE_GROUPS],
    actor: Actor,
    heading: Direction,
) -> bool {
    let passable = match map_tile(map, at) {
        // Anyone may walk into water. Whether they come out of it is a
        // question for the check that decides who the board has killed, which
        // is where the beams are answered too.
        Tile::Water => drowns(map, actor) || matches!(actor, Actor::Player),
        // The machines stand on their squares. Light turns at a mirror, is
        // stained at a lens, is cut at a wedge, leaves an emitter and ends at a
        // tower, and nothing else shares any of them.
        Tile::Mirror(_)
        | Tile::Emitter(_)
        | Tile::Receiver(_)
        | Tile::Prism(_)
        | Tile::Splitter => false,
        // A pane is a wall to everything that is not light.
        Tile::Glass => false,
        Tile::Void | Tile::Wall => false,
        Tile::Gate(group) => gates.get(group as usize).copied().unwrap_or(false),
        // A shutter is a gate read backwards: powered is shut.
        Tile::Shutter(group) => !gates.get(group as usize).copied().unwrap_or(false),
        Tile::Lock(colour) => holds_colour(map, state, colour),
        Tile::OneWay(way) => way == heading || !stopped_by_one_way(map, actor),
        Tile::Brittle => broken_open(map, state, at) || breaks_through(map, actor),
        Tile::Fragile if collapsed(map, state, at) => {
            pit_filled(map, state, at) || swallows(map, actor)
        }
        Tile::Pit => pit_filled(map, state, at) || swallows(map, actor),
        _ => true,
    };
    if !passable {
        return false;
    }
    // A watcher is as solid as anything else standing on a square, and unlike
    // a crate there is no shoving it out of the way.
    if state.watchers.contains(&at) {
        return false;
    }
    if let Some(index) = crate_at(state, at) {
        return matches!(actor, Actor::Crate(moving) if moving == index);
    }
    // A member of the party standing somewhere is as solid as a crate, except
    // to itself, because the one being played is leaving the square it is on.
    !state.members.iter().enumerate().any(|(index, member)| {
        *member == at && !(matches!(actor, Actor::Player) && index == state.active)
    })
}

/// Where something ends up sliding this way, written into a run the caller
/// owns so a search that makes millions of these does not make a list for each
/// one. The answer is whether it fell in on the way.
fn slide(
    map: &Map,
    state: &MapState,
    start: Position,
    direction: Direction,
    gates: &[bool; GATE_GROUPS],
    actor: Actor,
    path: &mut Vec<Position>,
) -> bool {
    let mut heading = direction;
    path.clear();
    // A pad's momentum, or a belt's, can carry a trip back onto a square that
    // already redirected it. Spending each of those once bounds the loop:
    // everything else here moves one way along a finite board.
    let mut spent: Vec<Position> = Vec::new();
    let mut at = start;
    loop {
        let delta = heading.delta();
        let next = at.offset(delta);
        if !can_enter(map, state, next, gates, actor, heading) {
            break;
        }
        at = next;
        path.push(at);
        if swallowing_hole(map, state, at, actor) {
            return true;
        }
        match map_tile(map, at) {
            Tile::Portal if carried_by_portal(map, actor) => {
                if spent.contains(&at) {
                    break;
                }
                spent.push(at);
                let Some(exit) = map_teleport_exit(map, at) else {
                    break;
                };
                if !can_enter(map, state, exit, gates, actor, heading) {
                    break;
                }
                at = exit;
                path.push(at);
                if swallowing_hole(map, state, at, actor) {
                    return true;
                }
                if map.rules.portal_exit_continues_on_ice
                    && map_tile(map, at.offset(delta)) == Tile::Ice
                    && slides_on_ice(map, actor)
                {
                    continue;
                }
                break;
            }
            Tile::Conveyor(way) if carried_by_conveyor(map, actor) => {
                if spent.contains(&at) {
                    break;
                }
                spent.push(at);
                heading = way;
                continue;
            }
            Tile::Elevator if matches!(actor, Actor::Crate(_)) => {
                let Some(target) = map_elevator_drop(map, at) else {
                    break;
                };
                if !can_enter(map, state, target, gates, actor, heading) {
                    break;
                }
                at = target;
                path.push(at);
                if swallowing_hole(map, state, at, actor) {
                    return true;
                }
                break;
            }
            Tile::Ice if slides_on_ice(map, actor) => continue,
            _ => break,
        }
    }
    false
}

/// Where a crate shoved this way would come to rest on a board with nothing
/// else on it, and every square it crosses getting there. Another crate in the
/// way can only stop it sooner, so this run is every square one push could
/// leave it on, which is what the solver's reading of the board is built from.
///
/// The run is written into a list the caller owns, because the reading asks
/// this of every square on the board in every direction and would otherwise
/// build a list for each of them.
pub fn crate_run(map: &Map, from: Position, direction: Direction, path: &mut Vec<Position>) {
    let empty = MapState::default();
    slide(
        map,
        &empty,
        from,
        direction,
        &[true; GATE_GROUPS],
        Actor::Crate(0),
        path,
    );
}

/// Every square something touched on its way through, which is what a fragile
/// floor and a switch each react to. The square it came to rest on is the one
/// exception for collapsing, because nothing has left it yet.
fn vacated(from: Position, path: &[Position]) -> impl Iterator<Item = Position> + '_ {
    std::iter::once(from).chain(path.iter().copied().take(path.len().saturating_sub(1)))
}

pub fn attempt_move(map: &Map, state: &MapState, direction: Direction) -> Option<MoveOutcome> {
    attempt_move_with(
        map,
        state,
        direction,
        &gate_flags(map, state),
        active_abilities(map, state),
        Detail::Full,
    )
}

/// The same move, told what the gates are doing. Working that out means tracing
/// every lamp and every beam on the board, and a position has one answer to it
/// however many moves are tried from there.
pub fn attempt_move_with(
    map: &Map,
    state: &MapState,
    direction: Direction,
    gates: &[bool; GATE_GROUPS],
    powers: Abilities,
    detail: Detail,
) -> Option<MoveOutcome> {
    let target = state.player().offset(direction.delta());

    // Trading places reaches past everything between, so it is asked before
    // anything about the square directly ahead.
    if powers.swap
        && let Some(outcome) = attempt_swap(map, state, direction, gates, detail)
    {
        return Some(outcome);
    }

    // A magnet takes what is behind it with it, but only when there is nothing
    // in front to shove. One pair of hands, one crate at a time.
    if powers.magnetic
        && crate_at(state, target).is_none()
        && let Some(outcome) = attempt_pull_with(map, state, direction, gates, powers, detail)
    {
        return Some(outcome);
    }

    let keeps_paths = detail.keeps_paths();
    let pushed = crate_at(state, target);
    let mut crate_moves = Vec::new();
    let mut sunk = Vec::new();
    // One run, handed to each slide in turn and emptied into the answer only
    // when somebody is going to read it.
    let mut trail = Vec::new();
    let mut next_state;

    if let Some(index) = pushed {
        // A boulder is the one thing on the board no shove moves. Walking into
        // one is an attempt to break it rather than an attempt to push it, and
        // the two are different moves with different answers.
        if !state.crates[index].kind.shoves() {
            return attempt_break(map, state, direction, index, gates, powers, detail);
        }
        // Only one character shoves. For the others a crate ahead is simply
        // something in the way, which is the whole of what makes them play
        // differently rather than play the same with an extra key.
        if !powers.push {
            return None;
        }
        // The run in front of the shove, nearest first. Only the head of it is
        // free to slide, and the ones behind are shoved into the space it
        // leaves, so they are moved from the front backwards.
        let mut run = [index, index];
        let mut run_length = 1;
        if map.rules.crates_push_in_pairs {
            let behind = target.offset(direction.delta());
            if let Some(second) = crate_at(state, behind)
                && state.crates[second].kind.shoves()
            {
                run[1] = second;
                run_length = 2;
            }
        }

        next_state = state.clone();
        for moving in run[..run_length].iter().rev().copied() {
            let from = state.crates[moving].at;
            let fell = slide(
                map,
                &next_state,
                from,
                direction,
                gates,
                Actor::Crate(moving),
                &mut trail,
            );
            let destination = trail.last().copied()?;
            next_state.crates[moving].at = destination;
            if fell {
                next_state.crates[moving].sunk = true;
                record_spent(map, &mut next_state, destination);
                if keeps_paths {
                    sunk.push(moving);
                }
            }
            if keeps_paths {
                crate_moves.push((moving, std::mem::take(&mut trail)));
            }
        }
        next_state.pushes += 1;
    } else {
        if !can_enter(map, state, target, gates, Actor::Player, direction) {
            // A wall or a gap ahead is not always a stop. Whoever can stride
            // over one lands beyond it, and where that is comes from the same
            // reading of the board the graph uses.
            if let Some(outcome) = attempt_stride(map, state, direction, gates, powers, detail) {
                return Some(outcome);
            }
            return None;
        }
        next_state = state.clone();
    }

    slide(
        map,
        &next_state,
        state.player(),
        direction,
        gates,
        Actor::Player,
        &mut trail,
    );
    let destination = trail.last().copied()?;
    next_state.set_player(destination);
    next_state.facing = direction;
    next_state.moves += 1;

    let player_path = if keeps_paths { trail } else { Vec::new() };
    apply_square_effects(map, state, &mut next_state, &crate_moves, &player_path);

    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path,
        crate_moves,
        sunk,
    })
}

/// What the squares themselves do about a move that has already been resolved:
/// fragile floor drops behind whatever left it, and switches answer to
/// everything that crossed them. Shoving and dragging both end up here, so a
/// pulled crate trips the same squares a pushed one does.
fn apply_square_effects(
    map: &Map,
    before: &MapState,
    next_state: &mut MapState,
    crate_moves: &[(usize, Vec<Position>)],
    player_path: &[Position],
) {
    if map.rules.fragile_floor_collapses {
        let mut dropped: Vec<Position> = Vec::new();
        for (index, path) in crate_moves {
            dropped.extend(vacated(before.crates[*index].at, path));
        }
        dropped.extend(vacated(before.player(), player_path));
        for at in dropped {
            // A square only goes once it is genuinely empty. Something that
            // slid over it and came to rest on it is still holding it up.
            // Anyone standing on it holds it up, not only the one being played.
            let empty = !next_state.members.contains(&at) && !covered(next_state, at);
            if map_tile(map, at) == Tile::Fragile && empty && !next_state.collapsed.contains(&at) {
                next_state.collapsed.push(at);
            }
        }
    }

    if map.rules.switches_toggle_gates {
        for (_, path) in crate_moves {
            throw_switches(map, next_state, path);
        }
        throw_switches(map, next_state, player_path);
    }
}

/// Breaking a boulder. It is not shoved anywhere, so nothing about where it
/// could go matters: it goes, and the square it stood on is clear enough to
/// step onto in the same move.
fn attempt_break(
    map: &Map,
    state: &MapState,
    direction: Direction,
    index: usize,
    gates: &[bool; GATE_GROUPS],
    powers: Abilities,
    detail: Detail,
) -> Option<MoveOutcome> {
    if !powers.smashes || !map.rules.stones_break_bare_handed {
        return None;
    }
    let mut next_state = state.clone();
    next_state.crates[index].sunk = true;
    next_state.pushes += 1;

    let mut trail = Vec::new();
    slide(
        map,
        &next_state,
        state.player(),
        direction,
        gates,
        Actor::Player,
        &mut trail,
    );
    let destination = trail.last().copied()?;
    next_state.set_player(destination);
    next_state.facing = direction;
    next_state.moves += 1;

    let player_path = if detail.keeps_paths() {
        trail
    } else {
        Vec::new()
    };
    // The boulder is handed back as something that moved so it can be shown
    // going, and left out of what the squares react to, because a square under
    // a boulder that was broken where it stood was never stepped off.
    let (crate_moves, sunk) = if detail.keeps_paths() {
        (vec![(index, vec![state.crates[index].at])], vec![index])
    } else {
        (Vec::new(), Vec::new())
    };
    apply_square_effects(map, state, &mut next_state, &[], &player_path);

    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path,
        crate_moves,
        sunk,
    })
}

/// Crossing a gap or stepping through a wall. Both are the board's own shape
/// rather than anything standing on it, so where they land is worked out by the
/// schema and this only has to check that the landing square is free and make
/// the move.
fn attempt_stride(
    map: &Map,
    state: &MapState,
    direction: Direction,
    gates: &[bool; GATE_GROUPS],
    powers: Abilities,
    detail: Detail,
) -> Option<MoveOutcome> {
    let landing = map_step(map, state.player(), direction, powers)?;
    if !can_enter(map, state, landing, gates, Actor::Player, direction) {
        return None;
    }
    let mut next_state = state.clone();
    next_state.set_player(landing);
    next_state.facing = direction;
    next_state.moves += 1;
    let player_path = if detail.keeps_paths() {
        vec![landing]
    } else {
        Vec::new()
    };
    apply_square_effects(map, state, &mut next_state, &[], &player_path);
    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path,
        crate_moves: Vec::new(),
        sunk: Vec::new(),
    })
}

/// Lifting a gem, putting one down, or seating one in the socket underfoot.
/// All three are one step because no square ever offers two of them: hands full
/// is a put down or a seat, hands empty is a lift, and everywhere else there is
/// nothing to do at all.
///
/// Nothing walks anywhere, so this has no path. What it changes is where a gem
/// is, and on a board with sockets that is a change to where the light goes,
/// which is the whole of why it is a move rather than a convenience.
pub fn attempt_handle(map: &Map, state: &MapState) -> Option<MoveOutcome> {
    if map.gems.is_empty() {
        return None;
    }
    let at = state.player();
    let mut next_state = state.clone();
    match carried_by(state, state.active) {
        Some(index) => {
            // One gem to a square. Two on one square would be two things to
            // pick up and no way to say which.
            if state
                .gems
                .iter()
                .enumerate()
                .any(|(other, spot)| other != index && spot.square() == Some(at))
            {
                return None;
            }
            next_state.gems[index] = match map_tile(map, at) {
                Tile::Socket(_) => GemSpot::Seated(at),
                // Water carries it off and a burner takes it, and a gem this
                // board can no longer produce is a board that cannot be
                // finished by a route nobody can see, so neither is allowed.
                Tile::Water | Tile::Incinerator => return None,
                _ => GemSpot::Loose(at),
            };
        }
        None => {
            let index = state
                .gems
                .iter()
                .position(|spot| spot.square() == Some(at))?;
            next_state.gems[index] = GemSpot::Held(state.active);
        }
    }
    next_state.moves += 1;
    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path: Vec::new(),
        crate_moves: Vec::new(),
        sunk: Vec::new(),
    })
}

/// Pointing the controls at another member. Nothing on the board moves, which
/// is why it has no path and only a state.
pub fn attempt_take(map: &Map, state: &MapState, index: usize) -> Option<MoveOutcome> {
    if index == state.active || index >= state.members.len() || index >= map.party_size() {
        return None;
    }
    let mut next_state = state.clone();
    next_state.active = index;
    next_state.moves += 1;
    Some(MoveOutcome {
        state: next_state,
        mover: index,
        player_path: Vec::new(),
        crate_moves: Vec::new(),
        sunk: Vec::new(),
    })
}

/// Trading places with the first crate down a line. Nothing between the two is
/// disturbed and nothing is shoved, so this is the only way a crate travels
/// towards the one moving it.
fn attempt_swap(
    map: &Map,
    state: &MapState,
    direction: Direction,
    gates: &[bool; GATE_GROUPS],
    detail: Detail,
) -> Option<MoveOutcome> {
    let delta = direction.delta();
    let mut at = state.player();
    loop {
        let next = at.offset(delta);
        // A watcher trades like anything else, and unlike anything else it is
        // the only way one of them ever moves. The reach starts two squares off,
        // because standing beside one is not a position anybody is alive in.
        if let Some(index) = state.watchers.iter().position(|watcher| *watcher == next) {
            let mut next_state = state.clone();
            next_state.watchers[index] = state.player();
            next_state.set_player(next);
            next_state.facing = direction;
            next_state.moves += 1;
            let player_path = if detail.keeps_paths() {
                vec![next]
            } else {
                Vec::new()
            };
            apply_square_effects(map, state, &mut next_state, &[], &player_path);
            return Some(MoveOutcome {
                state: next_state,
                mover: state.active,
                player_path,
                crate_moves: Vec::new(),
                sunk: Vec::new(),
            });
        }
        if let Some(index) = crate_at(state, next) {
            // A boulder is as good as a wall to a trade, since a trade is still
            // a way of moving something and nothing moves a boulder.
            if !state.crates[index].kind.shoves() {
                return None;
            }
            let mut next_state = state.clone();
            next_state.set_player(next);
            next_state.crates[index].at = state.player();
            next_state.facing = direction;
            next_state.moves += 1;
            next_state.pushes += 1;
            let (crate_moves, player_path) = if detail.keeps_paths() {
                (vec![(index, vec![state.player()])], vec![next])
            } else {
                (Vec::new(), Vec::new())
            };
            apply_square_effects(map, state, &mut next_state, &crate_moves, &player_path);
            return Some(MoveOutcome {
                state: next_state,
                mover: state.active,
                player_path,
                crate_moves,
                sunk: Vec::new(),
            });
        }
        // The reach stops at the first thing that is not a crate, so a swapper
        // cannot trade through a wall any more than it can walk through one.
        if !can_enter(map, state, next, gates, Actor::Player, direction) {
            return None;
        }
        at = next;
    }
}

/// Dragging a crate instead of shoving it. The player steps one square the way
/// they asked and the crate behind them comes with, which is the move ordinary
/// Sokoban withholds. A pull is a braced step, so neither of them slides.
pub fn attempt_pull(map: &Map, state: &MapState, direction: Direction) -> Option<MoveOutcome> {
    attempt_pull_with(
        map,
        state,
        direction,
        &gate_flags(map, state),
        active_abilities(map, state),
        Detail::Full,
    )
}

fn attempt_pull_with(
    map: &Map,
    state: &MapState,
    direction: Direction,
    gates: &[bool; GATE_GROUPS],
    powers: Abilities,
    detail: Detail,
) -> Option<MoveOutcome> {
    if !powers.pull {
        return None;
    }
    let delta = direction.delta();
    let target = state.player().offset(delta);
    if !can_enter(map, state, target, gates, Actor::Player, direction) {
        return None;
    }

    let behind = state.player().offset((-delta.0, -delta.1));
    let index = crate_at(state, behind)?;
    // A boulder is dragged as readily as it is shoved, which is to say not at
    // all.
    if !state.crates[index].kind.shoves() {
        return None;
    }

    let mut next_state = state.clone();
    next_state.set_player(target);
    next_state.facing = direction;
    next_state.crates[index].at = state.player();
    next_state.moves += 1;
    next_state.pushes += 1;

    // The crate lands where the player was standing, which they could stand on,
    // so it needs no test of its own beyond the one the player already passed.
    let (crate_moves, player_path) = if detail.keeps_paths() {
        (vec![(index, vec![state.player()])], vec![target])
    } else {
        (Vec::new(), Vec::new())
    };
    apply_square_effects(map, state, &mut next_state, &crate_moves, &player_path);

    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path,
        crate_moves,
        sunk: Vec::new(),
    })
}

/// A switch answers to anything crossing it, so passing over one twice in a
/// single slide flips it twice and leaves it as it was.
fn throw_switches(map: &Map, state: &mut MapState, path: &[Position]) {
    for at in path {
        if let Tile::Switch(group) = map_tile(map, *at)
            && let Some(flag) = state.latched.get_mut(group as usize)
        {
            *flag = !*flag;
        }
    }
}

/// A crate nothing on this board can ever move again. Only permanent walls
/// count, because a gate might open, a crate might move, and a brittle wall
/// might be broken through, so none of those pins anything.
///
/// What counts as pinned is a question about the party rather than about the
/// square. A shove needs the two axes closed, since a crate against a wall
/// above and a wall to the left has nowhere left to be shoved. A drag needs
/// somewhere for the crate to go and somewhere beyond it for the dragger to
/// stand, so every direction has to fail one of those. A trade reaches down a
/// clear line, so one open neighbour is enough to lift the crate out. Water is
/// a wall to a crate and a road to whoever wades, so it only closes an axis
/// while nobody in the party can stand in it.
fn stranded(map: &Map, at: Position, abilities: Abilities) -> bool {
    let blocked = |delta: (i32, i32)| {
        let tile = map_tile(map, at.offset(delta));
        tile.blocks_forever() && !(abilities.wades && tile == Tile::Water)
    };
    if !((blocked((0, -1)) || blocked((0, 1))) && (blocked((-1, 0)) || blocked((1, 0)))) {
        return false;
    }
    if abilities.swap && Direction::ALL.iter().any(|way| !blocked(way.delta())) {
        return false;
    }
    if abilities.pull
        && Direction::ALL.iter().any(|way| {
            let delta = way.delta();
            !blocked(delta) && !blocked((delta.0 * 2, delta.1 * 2))
        })
    {
        return false;
    }
    true
}

/// Whether this branch can still be won. A stranded crate off a goal is not by
/// itself the end, because a map may carry spares to spend on pits or plates.
/// What ends it is running out of crates that could still reach a goal.
pub fn deadlocked(map: &Map, state: &MapState) -> bool {
    // A board won by seating gems is owed nothing at all by its crates, so
    // counting them against the markers would end a search that was going
    // perfectly well.
    if !map.rules.win.wants_crates() {
        return false;
    }
    // What the party can do decides what pins a crate, and any member having a
    // hand for it is enough, since the party is one player with several sets of
    // them and pointing the controls at the one who can is a move like any
    // other. The light counts too, because a power somebody can go and stand in
    // is a power they have.
    let abilities = map.latent_abilities();
    // A boulder is never worked out of the corner it is in. It is taken off the
    // board where it stands, so a pair of hands that could break it is a pair of
    // hands that answers this before the square does.
    let smashable = map.rules.stones_break_bare_handed && abilities.smashes;
    let lost = |entry: &CrateState| {
        !entry.sunk
            && !map.goals.contains(&entry.at)
            && !(smashable && entry.kind == CrateKind::Stone)
            && stranded(map, entry.at, abilities)
    };
    match map.rules.win {
        WinCondition::CratesOnGoals => state.crates.iter().any(lost),
        WinCondition::GoalsCovered => {
            let usable = state
                .crates
                .iter()
                .filter(|entry| !entry.sunk && !lost(entry))
                .count();
            usable < map.goals.len()
        }
        WinCondition::SocketsFilled => false,
    }
}

/// Riding an elevator is a move like any other. It costs a move, it goes on
/// the undo stack, and the solver searches it alongside the four directions.
pub fn attempt_ride(map: &Map, state: &MapState, direction: i32) -> Option<MoveOutcome> {
    if !map.rules.elevators_move_player {
        return None;
    }
    let target = map_elevator_target(map, state.player(), direction)?;
    if crate_at(state, target).is_some() {
        return None;
    }
    let mut next_state = state.clone();
    next_state.set_player(target);
    next_state.moves += 1;
    Some(MoveOutcome {
        state: next_state,
        mover: state.active,
        player_path: vec![target],
        crate_moves: Vec::new(),
        sunk: Vec::new(),
    })
}

/// Which way the elevator under the player can go, for the prompt over their
/// head and for the input layer that acts on it.
pub fn elevator_options(map: &Map, state: &MapState) -> (bool, bool) {
    if !map.rules.elevators_move_player {
        return (false, false);
    }
    (
        map_elevator_target(map, state.player(), -1).is_some(),
        map_elevator_target(map, state.player(), 1).is_some(),
    )
}
