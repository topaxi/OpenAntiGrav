//! Omega's held cues through `Audio::race_tick` on a real race: each opens when
//! the game holds it, keeps its loops sounding for as long as it is held, and is
//! gone the tick it is released (`docs/ghidra/functions/ps4-omega-eu/wwise-cues.md`,
//! "Held loops"; the loop counts are `docs/formats/wwise.md`, "Loops").
//!
//! **`#[ignore]`d and never run in CI.** They need the decrypted Omega packages,
//! which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game omega_held_loops_ground_truth:: --run-ignored all
//! ```
//!
//! The state each cue reads is set directly and `Race::tick` is not called
//! once it is, as in `sfx_weapon_ground_truth.rs`: what is under test is the
//! sound path's read of the state, and a frozen race keeps every other voice
//! still, so a count over the idle pool is the held cue's alone.

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_raceplay as race;

/// Fifteen seconds at 60 Hz: past the end of every one-shot layer the held
/// events start (the longest, `_NGP_LEACHATTACH`'s, is 13.7 s), so what still
/// sounds then is a loop.
const HOLD_TICKS: usize = 900;

/// An Omega single race (opponents fielded, for the LeachBeam's target) frozen on
/// the grid with the audio open and its pool settled.
fn omega() -> Option<(race::Race, oag_sound::Audio, usize)> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    let mut loaded = race::load(&race::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..race::Options::default()
    })
    .expect("an Omega race loads");
    loaded.setup.countdown_voice = false;
    let mut race = race::Race::start(loaded.setup);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        step(&mut audio, &mut race);
    }
    // Five more seconds of sound with the race frozen: whatever one-shot the
    // grid fired has ended.
    for _ in 0..300 {
        step(&mut audio, &mut race);
    }
    let idle = sounding(&audio);
    for _ in 0..30 {
        step(&mut audio, &mut race);
        assert_eq!(
            sounding(&audio),
            idle,
            "the frozen race's pool is not still"
        );
    }
    Some((race, audio, idle))
}

/// One tick of sound: the composition root's pair. With the null backend
/// `Audio::tick` is what renders, so a one-shot only ends through it.
fn step(audio: &mut oag_sound::Audio, race: &mut race::Race) {
    oag_game::sound::race_tick(audio, race);
    audio.tick();
}

fn sounding(audio: &oag_sound::Audio) -> usize {
    audio.output().with_mixer(|mixer| mixer.active_voices())
}

/// Opens a held cue with `open`, holds it for [`HOLD_TICKS`] and releases it
/// with `close`, then opens it again and releases it while it still sounds.
///
/// `loops` is how many voices still sound at the end of the hold: the event's
/// looping layers (a sound's loop count `0`, or the pick of a continuous
/// container looping forever), measured, so dropping either law moves it. `0`
/// for an event that authors no loop: it played once and was not started again.
fn holds(
    name: &str,
    loops: usize,
    open: impl Fn(&mut race::Race),
    close: impl Fn(&mut race::Race),
) {
    let Some((mut race, mut audio, idle)) = omega() else {
        return;
    };
    open(&mut race);
    step(&mut audio, &mut race);
    let started = sounding(&audio);
    assert!(
        started > idle,
        "{name}: held, and no voice opened ({idle} idle)"
    );
    for tick in 0..HOLD_TICKS {
        step(&mut audio, &mut race);
        assert!(
            sounding(&audio) <= started,
            "{name}: re-triggered at tick {tick} ({} > {started})",
            sounding(&audio)
        );
    }
    let held = sounding(&audio);
    println!("{name}: {started} voices at the start, {held} after {HOLD_TICKS} ticks, {idle} idle");
    assert_eq!(
        held - idle,
        loops,
        "{name}: the voices still sounding after {HOLD_TICKS} ticks held"
    );
    // The original stops the held post with no fade (`StopPlayingID(id, 0,
    // linear)`): gone the tick it is released.
    close(&mut race);
    oag_game::sound::race_tick(&mut audio, &mut race);
    assert_eq!(
        sounding(&audio),
        idle,
        "{name}: released and still sounding"
    );
    // Again, released a second in: what still sounds is cut, one-shots included.
    open(&mut race);
    for _ in 0..60 {
        step(&mut audio, &mut race);
    }
    assert!(
        sounding(&audio) > idle,
        "{name}: held a second time, and no voice opened"
    );
    close(&mut race);
    oag_game::sound::race_tick(&mut audio, &mut race);
    assert_eq!(
        sounding(&audio),
        idle,
        "{name}: released while sounding, and still sounding"
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_shield_holds_its_loops_until_it_drops() {
    holds(
        "_SHIELD_PC",
        5,
        |race| race.sim.world.ships[0].physics.shield_pickup_timer = 1_000.0,
        |race| race.sim.world.ships[0].physics.shield_pickup_timer = 0.0,
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_autopilot_holds_its_loops_until_it_expires() {
    holds(
        "_AUTOPILOT_PC",
        5,
        |race| race.sim.world.ships[0].autopilot_timer = 1_000.0,
        |race| race.sim.world.ships[0].autopilot_timer = 0.0,
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_magstrip_holds_its_hum_until_the_craft_leaves_it() {
    holds(
        "_magstrip01",
        5,
        |race| race.raise_cue(Cue::Magstrip),
        |race| race.raise_cue(Cue::MagstripStop),
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn a_shuriken_holds_its_travel_loop_while_it_flies() {
    holds(
        "_SHURIKENTRAVEL",
        3,
        |race| {
            let position = race.sim.world.ships[0].physics.body.position;
            race.sim.world.projectiles.slots[0] = oag_weapons::projectile::Projectile {
                kind: Some(oag_tables::weapons::Weapon::Shuriken),
                position,
                owner: 0,
                lifetime: 5.0,
                ..Default::default()
            };
        },
        |race| {
            race.sim.world.projectiles.slots[0] = oag_weapons::projectile::Projectile::default();
        },
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn the_leach_attach_plays_once_while_the_beam_holds() {
    holds(
        "_NGP_LEACHATTACH",
        0,
        |race| {
            assert!(race.ship_count() > 1, "the beam needs a second craft");
            race.sim.world.leach_beams[0] = Some(oag_weapons::projectile::leach_beam::Beam {
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
                trip: None,
            });
        },
        |race| race.sim.world.leach_beams[0] = None,
    );
}
