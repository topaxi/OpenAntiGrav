//! What a Quake hit sounds like on the struck craft: `QUAKEHIT` once and the
//! hull's own `.COLLISIONS`, one for each of the one or two `Ship Collision Fx`
//! locators `Ship_Damage` throws a spark from.
//!
//! The original's live `Sound_Play` log at a Quake hit on the player shows
//! `QUAKEHIT` (return `0x08842088`) and two `.COLLISIONS` (return `0x0892486c`,
//! inside `ShipCollisionFx_Trigger`), volume `1.0`, behind the locator's 0.8 s
//! gate (`cannon-quake-leachbeam.md`, 2026-10-26). Ours raised the first alone
//! until the sparks' gate also raised `Cue::Collision`.
//!
//! The rival's wave is launched with `Race::force_quake`, the same aid
//! `--force-quake` is. Dropping the cue from `Race::throw_hit_spark` takes the
//! `.COLLISIONS` count to zero here.
//!
//! **`#[ignore]`d and never run in CI**: it needs a disc image.

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_raceplay as race;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quake_hit_on_the_player_plays_quakehit_and_the_hull_s_collision_cues() {
    let Some(path) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..oag_race::COUNTDOWN_TICKS + 30 {
        race.tick(&PlayerInputs::none());
    }
    let _ = race.drain_cues();
    let player = race.player_slot();

    assert!(race.force_quake(1), "the rival's wave never launched");
    let (mut quake_hits, mut collisions, mut every_craft) = (0, 0, 0);
    for _ in 0..20 {
        race.tick(&PlayerInputs::none());
        for event in race.drain_cues() {
            if event.cue == Cue::Collision {
                every_craft += 1;
            }
            if usize::from(event.slot) != player {
                continue;
            }
            match event.cue {
                Cue::QuakeHit => quake_hits += 1,
                Cue::Collision => collisions += 1,
                _ => {}
            }
        }
    }
    println!("QUAKEHIT {quake_hits}, .COLLISIONS {collisions} ({every_craft} on every craft)");
    assert_eq!(quake_hits, 1, "the wave hit the player {quake_hits} times");
    assert!(
        (1..=2).contains(&collisions),
        "{collisions} .COLLISIONS from one Quake hit, not one or two"
    );
    // The wave struck the rest of the grid too, and each hull throws its own.
    assert_eq!(
        every_craft,
        race.hit_sparks_started_for_tests(),
        "a spark and its cue came apart"
    );
}
