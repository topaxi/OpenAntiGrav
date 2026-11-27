//! HD's particle sprites are `.gtf` entries under `/data/psys/tex/`, read for
//! the effects its `Title` lists and for no others - and the emitter extent
//! law, which Pulse on the PSP applies to every effect, is lifted for HD per
//! effect the same way.

use oag_fx::psys::streak::{Kept, KeptBar};
use oag_title::Title;

use crate::assets::particle_effect;

#[test]
fn only_hd_walks_its_flipbooks_and_ramps_its_sizes() {
    let measured = Some(oag_title::Origin::Measured);
    assert_eq!(oag_hd::TITLE.effects.flipbooks, measured);
    assert_eq!(oag_hd::TITLE.effects.size_ramps, measured);
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert_eq!(title.effects.flipbooks, None, "{}", title.name);
        assert_eq!(title.effects.size_ramps, None, "{}", title.name);
    }
}

#[test]
fn only_hd_lists_effects_whose_gtf_sprites_play() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        assert!(title.effects.sprites.is_empty(), "{}", title.name);
    }
    assert_eq!(
        oag_hd::TITLE.effects.sprites,
        [
            oag_title::engine_effects::DETONATOR_CANNON_SPARKS_EFFECT,
            oag_title::engine_effects::TRACK_BLAST_EFFECT,
            oag_title::engine_effects::CRAFT_BLAST_EFFECT,
            oag_title::engine_effects::EXPLOSION_EFFECT,
            oag_title::engine_effects::QUAKE_EFFECT,
            oag_title::engine_effects::DETONATOR_EMP_EFFECT,
            oag_title::engine_effects::MISSILE_FLARE_EFFECT,
            oag_title::engine_effects::TRAIL_HITSHIP_EFFECT,
            oag_title::engine_effects::TRAIL_HITSHIP_RED_EFFECT,
            oag_title::engine_effects::PLASMA_FLARE_EFFECT,
            oag_title::engine_effects::PLASMA_CHARGING_EFFECT,
            oag_title::engine_effects::PLASMA_LAUNCH_EFFECT,
            oag_title::engine_effects::PLASMA_LIGHTNING_EXPAND_EFFECT,
            oag_title::engine_effects::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
            oag_title::engine_effects::MINE_EXPLO_EFFECT,
            oag_title::engine_effects::BOMB_SMOKERING_EFFECT,
            oag_title::engine_effects::BOMB_RAYS_EFFECT,
            oag_title::engine_effects::MISSILE_EXPLO_EFFECT,
            oag_title::engine_effects::MISSILE_BOUNCE_EFFECT,
            oag_title::engine_effects::DETONATOR_MINE_EXPLO_DEAD_EFFECT,
            oag_title::engine_effects::DETONATOR_MINE_EXPLO_EFFECT,
            oag_title::engine_effects::DETONATOR_BOMB_EXPLO_EFFECT,
            oag_title::engine_effects::ROCKET_FLARE_EFFECT,
            oag_title::engine_effects::CANNON_SPARKS_EFFECT,
            oag_title::engine_effects::SHURIKEN_FLARE_EFFECT,
            oag_title::engine_effects::SHURIKEN_TRAIL_EFFECT,
            oag_title::engine_effects::SHURIKEN_BOUNCE_EFFECT,
            oag_title::engine_effects::SHURIKEN_EXPIRE_EFFECT,
            oag_title::engine_effects::REPULSER_BLAST_EFFECT,
            oag_title::engine_effects::ABSORB_EFFECT,
            oag_title::engine_effects::COLLISION_SPARK_EFFECT,
            oag_title::engine_effects::DEATH_PLUME_EFFECT,
            oag_title::engine_effects::DEBRIS_FIRE_EFFECT,
            oag_title::engine_effects::DAMAGE_MILD_EFFECT,
            oag_title::engine_effects::DAMAGE_MODERATE_EFFECT,
            oag_title::engine_effects::MODESTO_STEAM_EFFECT,
        ]
    );
    let names = oag_hd::TITLE.effects.names();
    for listed in oag_hd::TITLE.effects.sprites {
        assert!(names.contains(listed), "{listed} is loaded by the race");
    }
}

