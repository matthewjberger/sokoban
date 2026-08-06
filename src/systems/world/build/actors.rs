//! The moving half of a map: the crates, the player and their parts, the goal
//! markers they are aimed at, the prompt over the player's head, and the lights
//! that show all of it.

use crate::ecs::{Facing, GemVisual, GoalMarker, Part, SokobanResources, TileMotion};
use crate::palette::{
    LAMP_BODY, ORB_BODY, Palette, STONE_BODY, WATCHER_BODY, WATCHER_EYE, WATCHER_GLARE,
    character_body, gem_body, gem_light,
};
use crate::rules::GemSpot;
use crate::schema::{Character, CrateKind, Position, map_layer_bounds, map_layers};
use crate::systems::world::build::{
    CRATE_Y, GEM_SEATED_Y, GEM_Y, NAME_Y, PLAYER_Y, PROMPT_Y, WATCHER_Y, block, chrome, glowing,
    lacquered, layer_height, machined, solid, track_entity, world_position,
};
use nightshade::prelude::*;

/// How far the room's own lighting stands back on a board where light is part
/// of the puzzle, so a lamp reads as a light rather than as a warm patch on an
/// already bright floor.
const LIT_BOARD_FILL: f32 = 0.35;

/// An emitter entity, which the engine's own particle pass then runs.
pub fn spawn_emitter(world: &mut World, at: Vec3, emitter: ParticleEmitter) -> Entity {
    let entity = spawn_entities(
        world,
        PARTICLE_EMITTER | LOCAL_TRANSFORM | GLOBAL_TRANSFORM,
        1,
    )[0];
    setup_entity_transforms(world, entity, LocalTransform::from_translation(at));
    world.set(entity, emitter);
    entity
}

pub fn build_goals(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    let goals = game.map.goals.clone();
    for (index, at) in goals.into_iter().enumerate() {
        let base = world_position(at, 0.035);
        let marker = block(
            world,
            "Cylinder",
            base,
            Vec3::new(0.98, 0.07, 0.98),
            format!("sokoban_goal_{index}"),
            glowing(palette.goal, [0.1, 0.42, 0.38], 0.35),
        );
        world.set(
            marker,
            GoalMarker {
                index,
                base,
                glow: 0.0,
            },
        );
        track_entity(game, world, marker, at.layer);
        game.entities.goal_markers.push(marker);
        game.entities.goal_covered.push(false);
    }
}

