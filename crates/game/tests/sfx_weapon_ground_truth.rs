//! Ground-truth checks specific to individual weapons, split out of
//! `sfx_ground_truth.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; each addition is a move, with no
//! behaviour change to what it moved.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test sfx_weapon_ground_truth --run-ignored all
//! ```

use std::path::PathBuf;

use oag_game::race;
use oag_gameplay::PlayerInputs;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// `CANNONEXPLSHIP` is a **child** of `CANNONEXPLWALL`, not an empty cue -
/// the mechanism behind the correction on `sfx_ground_truth.rs`'s own
/// `NOT_ON_PURE` doc comment, pinned here at the `oag_formats::sblk` level
/// so a change to either cue's own command table would be caught
/// structurally rather than only by a waveform count moving.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn cannonexplship_is_a_child_reference_to_cannonexplwall() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let opened = oag_game::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
        .expect("opening the source");
    let mut archives = opened.archives;
    let blob = archives
        .read_name(r"Data\Sound\weapons.bnk")
        .expect("weapons.bnk");
    let bank = oag_formats::sblk::Bank::parse(&blob).expect("parse bank");
    let ship = bank.cue_named("CANNONEXPLSHIP").expect("CANNONEXPLSHIP");
    let wall = bank.cue_named("CANNONEXPLWALL").expect("CANNONEXPLWALL");

    // The cue's own direct run binds nothing - it is one command, and that
    // command is a child reference rather than a key-on.
    assert!(bank.cue_sounds(&ship).is_empty());

    let children = bank.cue_children(&ship);
    assert_eq!(children.len(), 1, "expected exactly one child grain");
    assert_eq!(
        children[0].opcode, 0x05,
        "expected the indexed-child opcode, not the named one HD uses"
    );
    assert_eq!(
        bank.resolve_child(&children[0]),
        Some(wall),
        "CANNONEXPLSHIP's own child does not resolve to CANNONEXPLWALL"
    );

    // So the tree walk `Banks::load` actually uses finds real audio: exactly
    // the wall cue's own nine waveforms.
    let tree = bank.cue_tree_sounds(&ship);
    assert_eq!(tree.len(), bank.cue_sounds(&wall).len());
    assert_eq!(tree.len(), 9, "CANNONEXPLWALL's own waveform count moved");
}

/// `~REPULSORTRAVEL` (cue 34) is why the Repulser's travel loop is not wired:
/// on both Pulse PSP pressings its whole run is one command, opcode `0x14` with
/// a zero operand - no key-on and no child grain - so there is nothing to play.
/// `Repulser_Init` does start it, on an emitter at the world origin (read live,
/// `docs/ghidra/functions/psp-pulse-usa/repulser.md`). A disc whose cue gains
/// a sound fails here, and the cue should then be wired.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn repulsortravel_binds_no_sound_on_pulse_psp() {
    for disc in ["pulse-psp-usa.chd", "pulse-psp-eu.chd"] {
        let Some(path) = image(disc) else {
            continue;
        };
        let opened =
            oag_game::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
                .expect("opening the source");
        let mut archives = opened.archives;
        let blob = archives
            .read_name(r"Data\Sound\weapons.bnk")
            .expect("weapons.bnk");
        let bank = oag_formats::sblk::Bank::parse(&blob).expect("parse bank");
        let cue = bank.cue_named("~REPULSORTRAVEL").expect("~REPULSORTRAVEL");
        assert!(bank.cue_sounds(&cue).is_empty(), "{disc}");
        assert!(bank.cue_children(&cue).is_empty(), "{disc}");
        assert!(bank.cue_tree_sounds(&cue).is_empty(), "{disc}");
    }
}

