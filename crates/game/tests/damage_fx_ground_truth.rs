//! HD's damage smoke on a real race out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test damage_fx_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! That `WO_DAMAGE_MILD`, `_MODERATE` and `_CRITICAL` load off HD's disc
//! (they are in `DATA02`, and `WO_DAMAGE_MILD` again in Fury's `DATA06`) and
//! play when a weapon hit lands on a hurt craft, and that Pulse, whose
//! executable has no such smoke, plays none for the same hit.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn single_race(image: &str, team: Option<&str>) -> Option<race::Loaded> {
    let image: PathBuf = oag_testdata::image(image)?;
    Some(
        race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::SingleRace,
            team: team.map(str::to_string),
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// Lands one hit on the player at `shield` and returns the smokes started.
fn hit_at(loaded: race::Loaded, shield: f32) -> u32 {
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[0].physics.shield = shield;
    race.force_weapon_hit(0);
    race.damage_smokes_started_for_tests()
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_smokes_by_the_shield_a_hit_leaves() {
    for (shield, want) in [(90.0, 1), (55.0, 1), (20.0, 2)] {
        let Some(loaded) = single_race("data/images/hdfury-ps3-eu-dec.iso", Some("Feisar")) else {
            return;
        };
        for effect in ["WO_DAMAGE_MILD", "WO_DAMAGE_MODERATE", "WO_DAMAGE_CRITICAL"] {
            assert!(
                !loaded.report.iter().any(|line| line.contains(effect)
                    && (line.contains("missing") || line.contains("not found"))),
                "{effect} did not load"
            );
        }
        assert_eq!(hit_at(loaded, shield), want, "shield {shield}");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_plays_no_damage_smoke() {
    let Some(loaded) = single_race("data/images/pulse-psp-usa.chd", Some("Assegai")) else {
        return;
    };
    assert_eq!(hit_at(loaded, 20.0), 0);
}