/// The extent law is lifted per effect. HD and Pure lift it for the Quake's wave, the one effect
/// a matched capture of the original has shown spreading across the road on each (Pure's
/// `Quake_Update`, `psp-pure-usa/quake.md`); HD also for the Detonator EMP's wave, its blue
/// recolour (`ps3-hdfury-eu/detonator-emp.md`, "What HD draws for a fired EMP"), and the
/// Plasma blast's two lightning effects (`ps3-hdfury-eu/plasma.md`, "The pairs"), the Rocket's
/// craft blast (`docs/rendering/particle-fill.md`, "The trails start on the blast's
/// hemisphere") and the Bomb's smoke ring and rays (`docs/rendering/particle-fill.md`, "The
/// Bomb's and the Missile's explosions paired"). 2048 and Omega list the EMP's wave alone, as
/// HD's does (`detonator-emp.md`, "2048 and Omega draw it").
#[test]
fn the_extent_law_is_lifted_only_for_the_effects_each_title_lists() {
    use oag_title::engine_effects as n;
    assert!(oag_pulse::TITLE.effects.extents.is_empty());
    for title in [oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.effects.extents,
            [n::DETONATOR_EMP_EFFECT],
            "{}",
            title.name
        );
        assert_eq!(
            title.effects.turbulence,
            [n::DETONATOR_EMP_EFFECT],
            "{}",
            title.name
        );
    }
    assert_eq!(oag_pure::TITLE.effects.extents, [n::QUAKE_EFFECT]);
    assert_eq!(
        oag_hd::TITLE.effects.extents,
        [
            n::QUAKE_EFFECT,
            n::DETONATOR_EMP_EFFECT,
            n::PLASMA_LIGHTNING_EXPAND_EFFECT,
            n::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
            n::CRAFT_BLAST_EFFECT,
            n::BOMB_SMOKERING_EFFECT,
            n::BOMB_RAYS_EFFECT,
        ]
    );
    for title in [
        oag_hd::TITLE,
        oag_pure::TITLE,
        oag_2048::TITLE,
        oag_omega::TITLE,
    ] {
        let names = title.effects.names();
        for listed in title.effects.extents {
            assert!(names.contains(listed), "{listed} is loaded by the race");
        }
    }
}

/// Each title's executable reads the colour-mode word its own way (`docs/formats/pob.md`, "The
/// colour table is RGB-only"): HD blends toward the scene light, 2048 and Omega walk backwards.
#[test]
fn each_title_names_how_its_particles_walk_the_colour_table() {
    use oag_title::ColourWalk::{Forward, LightBlend, Reversed};
    for (title, walk) in [
        (oag_pulse::TITLE, Forward),
        (oag_pure::TITLE, Forward),
        (oag_hd::TITLE, LightBlend),
        (oag_2048::TITLE, Reversed),
    ] {
        assert_eq!(title.effects.colour_walk, walk, "{}", title.name);
    }
    // Omega reads by layout: its 2048 set reversed, its HD-era set the plain walk.
    let omega = oag_omega::TITLE.effects;
    assert_eq!(omega.colour_walk_for(r"Data\particles2048"), Reversed);
    assert_eq!(omega.colour_walk_for(r"Data\particles"), Forward);
    assert_eq!(omega.colour_walk_for("data/Particles2048/"), Reversed);
}

/// The loader applies it: HD's ship explosion root, mode 3, blends.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_loader_gives_hds_ship_explosion_the_light_blend() {
    use oag_fx::psys::ColourMode;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = crate::assets::listed_particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        title.effects,
        oag_title::engine_effects::EXPLOSION_EFFECT,
        false,
        title.effects.colour_walk,
        title.effects,
    )
    .expect("the effect loads");
    assert_eq!(effect.emitters[0].authored_colour_mode, 3);
    assert_eq!(effect.emitters[0].colour_mode, ColourMode::LightBlend);
}

fn track_blast(listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::TRACK_BLAST_EFFECT,
        false,
        false,
        listed,
        Kept::NONE,
    )
    .expect("the effect loads");
    Some(effect)
}