/// The four held cues nothing else here drives: `~ROCKETTVL`, `~MISSILETVL`,
/// `~SHURIKENTRAVEL` and `~LEACHATTACH` are all in `BY_LEVEL` in
/// `race::tests::cues::every_cue_has_something_that_raises_it`, exactly
/// because none of them is a `CueEvent` - each is read directly off
/// `race.sim.world` every tick by its own bespoke tracker (`TravelVoices`,
/// `SfxVoices::leach_attach`), the same shape `~PLASMATVL` already has. So
/// unlike every one-shot cue, "does this fire" cannot be answered by
/// draining the queue; it has to be answered the same way `~SHIELD`'s own
/// held voice is in `sfx_ground_truth.rs` - by watching the pool.
///
/// **Projectiles are placed directly rather than fired**, deliberately: this
/// is a test of `Audio::race_tick`'s own read of `World::projectiles`/
/// `World::leach_beam`, not of any weapon's fire path, which is what
/// `race::tests::cues` already covers. `race.tick()` is never called after
/// placing them - calling it would run real flight physics on a synthetic
/// bolt against this disc's own collision mesh, which is not what this test
/// means to exercise - so the same placed state is read on every
/// `audio.race_tick` call here.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_four_travel_voices_open_together_and_close_together() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        // The only mode that fields opponents - the LeachBeam below needs a
        // second craft to fasten onto.
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert!(
        race.ship_count() > 1,
        "the fixture raced alone, so the LeachBeam below has no target"
    );
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_game::audio::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    // Settle first, so the engines are already open and the counts below are
    // differences rather than absolutes - the same reason the shield test in
    // `sfx_ground_truth.rs` does this.
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
    }
    let idle = voices(&audio);

    let origin = race.sim.world.ships[0].physics.body.position;
    let rocket = oag_weapons::projectile::Projectile {
        kind: Some(oag_tables::weapons::Weapon::Rocket),
        position: origin,
        owner: 0,
        lifetime: 5.0,
        ..Default::default()
    };
    let missile = oag_weapons::projectile::Projectile {
        kind: Some(oag_tables::weapons::Weapon::Missile),
        position: origin,
        owner: 0,
        lifetime: 5.0,
        ..Default::default()
    };
    // On the blade's own emitter, so it needs no owner to be heard from.
    let shuriken = oag_weapons::projectile::Projectile {
        kind: Some(oag_tables::weapons::Weapon::Shuriken),
        position: origin,
        owner: 0,
        lifetime: 5.0,
        ..Default::default()
    };
    race.sim.world.projectiles.slots[0] = rocket;
    race.sim.world.projectiles.slots[1] = missile;
    race.sim.world.projectiles.slots[2] = shuriken;
    // A locked beam between slots 0 and 1, built directly rather than fired -
    // this test needs a `Kind::Locked` instance to exist, not a lock to be
    // acquired. The numbers are otherwise inert: nothing here calls `advance`.
    race.sim.world.leach_beam = Some(oag_weapons::projectile::leach_beam::Beam {
        owner: 0,
        target: 1,
        kind: oag_weapons::projectile::leach_beam::Kind::Locked,
        age: 0.0,
        disconnected_at: None,
        first_drain: true,
        first_repair: true,
        damage: 0.0,
        repair: 0.0,
        range: 1_000.0,
        active_time: 100.0,
        energy_multiplier: 1.0,
        slow_ship_factor: 1.0,
    });

    audio.race_tick(&mut race);
    audio.tick();
    assert_eq!(
        voices(&audio),
        // Plus one: `~LEACHATTACH` keys a one-shot (0.6 s) beside its held
        // loop, and it is still sounding a tick later.
        idle + 4 + 1,
        "the Rocket/Missile/Shuriken travel loops and the LeachBeam's own \
         body did not all open"
    );

    // Clear the rest and confirm every held voice this test opened closes.
    race.sim.world.projectiles.slots[0] = oag_weapons::projectile::Projectile::default();
    race.sim.world.projectiles.slots[1] = oag_weapons::projectile::Projectile::default();
    race.sim.world.projectiles.slots[2] = oag_weapons::projectile::Projectile::default();
    race.sim.world.leach_beam = None;
    audio.race_tick(&mut race);
    audio.tick();
    assert_eq!(
        voices(&audio),
        // The attach one-shot is not held and runs out on its own.
        idle + 1,
        "a held travel voice outlived the projectile or beam that opened it"
    );
}