pub fn build_crates(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    let starts: Vec<(Position, CrateKind)> = game
        .state
        .crates
        .iter()
        .map(|entry| (entry.at, entry.kind))
        .collect();
    for (index, (at, kind)) in starts.into_iter().enumerate() {
        let orb = kind == CrateKind::Orb;
        let lamp = kind == CrateKind::Lamp;
        let round = orb || lamp || kind == CrateKind::Stone;
        let position = world_position(at, CRATE_Y);
        let body = block(
            world,
            if round { "Sphere" } else { "Cube" },
            position,
            match kind {
                // The sphere primitive is built at radius one, so it comes out
                // twice the size it is scaled to. Everything round that has to
                // sit inside its own square is scaled for that.
                CrateKind::Orb => Vec3::new(0.39, 0.39, 0.39),
                CrateKind::Lamp => Vec3::new(0.27, 0.27, 0.27),
                // Wider than it is tall, so a boulder reads as something
                // squatting on the square rather than something set on it.
                CrateKind::Stone => Vec3::new(0.46, 0.37, 0.45),
                // A pallet with a mirror standing on it, so the plate that does
                // the work is what the eye lands on.
                CrateKind::Mirror(_) => Vec3::new(0.88, 0.16, 0.88),
                CrateKind::Box => Vec3::new(0.84, 0.84, 0.84),
            },
            format!("sokoban_crate_{index}"),
            match kind {
                CrateKind::Orb => chrome(ORB_BODY),
                CrateKind::Lamp => glowing(LAMP_BODY, [2.4, 2.0, 1.2], 0.2),
                CrateKind::Stone => solid(STONE_BODY, 0.95, 0.0),
                CrateKind::Mirror(_) => machined(palette.wall_cap, [0.03, 0.03, 0.04]),
                CrateKind::Box => lacquered(palette.crate_body, 0.62, 0.45),
            },
        );
        world.set(body, TileMotion::default());
        track_entity(game, world, body, at.layer);

        // The light a lamp carries is a real one, placed where the rules say
        // the lamp is, so the shadows on screen fall where the rules put them.
        if lamp {
            let light = spawn_light_entity(world, world_position(at, 0.9), "SokobanLamp");
            world.set(
                light,
                Light {
                    light_type: LightType::Point,
                    color: Vec3::new(1.0, 0.86, 0.6),
                    intensity: 30.0,
                    // The light reaches exactly as far as the rules say it
                    // does, so what is lit on screen is what is lit in the
                    // rules.
                    range: game.map.rules.light_range as f32 + 0.5,
                    cast_shadows: true,
                    ..Default::default()
                },
            );
            world.set(
                light,
                Part {
                    owner: body,
                    offset: Vec3::new(0.0, 0.5, 0.0),
                    pitch: 0.0,
                    follows_rotation: false,
                },
            );
            track_entity(game, world, light, at.layer);

            // Motes drifting off the lamp, which is the cheapest way to say a
            // thing is giving off light rather than painted bright.
            let motes = spawn_emitter(
                world,
                world_position(at, 0.5),
                ParticleEmitter {
                    shape: EmitterShape::Sphere { radius: 0.22 },
                    spawn_rate: 14.0,
                    particle_lifetime_min: 0.8,
                    particle_lifetime_max: 1.6,
                    initial_velocity_min: 0.1,
                    initial_velocity_max: 0.35,
                    velocity_spread: 1.2,
                    gravity: Vec3::new(0.0, 0.35, 0.0),
                    drag: 0.6,
                    size_start: 0.07,
                    size_end: 0.0,
                    color_gradient: ColorGradient::fire(),
                    emissive_strength: 2.0,
                    enabled: true,
                    ..Default::default()
                },
            );
            world.set(
                motes,
                Part {
                    owner: body,
                    offset: Vec3::new(0.0, 0.2, 0.0),
                    pitch: 0.0,
                    follows_rotation: false,
                },
            );
            track_entity(game, world, motes, at.layer);
        }

        // An orb is one polished piece. A crate is banded and capped, which is
        // what makes the two read apart at a glance from overhead.
        let parts = if round {
            [Entity::default(), Entity::default()]
        } else {
            let band = block(
                world,
                "Cube",
                position,
                Vec3::new(0.9, 0.2, 0.9),
                "sokoban_crate_band",
                solid(palette.crate_band, 0.8, 0.0),
            );
            world.set(
                band,
                Part {
                    owner: body,
                    offset: Vec3::new(0.0, 0.0, 0.0),
                    pitch: 0.0,
                    follows_rotation: false,
                },
            );
            track_entity(game, world, band, at.layer);

            let cap = block(
                world,
                "Cube",
                position,
                Vec3::new(0.48, 0.9, 0.48),
                "sokoban_crate_cap",
                lacquered(palette.crate_cap, 0.55, 0.55),
            );
            world.set(
                cap,
                Part {
                    owner: body,
                    offset: Vec3::new(0.0, 0.0, 0.0),
                    pitch: 0.0,
                    follows_rotation: false,
                },
            );
            track_entity(game, world, cap, at.layer);
            [band, cap]
        };

        game.entities.crates.push(body);
        game.entities.crate_parts.push(parts);
        game.entities.crate_covered.push(false);
    }
}