/// All nine emitters of HD's own `WO_ROCKET_EXPLO_TRACK` name a `.gtf` that
/// decodes, and the same effect not listed reads none of them.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_rocket_wall_burst_reads_nine_sprites_when_listed_and_none_when_not() {
    let Some(listed) = track_blast(true) else {
        return;
    };
    assert_eq!(listed.emitters.len(), 9);
    assert!(listed.emitters.iter().all(|e| e.sprite.is_some()));
    let unlisted = track_blast(false).expect("same image");
    assert!(unlisted.emitters.iter().all(|e| e.sprite.is_none()));
}

fn quake(extent_law: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::QUAKE_EFFECT,
        false,
        extent_law,
        true,
        Kept::NONE,
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD's own `WO_QUAKE` has five emitters, all five sprites decode, and its line
/// emitters spread across the road only when the title lists the effect: the
/// Quake's driver (`RaceView::advance_quake_visual`) keys its extent feed, frame and
/// severity on `has_extents`.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_quake_wave_spreads_by_the_extent_law_when_listed_and_stays_a_point_when_not() {
    let Some(listed) = quake(true) else {
        return;
    };
    assert_eq!(listed.emitters.len(), 5, "WO_QUAKE's five emitters");
    assert!(listed.emitters.iter().all(|e| e.sprite.is_some()));
    assert!(listed.has_extents());
    let unlisted = quake(false).expect("same image");
    assert!(!unlisted.has_extents());
}

fn missile_head(listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::MISSILE_FLARE_EFFECT,
        false,
        false,
        listed,
        Kept::NONE,
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD's flying Missile plays `WO_MISSILE_HEAD`, whose two head emitters name
/// `vandergraf_balls_1024x512_missile`, an 8 x 4 atlas of red filament rings.
/// Drawn without it every particle is the procedural soft disc, which the
/// emitter's white-hot colour ramp blows out to a white blob; listed, the
/// rings are what a player sees (`docs/ghidra/functions/ps3-hdfury-eu/weapons.md`,
/// "The Missile's red rings").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_missile_head_draws_its_ring_atlas_when_listed_and_a_white_disc_when_not() {
    let Some(listed) = missile_head(true) else {
        return;
    };
    let rings = |effect: &oag_fx::psys::Effect| {
        effect
            .emitters
            .iter()
            .filter(|e| e.name == "WO_MISSILE_HEAD" || e.name == "MISSILE_HEAD_TRANSFORMED")
            .filter(|e| e.sprite.is_some())
            .count()
    };
    assert_eq!(rings(&listed), 2, "both head emitters read the atlas");
    let unlisted = missile_head(false).expect("same image");
    assert_eq!(rings(&unlisted), 0);
}

fn trail_hit_burst(listed: bool) -> Option<oag_fx::psys::Effect> {
    let image = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let (effect, _) = particle_effect(
        &mut archives,
        title.race.effect_dir_for("track"),
        oag_title::engine_effects::TRAIL_HITSHIP_EFFECT,
        false,
        false,
        listed,
        if listed {
            Kept::bar(KeptBar::Far)
        } else {
            Kept::NONE
        },
    )
    .expect("the effect loads");
    Some(effect)
}