/// `~QUAKETRAVEL` is held for as long as a wave travels: `Quake_Update` opens
/// it on the emitter it gives the span and stops it when the span goes.
///
/// The wave is placed on the far side of the circuit from the grid, so no craft
/// is near enough to draw a `QUAKEHIT` one-shot into the count.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quake_wave_holds_its_travel_loop_for_as_long_as_it_lasts() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_game::audio::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
    }
    let idle = voices(&audio);

    let stats = race.quake_stats().expect("Pulse authors a Quake");
    let length = race.course().expect("a course").length();
    race.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
        0,
        length * 0.5,
        1.0,
        &stats,
    ));
    race.tick(&PlayerInputs::none());
    audio.race_tick(&mut race);
    audio.tick();
    assert!(race.quake_point().is_some(), "the wave has no road point");
    assert_eq!(voices(&audio), idle + 1, "~QUAKETRAVEL did not open");

    race.tick(&PlayerInputs::none());
    audio.race_tick(&mut race);
    audio.tick();
    assert_eq!(voices(&audio), idle + 1, "~QUAKETRAVEL did not stay open");

    race.sim.world.quake = None;
    race.tick(&PlayerInputs::none());
    audio.race_tick(&mut race);
    audio.tick();
    assert_eq!(voices(&audio), idle, "~QUAKETRAVEL outlived the wave");
}

/// `~AUTOPILOT` and `autopilot_eng` through the whole path, the same shape
/// `sfx_ground_truth.rs`'s own
/// `the_shield_opens_a_held_voice_and_closes_it_when_the_pickup_expires`
/// already proves for the shield: `Cue::Autopilot` is a level
/// (`Race::autopilot_is_active`), not a queued edge, so watching the pool is
/// the only way to see it open and close on the right tick.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_autopilot_opens_a_held_voice_and_closes_it_when_the_pickup_expires() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    // **The start-of-race voice is dropped from this fixture.** `ready` opens
    // voices from tick 91 and this counts them; what it counts is the held
    // loops, and the start voice has its own file
    // (`countdown_voice_ground_truth`).
    loaded.setup.countdown_voice = false;
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let voices =
        |audio: &oag_game::audio::Audio| audio.output().with_mixer(|mixer| mixer.active_voices());

    // Settle first, so the engine's own voice is already open and the counts
    // below are differences rather than absolutes.
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
    }
    let idle = voices(&audio);
    assert!(idle >= 1, "the engine never opened");

    // Set directly, the same reason `shield_pickup_timer` is set directly in
    // the shield's own test: this is the field `Race::autopilot_is_active`
    // reads, and granting a real pickup would be testing the pad table
    // instead. Long enough to clear both the `autopilot_eng` one-shot below
    // and the wait for it to end.
    race.sim.world.ships[0].autopilot_timer = 10.0;
    race.tick(&PlayerInputs::none());
    audio.race_tick(&mut race);
    assert!(
        voices(&audio) > idle,
        "the autopilot activated and opened no voice"
    );

    // Held across ticks rather than re-triggered - `~AUTOPILOT` is held, but
    // the same tick also fires `autopilot_eng`, a one-shot, so the pool is
    // walked until that has ended and only the loop is left over idle.
    let held = voices(&audio);
    let mut settled = None;
    for tick in 0..180 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
        assert!(
            voices(&audio) <= held,
            "the autopilot loop is being re-triggered every tick"
        );
        if voices(&audio) == idle + 1 {
            settled = Some(tick);
            break;
        }
    }
    let settled =
        settled.expect("the autopilot's one-shot line never ended, or the loop is not one voice");
    println!("autopilot held with only its loop open after {settled} ticks");

    // Expiry, not the one-second `disengaging` warning: `Autopilot_Update`'s
    // own `<= 0.0f` arm is where the held handle is released, per
    // `autopilot.md`'s "`Autopilot_Update` counts it down" section - the
    // warning fires a tick earlier and changes nothing about the loop.
    race.sim.world.ships[0].autopilot_timer = 0.0;
    race.tick(&PlayerInputs::none());
    audio.race_tick(&mut race);
    assert_eq!(
        voices(&audio),
        idle,
        "the autopilot expired and its voice kept sounding"
    );
}