/// One crystal for each gem the map lists. Where it stands is read off the
/// state rather than off the map, because a gem is carried and a board reloaded
/// in the middle of a run has to find its gems where it left them.
pub fn build_gems(game: &mut SokobanResources, world: &mut World) {
    let spots: Vec<GemSpot> = game.state.gems.clone();
    for (index, gem) in game.map.gems.clone().into_iter().enumerate() {
        let at = spots
            .get(index)
            .and_then(|spot| spot.square())
            .unwrap_or(gem.at);
        let color = gem_body(gem.color);
        // A gem the board opens with already seated stands on its plinth from
        // the first frame rather than climbing onto it once the game starts.
        let height = match spots.get(index) {
            Some(GemSpot::Seated(_)) => GEM_SEATED_Y,
            _ => GEM_Y,
        };
        let position = world_position(at, height);
        let body = block(
            world,
            "Cone",
            position,
            Vec3::new(0.44, 0.52, 0.44),
            format!("sokoban_gem_{}", gem.color.label()),
            glowing(color, gem_light(gem.color), 0.15),
        );
        world.set(body, TileMotion::default());
        world.set(
            body,
            GemVisual {
                index,
                phase: (at.cell.0 * 7 + at.cell.1 * 3) as f32,
            },
        );
        track_entity(game, world, body, at.layer);

        let glow = spawn_light_entity(world, position, "SokobanGem");
        world.set(
            glow,
            Light {
                light_type: LightType::Point,
                color: Vec3::new(color[0], color[1], color[2]),
                intensity: 12.0,
                range: 3.5,
                cast_shadows: false,
                ..Default::default()
            },
        );
        world.set(
            glow,
            Part {
                owner: body,
                offset: Vec3::new(0.0, 0.25, 0.0),
                pitch: 0.0,
                follows_rotation: false,
            },
        );
        track_entity(game, world, glow, at.layer);

        game.entities.gems.push(body);
    }
}

/// What each class wears over its head: a mesh, how big, and which way up. The
/// sphere primitive is built at radius one and the torus with it, so the round
/// ones are scaled for that. Turning one over is a half turn rather than a
/// negative scale, because a mirrored mesh is a mesh drawn inside out.
fn crest_of(character: Character) -> (&'static str, Vec3, f32) {
    match character {
        Character::Pusher => ("Cube", Vec3::new(0.17, 0.17, 0.17), 0.0),
        Character::Dragger => ("Torus", Vec3::new(0.1, 0.1, 0.1), 0.0),
        Character::Magnet => ("Sphere", Vec3::new(0.1, 0.1, 0.1), 0.0),
        Character::Swapper => ("Cone", Vec3::new(0.22, 0.22, 0.22), 0.0),
        Character::Wader => ("Cylinder", Vec3::new(0.2, 0.14, 0.2), 0.0),
        // Point down, because what this one does is go through things.
        Character::Phaser => ("Cone", Vec3::new(0.22, 0.22, 0.22), std::f32::consts::PI),
        Character::Warden => ("Torus", Vec3::new(0.12, 0.12, 0.12), 0.0),
        Character::Blinker => ("Sphere", Vec3::new(0.08, 0.08, 0.08), 0.0),
        Character::Breaker => ("Cube", Vec3::new(0.22, 0.11, 0.22), 0.0),
    }
}

/// One body for each watcher. They are the only thing on the board that kills
/// by standing still, so they are built to be looked at: a dark post with an
/// eye on top of it, and the squares it watches are drawn by the pass that
/// keeps up with where they are.
pub fn build_watchers(game: &mut SokobanResources, world: &mut World) {
    for at in game.state.watchers.clone() {
        let post = block(
            world,
            "Cylinder",
            world_position(at, WATCHER_Y),
            Vec3::new(0.5, 0.72, 0.5),
            "sokoban_watcher",
            machined(WATCHER_BODY, [0.06, 0.01, 0.02]),
        );
        world.set(post, TileMotion::default());
        track_entity(game, world, post, at.layer);

        let eye = block(
            world,
            "Sphere",
            world_position(at, WATCHER_Y + 0.34),
            Vec3::new(0.16, 0.16, 0.16),
            "sokoban_watcher_eye",
            glowing(WATCHER_EYE, WATCHER_GLARE, 0.1),
        );
        world.set(
            eye,
            Part {
                owner: post,
                offset: Vec3::new(0.0, 0.34, 0.0),
                pitch: 0.0,
                follows_rotation: false,
            },
        );
        track_entity(game, world, eye, at.layer);

        game.entities.watchers.push(post);
    }
}

/// One body for each member of the party, each in the colour of whoever it is.
/// The one being played is the one the controls move, and the rest stand where
/// they were left and are as solid as crates.
pub fn build_player(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    for index in 0..game.map.party_size() {
        build_member(game, palette, world, index);
    }
}