/// HD, 2048 and Omega list the trail-hit burst, the Detonator wall spark's three streaks and the
/// EMP's sparks for HD's class-6 bar draw, and HD the Plasma's five effects, whose lightning is
/// that class; a per-effect choice, not a title-wide one.
#[test]
fn hd_2048_and_omega_list_the_trail_hit_burst_for_the_far_bar_draw_and_hd_its_plasma() {
    use oag_title::engine_effects as n;
    for title in [oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.effects.far_bars,
            [
                n::TRAIL_HITSHIP_EFFECT,
                n::TRAIL_HITSHIP_RED_EFFECT,
                n::DETONATOR_CANNON_SPARKS_EFFECT,
                n::DETONATOR_EMP_EFFECT,
            ],
            "{}",
            title.name
        );
    }
    assert_eq!(
        oag_hd::TITLE.effects.far_bars,
        [
            n::DETONATOR_CANNON_SPARKS_EFFECT,
            n::TRAIL_HITSHIP_EFFECT,
            n::TRAIL_HITSHIP_RED_EFFECT,
            n::DETONATOR_EMP_EFFECT,
            n::PLASMA_FLARE_EFFECT,
            n::PLASMA_CHARGING_EFFECT,
            n::PLASMA_LAUNCH_EFFECT,
            n::PLASMA_LIGHTNING_EXPAND_EFFECT,
            n::PLASMA_LIGHTNING_COLLAPSE_EFFECT,
            n::BOMB_RAYS_EFFECT,
            n::ROCKET_FLARE_EFFECT,
        ]
    );
    // The head's and the wind-up's sprite templates play, and the Rocket craft blast's (its
    // screen wash); the release's and the blast's flashes stay muted.
    assert_eq!(
        oag_hd::TITLE.effects.templates,
        [
            n::PLASMA_FLARE_EFFECT,
            n::PLASMA_CHARGING_EFFECT,
            n::CRAFT_BLAST_EFFECT
        ]
    );
}

/// The Rocket's three effects as HD's own lists load them: the flare's two class-6 emitters as
/// HD's bar with their sprite, every class-6 emitter of both blasts procedural
/// (`docs/rendering/particle-fill.md`, "A Rocket on the wall paired").
fn check_the_rockets_streaks(load: &mut impl FnMut(&'static str) -> oag_fx::psys::Effect) {
    use oag_fx::psys::streak::StreakDraw;
    use oag_title::engine_effects as n;
    let flare = load(n::ROCKET_FLARE_EFFECT);
    let bars: Vec<&str> = flare
        .emitters
        .iter()
        .filter(|e| matches!(e.streak, StreakDraw::FarBar { .. }))
        .map(|e| {
            assert!(e.sprite.is_some(), "{}", e.name);
            e.name.as_str()
        })
        .collect();
    assert_eq!(bars, ["WO_ROCKET_FIRE", "launch_flare"]);
    for blast in [n::TRACK_BLAST_EFFECT, n::CRAFT_BLAST_EFFECT] {
        let effect = load(blast);
        let streaks: Vec<StreakDraw> = effect
            .emitters
            .iter()
            .filter(|e| e.name == "FIRESPIKES" || e.name == "SPARKS")
            .map(|e| e.streak)
            .collect();
        assert_eq!(streaks, [StreakDraw::Procedural; 2], "{blast}");
    }
}

/// The Bomb's two effects as HD's own lists load them: both spread by the extent the file
/// authors, the rays as HD's bar with their sprite, the ring's class-6 `sparks` still
/// procedural (`docs/rendering/particle-fill.md`, "The Bomb's and the Missile's explosions
/// paired"). The Rocket's streaks are checked on the same load.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_bombs_and_the_rockets_streaks_load_as_the_pairs_decided() {
    use oag_fx::psys::streak::StreakDraw;
    use oag_title::engine_effects as n;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let mut load = |name: &'static str| {
        crate::assets::listed_particle_effect(
            &mut archives,
            title.race.effect_dir_for("track"),
            title.effects,
            name,
            false,
            title.effects.colour_walk,
            title.effects,
        )
        .expect("the effect loads")
        .0
    };

    let rays = load(n::BOMB_RAYS_EFFECT);
    assert!(rays.has_extents(), "the rays start on the authored shell");
    assert_eq!(rays.emitters.len(), 1);
    assert_eq!(rays.emitters[0].streak, StreakDraw::FarBar { aspect: 1.0 });
    assert!(rays.emitters[0].sprite.is_some(), "bomb_spike");

    let ring = load(n::BOMB_SMOKERING_EFFECT);
    assert!(ring.has_extents(), "the smoke stands on the authored ring");
    let sparks = ring
        .emitters
        .iter()
        .find(|e| e.name == "sparks")
        .expect("the ring carries sparks");
    assert_eq!(sparks.streak, StreakDraw::Procedural);

    check_the_rockets_streaks(&mut load);
}