/// A Pulse race with the audio open and the start-of-race voice dropped, and a
/// closure-free way to count what is sounding.
fn race_and_audio() -> (race::Race, oag_game::audio::Audio) {
    let path = image("pulse-psp-usa.chd").expect("Pulse USA image");
    let mut loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    loaded.setup.countdown_voice = false;
    let audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    (race::Race::start(loaded.setup), audio)
}

fn sounding(audio: &oag_game::audio::Audio) -> usize {
    audio.output().with_mixer(|mixer| mixer.active_voices())
}

/// `~ROCKLOCK` through `Audio::race_tick`: the reticle seeks, then locks, and the
/// tone beeps on the list's own tempo - about every 116 ms seeking and every
/// 58 ms locked - and stops when the target goes away.
///
/// The race locks a grid opponent by itself once a Missile is held (the reticle
/// reads `Seeking` from the first tick and `Locked` about a second in), so
/// nothing is placed. Counted as the share of ticks with a voice above the
/// engine floor: a one-shot per reticle edge, which is what a title with no
/// program falls back to, leaves the locked window silent.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_lock_tone_runs_through_the_race_and_stops_with_the_target() {
    if image("pulse-psp-usa.chd").is_none() {
        return;
    }
    let (mut race, mut audio) = race_and_audio();
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Missile);
    let mut counts = Vec::new();
    for _ in 0..130 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
        counts.push(sounding(&audio));
    }
    assert_eq!(race.sight_state(), oag_race::sight::State::Locked);
    let floor = *counts[10..].iter().min().unwrap();
    let share = |window: &[usize]| {
        window.iter().filter(|&&n| n > floor).count() as f32 / window.len() as f32
    };
    let seeking = share(&counts[10..55]);
    let locked = share(&counts[75..130]);
    assert!(
        (0.2..=0.7).contains(&seeking),
        "seeking beeps are 52 ms in every 116: {seeking}"
    );
    assert!(
        locked >= 0.75,
        "locked beeps are 52 ms in every 58: {locked}"
    );

    race.sim.world.ships[0].pickup.weapon = None;
    for _ in 0..6 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
    }
    assert_eq!(race.sight_state(), oag_race::sight::State::Absent);
    assert_eq!(sounding(&audio), floor, "the tone outlived its target");
}

/// `~BLOWUP` through `Audio::race_tick`: while the player's craft is destroyed
/// the second waveform is re-keyed over the held loop, so more than one voice
/// sounds at once; releasing the level stops all of it. A flat pick is one
/// voice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_explosion_re_keys_through_the_race_and_stops_when_it_ends() {
    if image("pulse-psp-usa.chd").is_none() {
        return;
    }
    let (mut race, mut audio) = race_and_audio();
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        audio.race_tick(&mut race);
        audio.tick();
    }
    let floor = sounding(&audio);
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Destroyed;
    assert!(race.craft_is_exploding());
    let mut peak = 0;
    for _ in 0..60 {
        audio.race_tick(&mut race);
        audio.tick();
        peak = peak.max(sounding(&audio));
    }
    assert!(
        peak >= floor + 2,
        "the loop and its re-keys never sounded together: {peak} over {floor}"
    );
    race.sim.world.ships[0].physics.craft_state = oag_physics::CraftState::Racing;
    for _ in 0..2 {
        audio.race_tick(&mut race);
        audio.tick();
    }
    assert!(
        sounding(&audio) <= floor,
        "the explosion outlived its level"
    );
}