fn build_member(game: &mut SokobanResources, palette: &Palette, world: &mut World, index: usize) {
    let at = game.map.member_start(index);
    let yaw = game.state.facing.yaw();
    let position = world_position(at, PLAYER_Y);
    let character = game.map.member_character(index);

    // A material is its name, so a body named the same as another body is the
    // same body. One name per class, or the party comes out one colour however
    // many different colours were asked for.
    let body = block(
        world,
        "Cylinder",
        position,
        Vec3::new(0.62, 0.58, 0.62),
        format!("sokoban_body_{}", character.label()),
        lacquered(character_body(character), 0.42, 0.7),
    );
    world.set(body, TileMotion::default());
    world.set(
        body,
        Facing {
            current: yaw,
            target: yaw,
        },
    );
    track_entity(game, world, body, at.layer);
    game.entities.members.push(body);
    game.entities.member_glow.push([0.0; 3]);

    let head = block(
        world,
        "Sphere",
        position,
        Vec3::new(0.2, 0.2, 0.2),
        "sokoban_player_head",
        lacquered(palette.player_head, 0.55, 0.25),
    );
    world.set(
        head,
        Part {
            owner: body,
            offset: Vec3::new(0.0, 0.42, 0.0),
            pitch: 0.0,
            follows_rotation: false,
        },
    );
    track_entity(game, world, head, at.layer);

    let visor = block(
        world,
        "Cone",
        position,
        Vec3::new(0.24, 0.3, 0.24),
        "sokoban_player_trim",
        glowing(palette.player_trim, [0.25, 0.25, 0.3], 0.4),
    );
    world.set(
        visor,
        Part {
            owner: body,
            offset: Vec3::new(0.0, 0.4, -0.24),
            pitch: -std::f32::consts::FRAC_PI_2,
            follows_rotation: true,
        },
    );
    track_entity(game, world, visor, at.layer);

    let boots = block(
        world,
        "Cylinder",
        position,
        Vec3::new(0.8, 0.12, 0.8),
        "sokoban_player_boots",
        solid(palette.crate_band, 0.8, 0.0),
    );
    world.set(
        boots,
        Part {
            owner: body,
            offset: Vec3::new(0.0, -0.24, 0.0),
            pitch: 0.0,
            follows_rotation: false,
        },
    );
    track_entity(game, world, boots, at.layer);

    // A crest over the head, in the shape its class wears. Colour tells the
    // party apart at a glance and a silhouette tells them apart when the
    // colours are hard to read, which from overhead they sometimes are.
    let (crest_mesh, crest_scale, crest_pitch) = crest_of(character);
    let crest = block(
        world,
        crest_mesh,
        position,
        crest_scale,
        format!("sokoban_crest_{}", character.label()),
        glowing(
            character_body(character),
            [
                character_body(character)[0] * 0.9,
                character_body(character)[1] * 0.9,
                character_body(character)[2] * 0.9,
            ],
            0.35,
        ),
    );
    world.set(
        crest,
        Part {
            owner: body,
            offset: Vec3::new(0.0, 0.74, 0.0),
            pitch: crest_pitch,
            follows_rotation: false,
        },
    );
    track_entity(game, world, crest, at.layer);

    game.entities
        .member_parts
        .push(vec![head, visor, boots, crest]);
    // Only the first member's parts are followed by the storey retag, which is
    // what that list is for.
    if index == 0 {
        game.entities.player_parts = vec![head, visor, boots, crest];
    }
}

/// The floating call sign that appears over the player's head while they stand
/// on an elevator. It is world space text, so it sits in the scene at the
/// storey it belongs to rather than floating in the interface.
/// The map's name, standing on the board rather than printed over it. It sits
/// off the near edge of the storey the player starts on, so it reads as a sign
/// in the room instead of a caption on the screen.
pub fn build_name(game: &mut SokobanResources, world: &mut World) {
    if game.map.name.is_empty() {
        return;
    }
    let Some((minimum, maximum)) = map_layer_bounds(&game.map, game.map.player.layer) else {
        return;
    };
    let anchor = Vec3::new(
        (minimum.0 + maximum.0) as f32 * 0.5,
        layer_height(game.map.player.layer) + NAME_Y,
        maximum.1 as f32 + 1.35,
    );
    let entity = spawn_3d_billboard_text_with_properties(
        world,
        &game.map.name.to_uppercase(),
        anchor,
        TextProperties {
            font_size: 34.0,
            color: Vec4::new(1.0, 0.86, 0.62, 1.0),
            alignment: TextAlignment::Center,
            vertical_alignment: VerticalAlignment::Middle,
            outline_width: 0.8,
            outline_color: Vec4::new(0.0, 0.0, 0.0, 0.95),
            ..Default::default()
        },
    );
    track_entity(game, world, entity, game.map.player.layer);
}