/// `WO_TRAIL_HITSHIP`'s two emitters are class 6 (`+0x4c8` aspect 1.0 and 2.0). Listed, both
/// load as HD's bar with their own aspect; unlisted, both stay procedural.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_trail_hit_burst_loads_as_far_bars_when_listed_and_procedural_when_not() {
    use oag_fx::psys::streak::StreakDraw;
    let Some(listed) = trail_hit_burst(true) else {
        return;
    };
    let draws: Vec<StreakDraw> = listed.emitters.iter().map(|e| e.streak).collect();
    assert_eq!(
        draws,
        [
            StreakDraw::FarBar { aspect: 1.0 },
            StreakDraw::FarBar { aspect: 2.0 }
        ]
    );
    assert!(
        listed.emitters.iter().all(|e| e.sprite.is_some()),
        "HD authors both emitters' sprites, and the list reads them"
    );
    let scales: Vec<f32> = listed.emitters.iter().map(|e| e.colour_scale).collect();
    assert_eq!(scales, [0.7, 1.0], "`+0xc84` of the two class-6 emitters");
    let unlisted = trail_hit_burst(false).expect("same image");
    assert!(
        unlisted
            .emitters
            .iter()
            .all(|e| e.streak == StreakDraw::Procedural && e.sprite.is_none())
    );
}

/// The EMP's `SPARKS` author no speed and move only by their noise force (modifier type
/// `0x11`): HD lists the effect, so it loads with the force on and the sparks drawn as HD's
/// bar; without the list the force stays off (`ps3-hdfury-eu/detonator-emp.md`, "The grey
/// rows were sparks that never moved").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_emp_sparks_feel_their_noise_force_only_where_the_title_lists_it() {
    use oag_fx::psys::streak::StreakDraw;
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_hd::open(&image.display().to_string()).expect("HD opens");
    let title: &Title = oag_hd::TITLE;
    let name = oag_title::engine_effects::DETONATOR_EMP_EFFECT;
    let dir = title.race.effect_dir_for("track");
    let load = |archives: &mut oag_assets::Archives, effects: &oag_title::Effects| {
        crate::assets::listed_particle_effect(
            archives,
            dir,
            effects,
            name,
            false,
            title.effects.colour_walk,
            title.effects,
        )
        .expect("the effect loads")
        .0
    };
    let listed = load(&mut archives, title.effects);
    assert_eq!(oag_hd::TITLE.effects.turbulence, [name]);
    let sparks = listed
        .emitters
        .iter()
        .find(|e| e.name == "SPARKS")
        .expect("the wave carries SPARKS");
    assert_eq!(sparks.speed_per_tick, (0.0, 0.0));
    let force = sparks.turbulence.expect("SPARKS authors modifier 0x11");
    assert!(force.on);
    assert_eq!(force.frequency, [0.3, 0.1, 0.3]);
    assert_eq!(force.amplitude, [0.1, 0.1, 0.1]);
    assert!(force.timed && !force.on_position);
    assert_eq!(sparks.streak, StreakDraw::FarBar { aspect: 1.0 });
    let mut unlisted_effects = *title.effects;
    unlisted_effects.turbulence = &[];
    assert!(!load(&mut archives, &unlisted_effects).has_turbulence());
}

