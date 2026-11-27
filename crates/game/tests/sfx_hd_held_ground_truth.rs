//! HD's `~AUTOPILOT` and travel loops through `Audio::race_tick`: each runs its
//! list while its condition holds and stops with it
//! (`docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md`, "Held cues the game
//! starts and stops").
//!
//! **`#[ignore]`d and never run in CI**: it needs game content. Run with
//! `just test-data`.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_weapons::projectile::Projectile;

const HD: &str = "hdfury-ps3-eu-dec.iso";

/// An HD race and a silent audio rig that has run a second, so the engine and
/// ambience voices are already open.
fn settled() -> Option<(race::Race, oag_sound::Audio)> {
    let path = oag_testdata::image(HD)?;
    let mut loaded = race::load(&race::Options {
        source: path.display().to_string(),
        ..race::Options::default()
    })
    .expect("loading the race");
    // `ready` and `go` open voices partway through the counts below.
    loaded.setup.countdown_voice = false;
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    Some((race, audio))
}

fn voices(audio: &oag_sound::Audio) -> usize {
    audio.output().with_mixer(|mixer| mixer.active_voices())
}

/// Ticks the audio alone, so the world stays as the test left it.
fn listen(audio: &mut oag_sound::Audio, race: &mut race::Race, ticks: usize) {
    for _ in 0..ticks {
        oag_game::sound::race_tick(audio, race);
        audio.tick();
    }
}

/// One HD race load for both claims, each re-measuring what is idle before it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_s_held_lists_run_while_held_and_stop_with_their_condition() {
    let Some((mut race, mut audio)) = settled() else {
        return;
    };
    autopilot_runs_its_list_and_releases_into_two_closing_voices(&mut race, &mut audio);
    travel_loops_run_their_lists_while_flying(&mut race, &mut audio);
}

fn autopilot_runs_its_list_and_releases_into_two_closing_voices(
    race: &mut race::Race,
    audio: &mut oag_sound::Audio,
) {
    let idle = voices(audio);
    race.core.sim.world.ships[0].autopilot_timer = 100.0;
    listen(audio, race, 1);
    // Three one-shots and the 90-degree loop, and `autopilot_eng`'s two voices.
    assert_eq!(voices(audio), idle + 4 + 2, "the opening voices");

    // Ten seconds on the intro and the line are over; the two loops play on,
    // with a blip some of the time.
    listen(audio, race, 600);
    let held = voices(audio);
    assert!(
        (idle + 2..=idle + 3).contains(&held),
        "{held} against {idle} idle"
    );

    race.core.sim.world.ships[0].autopilot_timer = 0.0;
    listen(audio, race, 1);
    assert_eq!(voices(audio), idle + 2, "the two voices after the `0x2b`");
    listen(audio, race, 600);
    assert_eq!(voices(audio), idle, "nothing outlived the release");
}

fn travel_loops_run_their_lists_while_flying(race: &mut race::Race, audio: &mut oag_sound::Audio) {
    use oag_tables::weapons::Weapon;
    // The Autopilot's timer is spent and its voices closed.
    let idle = voices(audio);
    let origin = race.core.sim.world.ships[0].physics.body.position;
    let flying = |kind| Projectile {
        kind: Some(kind),
        position: origin,
        owner: 0,
        lifetime: 5.0,
        ..Default::default()
    };
    race.core.sim.world.projectiles.slots[0] = flying(Weapon::Rocket);
    race.core.sim.world.projectiles.slots[1] = flying(Weapon::Plasma);
    race.core.sim.world.projectiles.slots[2] = flying(Weapon::Shuriken);
    listen(audio, race, 1);
    // `~ROCKETTVL` one loop; `~PLASMATVL` and `~SHURIKENTRAVEL` a loop and the
    // first pair each.
    assert_eq!(voices(audio), idle + 1 + 3 + 3, "the three lists opened");
    // A second on, the pairs have been re-keyed and the loops are held.
    listen(audio, race, 60);
    assert!(voices(audio) >= idle + 3, "the loops are still held");

    for slot in 0..3 {
        race.core.sim.world.projectiles.slots[slot] = Projectile::default();
    }
    listen(audio, race, 1);
    assert_eq!(voices(audio), idle, "a held voice outlived its bolt");

    let stats = race.core.sim.quake_stats().expect("HD authors a Quake");
    let length = race.core.sim.course().expect("a course").length();
    listen(audio, race, 1);
    let idle = voices(audio);
    race.core.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
        0,
        length * 0.5,
        1.0,
        &stats,
    ));
    race.tick(&PlayerInputs::none());
    // The 90-degree loop keys 15 master ticks (62 ms) after the 270-degree one.
    listen(audio, race, 5);
    assert!(race.quake_point().is_some(), "the wave has no road point");
    assert_eq!(voices(audio), idle + 2, "~QUAKETRAVEL's 270 and 90 loops");
    race.core.sim.world.quake = None;
    race.tick(&PlayerInputs::none());
    listen(audio, race, 1);
    assert_eq!(voices(audio), idle, "~QUAKETRAVEL outlived the wave");
}