/// The one burst this board will ever need. Firing it again is moving it and
/// telling it to go, rather than making another.
pub fn build_splash(game: &mut SokobanResources, world: &mut World) {
    let splash = spawn_emitter(
        world,
        world_position(game.map.player, 0.25),
        ParticleEmitter {
            shape: EmitterShape::Sphere { radius: 0.3 },
            spawn_rate: 0.0,
            burst_count: 34,
            particle_lifetime_min: 0.4,
            particle_lifetime_max: 0.9,
            initial_velocity_min: 1.0,
            initial_velocity_max: 2.6,
            velocity_spread: 1.1,
            gravity: Vec3::new(0.0, -6.0, 0.0),
            drag: 0.25,
            size_start: 0.12,
            size_end: 0.0,
            color_gradient: ColorGradient::sparks(),
            emissive_strength: 2.4,
            enabled: false,
            one_shot: true,
            ..Default::default()
        },
    );
    track_entity(game, world, splash, game.map.player.layer);
    game.entities.splash = splash;
}

pub fn build_prompt(game: &mut SokobanResources, world: &mut World) {
    let entity = spawn_3d_billboard_text_with_properties(
        world,
        "",
        world_position(game.map.player, PROMPT_Y),
        TextProperties {
            font_size: 15.0,
            color: Vec4::new(1.0, 0.92, 0.7, 1.0),
            alignment: TextAlignment::Center,
            vertical_alignment: VerticalAlignment::Middle,
            outline_width: 0.6,
            outline_color: Vec4::new(0.0, 0.0, 0.0, 0.95),
            ..Default::default()
        },
    );
    world.set(entity, Visibility { visible: false });
    game.entities.spawned.push(entity);
    game.entities.prompt = entity;
}

pub fn build_lights(game: &mut SokobanResources, palette: &Palette, world: &mut World) {
    // A board where the light is the puzzle is a board whose own lighting has
    // to be worth reading. The sun and the corner fills are what wash a lamp
    // out, so on those boards they stand back and let it carry the room.
    let dim = if game.map.lamps.is_empty() && game.map.gems.is_empty() {
        1.0
    } else {
        LIT_BOARD_FILL
    };

    let sun = spawn_sun(world);
    if dim < 1.0
        && let Some(light) = world.get_mut::<Light>(sun)
    {
        light.intensity *= dim;
    }
    game.entities.spawned.push(sun);

    for layer in map_layers(&game.map) {
        let Some((minimum, maximum)) = map_layer_bounds(&game.map, layer) else {
            continue;
        };
        let height = layer_height(layer);
        let warm = spawn_light_entity(
            world,
            Vec3::new(minimum.0 as f32 - 1.5, height + 4.5, minimum.1 as f32 - 1.5),
            "SokobanWarmLight",
        );
        world.set(
            warm,
            Light {
                light_type: LightType::Point,
                color: palette.warm_light,
                intensity: 26.0 * dim,
                range: 26.0,
                cast_shadows: false,
                ..Default::default()
            },
        );
        track_entity(game, world, warm, layer);

        let cool = spawn_light_entity(
            world,
            Vec3::new(maximum.0 as f32 + 1.5, height + 4.5, maximum.1 as f32 + 1.5),
            "SokobanCoolLight",
        );
        world.set(
            cool,
            Light {
                light_type: LightType::Point,
                color: palette.cool_light,
                intensity: 22.0 * dim,
                range: 26.0,
                cast_shadows: false,
                ..Default::default()
            },
        );
        track_entity(game, world, cool, layer);
    }
}
