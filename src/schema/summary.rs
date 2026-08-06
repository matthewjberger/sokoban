//! The schema said out loud. A map is a value with a lot in it, and an author
//! deserves to read what they have built without opening the file it saves to.
//! Each line is either a heading or a fact, which is all a reader needs to see
//! the shape of it.

use crate::schema::{Map, Position, Rules, Tile, complexity, map_layers, map_positions, map_tile};

/// One switch on [`Rules`]: what to call it, and how to reach it. Naming them
/// once means the panel that edits them and the view that reads them cannot
/// drift apart.
pub type RuleSwitch = (&'static str, fn(&mut Rules) -> &mut bool);

pub const RULE_SWITCHES: [RuleSwitch; 24] = [
    ("Ice slides the player", |rules| {
        &mut rules.ice_slides_player
    }),
    ("Ice slides crates", |rules| &mut rules.ice_slides_crates),
    ("Pits swallow crates", |rules| {
        &mut rules.pits_swallow_crates
    }),
    ("Filled pits are floor", |rules| {
        &mut rules.filled_pits_are_floor
    }),
    ("Portals carry the player", |rules| {
        &mut rules.portals_carry_player
    }),
    ("Portals carry crates", |rules| {
        &mut rules.portals_carry_crates
    }),
    ("Portal exit keeps sliding", |rules| {
        &mut rules.portal_exit_continues_on_ice
    }),
    ("Plates sense the player", |rules| {
        &mut rules.plates_sense_player
    }),
    ("Plates sense crates", |rules| {
        &mut rules.plates_sense_crates
    }),
    ("Elevators carry the player", |rules| {
        &mut rules.elevators_move_player
    }),
    ("Elevators carry crates", |rules| {
        &mut rules.elevators_move_crates
    }),
    ("One way stops the player", |rules| {
        &mut rules.one_way_stops_player
    }),
    ("One way stops crates", |rules| {
        &mut rules.one_way_stops_crates
    }),
    ("Belts carry the player", |rules| {
        &mut rules.conveyors_carry_player
    }),
    ("Belts carry crates", |rules| {
        &mut rules.conveyors_carry_crates
    }),
    ("Fragile floor collapses", |rules| {
        &mut rules.fragile_floor_collapses
    }),
    ("Switches latch gates", |rules| {
        &mut rules.switches_toggle_gates
    }),
    ("Crates break brittle walls", |rules| {
        &mut rules.crates_break_brittle
    }),
    ("Crates sink in water", |rules| {
        &mut rules.crates_sink_in_water
    }),
    ("One shove moves two crates", |rules| {
        &mut rules.crates_push_in_pairs
    }),
    ("Gem light lends its power", |rules| {
        &mut rules.gem_light_grants_powers
    }),
    ("Boulders break bare handed", |rules| {
        &mut rules.stones_break_bare_handed
    }),
    ("Raised spikes kill", |rules| &mut rules.spikes_impale),
    ("Gratings swallow crates", |rules| {
        &mut rules.incinerators_burn_crates
    }),
];

pub struct SummaryLine {
    pub text: String,
    pub heading: bool,
}

fn heading(text: &str) -> SummaryLine {
    SummaryLine {
        text: text.to_string(),
        heading: true,
    }
}

fn fact(text: String) -> SummaryLine {
    SummaryLine {
        text,
        heading: false,
    }
}

fn cells(positions: &[Position]) -> String {
    if positions.is_empty() {
        return "none".to_string();
    }
    let listed: Vec<String> = positions
        .iter()
        .take(6)
        .map(|at| format!("{},{}", at.cell.0, at.cell.1))
        .collect();
    let extra = positions.len().saturating_sub(listed.len());
    if extra > 0 {
        format!("{} and {extra} more", listed.join("  "))
    } else {
        listed.join("  ")
    }
}

/// A name in one column and its value in the next, which is what makes a wall
/// of facts scannable.
fn entry(name: &str, value: &str) -> SummaryLine {
    fact(format!("{name:<26}{value}"))
}

pub fn summarize(map: &Map) -> Vec<SummaryLine> {
    let mut lines = Vec::new();

    lines.push(heading("IDENTITY"));
    lines.push(entry("name", &map.name));
    if !map.hint.is_empty() {
        lines.push(entry("hint", &map.hint));
    }
    lines.push(entry("skin", map.skin.label()));
    lines.push(entry(
        "character",
        &format!("{}, {}", map.character.label(), map.character.blurb()),
    ));
    lines.push(entry("won by", map.rules.win.label()));
    lines.push(entry("par", &format!("{} moves", map.par)));
    lines.push(entry("complexity", &complexity(map).to_string()));

    lines.push(heading("LATTICE"));
    lines.push(entry(
        "floor size",
        &format!("{} by {}", map.floor_width, map.floor_height),
    ));
    for layer in map_layers(map) {
        let slots: Vec<String> = map
            .floors
            .iter()
            .filter(|floor| floor.slot.layer == layer)
            .map(|floor| format!("{},{}", floor.slot.column, floor.slot.row))
            .collect();
        let name = format!("storey {layer}");
        let value = format!("{} floor(s) at {}", slots.len(), slots.join("  "));
        lines.push(entry(&name, &value));
    }

    lines.push(heading("ON THE BOARD"));
    let player = format!(
        "storey {} at {},{}",
        map.player.layer, map.player.cell.0, map.player.cell.1
    );
    lines.push(entry("player", &player));
    let crates = format!("crates ({})", map.crates.len());
    lines.push(entry(&crates, &cells(&map.crates)));
    let goals = format!("goals ({})", map.goals.len());
    lines.push(entry(&goals, &cells(&map.goals)));
    if !map.stones.is_empty() {
        let stones = format!("boulders ({})", map.stones.len());
        lines.push(entry(&stones, &cells(&map.stones)));
    }
    for gem in &map.gems {
        lines.push(entry(
            &format!("{} gem", gem.color.label().to_lowercase()),
            &format!(
                "{},{} on storey {}",
                gem.at.cell.0, gem.at.cell.1, gem.at.layer
            ),
        ));
    }

    let counted = |wanted: fn(Tile) -> bool| {
        map_positions(map)
            .into_iter()
            .filter(|at| wanted(map_tile(map, *at)))
            .count()
    };
    for (name, count) in [
        ("ice squares", counted(|tile| tile == Tile::Ice)),
        ("pits", counted(|tile| tile == Tile::Pit)),
        ("elevator squares", counted(|tile| tile == Tile::Elevator)),
        ("fragile squares", counted(|tile| tile == Tile::Fragile)),
        (
            "one way squares",
            counted(|tile| matches!(tile, Tile::OneWay(_))),
        ),
        (
            "belt squares",
            counted(|tile| matches!(tile, Tile::Conveyor(_))),
        ),
        ("switches", counted(|tile| matches!(tile, Tile::Switch(_)))),
        ("brittle walls", counted(|tile| tile == Tile::Brittle)),
        ("water squares", counted(|tile| tile == Tile::Water)),
        ("sockets", counted(|tile| matches!(tile, Tile::Socket(_)))),
        ("glass panes", counted(|tile| tile == Tile::Glass)),
        ("lenses", counted(|tile| matches!(tile, Tile::Prism(_)))),
        ("splitters", counted(|tile| tile == Tile::Splitter)),
        ("spike beds", counted(|tile| matches!(tile, Tile::Spike(_)))),
        ("burners", counted(|tile| tile == Tile::Incinerator)),
    ] {
        if count > 0 {
            lines.push(entry(name, &count.to_string()));
        }
    }
    for (index, (first, second)) in map.portals.iter().enumerate() {
        let name = format!("portal {}", index + 1);
        let value = format!(
            "{},{} on {}  to  {},{} on {}",
            first.cell.0, first.cell.1, first.layer, second.cell.0, second.cell.1, second.layer
        );
        lines.push(entry(&name, &value));
    }

    lines.push(heading("RULES"));
    let mut rules = map.rules;
    for (name, access) in RULE_SWITCHES {
        let on = *access(&mut rules);
        lines.push(entry(name, if on { "yes" } else { "no" }));
    }

    lines
}