/// A Rocket that reaches a craft plays HD's `WO_ROCKET_EXPLO` with the disc's own
/// sprites, every one of the twelve on the particle sheet.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_forced_rocket_hit_plays_the_craft_blast_with_its_sprites() {
    use oag_gameplay::input::{Button, Input};
    use oag_gameplay::{InputSnapshot, PlayerInputs};

    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..crate::Options::default()
    })
    .expect("loading an HD race");
    let mut race = crate::Race::start(loaded.setup, loaded.view, loaded.modules);
    let held = |before: u32, now: u32| {
        let mut buttons = Input::new();
        buttons.begin_frame(before);
        buttons.begin_frame(now);
        PlayerInputs::single(InputSnapshot {
            buttons,
            ..InputSnapshot::new()
        })
    };
    for _ in 0..oag_race::COUNTDOWN_TICKS + 10 {
        race.tick(&held(0, 0));
    }
    race.core.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    race.tick(&held(0, Button::Square.bit()));
    for _ in 0..3 {
        race.tick(&held(0, 0));
    }
    let blast = race
        .effect_for_tests(oag_title::Trigger::CraftBlast)
        .expect("HD loads the craft blast");
    assert_eq!(race.view.stage.alive_of_effect(&blast), 0, "no blast yet");
    assert_eq!(
        blast.emitters.len(),
        15,
        "WO_ROCKET_EXPLO's twelve emitters and its three root templates"
    );
    for spec in &blast.emitters {
        assert!(spec.sprite.is_some(), "{} has no sprite", spec.name);
    }
    assert_eq!(
        blast.unplaced_sprites().collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    let before = race.core.sim.world.ships[1].physics.shield;
    assert!(race.core.sim.force_rocket_hit(1), "a Rocket is in flight");
    for _ in 0..3 {
        race.tick(&held(0, 0));
    }
    assert!(
        race.core.sim.world.ships[1].physics.shield < before,
        "rival 1 took no damage"
    );
    assert!(
        race.view.stage.alive_of_effect(&blast) > 0,
        "the craft blast is not playing"
    );
    check_hds_flipbooks_walk(&race, &blast);
    check_a_forced_shot_stands_the_players_bomb_where_asked(&mut race, &held);
}

/// `--force-shot-at` moves the player's own laid Bomb, and moves nothing when the player has
/// no weapon out.
fn check_a_forced_shot_stands_the_players_bomb_where_asked(
    race: &mut crate::Race,
    held: &dyn Fn(u32, u32) -> oag_gameplay::PlayerInputs,
) {
    use oag_gameplay::input::Button;
    let player = race.core.player_slot();
    let own = |race: &crate::Race| {
        race.core
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|shot| shot.kind.is_some() && usize::from(shot.owner) == player)
            .map(|shot| shot.position)
            .collect::<Vec<_>>()
    };
    for _ in 0..1200 {
        if own(race).is_empty() {
            break;
        }
        race.tick(&held(0, 0));
    }
    assert!(own(race).is_empty(), "the Rocket's own shots have retired");
    let point = oag_core::math::Vec3::new(12.0, -34.0, 56.0);
    assert!(!race.core.sim.force_shot_at(player, point), "nothing out");
    race.core.sim.world.ships[player].pickup.weapon = Some(oag_tables::weapons::Weapon::Bomb);
    race.tick(&held(0, Button::Square.bit()));
    for _ in 0..5 {
        race.tick(&held(0, 0));
    }
    let laid = own(race);
    assert!(!laid.is_empty(), "the Bomb is on the road");
    assert!(race.core.sim.force_shot_at(player, point), "a Bomb is out");
    assert_eq!(own(race).first(), Some(&point));
}

/// HD's update walks its flipbooks: the craft blast's root over its life, the wall blast's
/// root at its rate from a frame in `0..=10`, held on the last cell
/// (`docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`, "The atlas frame"). The craft
/// blast's `trail` alone is also drawn smaller the later in its run a particle is born
/// ("The per-particle size multiplier").
fn check_hds_flipbooks_walk(race: &crate::Race, craft: &oag_fx::psys::Effect) {
    use oag_fx::psys::frames::FrameAdvance;
    assert_eq!(craft.emitters[0].frames, FrameAdvance::OverLife);
    let ramped: Vec<_> = craft
        .emitters
        .iter()
        .filter_map(|spec| Some((spec.name.as_str(), spec.size_ramp?)))
        .collect();
    assert_eq!(ramped, [("trail", (0.2, 1.0))]);
    let wall = race
        .effect_for_tests(oag_title::Trigger::TrackBlast)
        .expect("HD loads the wall blast");
    let root = &wall.emitters[0];
    assert!(
        matches!(root.frames, FrameAdvance::Rate(_)),
        "{:?}",
        root.frames
    );
    assert_eq!(
        root.frame_range.map(|range| (range.spawn, range.hold)),
        Some(((0, 10), (31, 31)))
    );
}
