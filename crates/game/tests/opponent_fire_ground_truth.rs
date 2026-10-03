//! An opponent fires its forward weapon on the original's law, on a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The law is `oag_ai::weapon_ai`, read off `WeaponAi_DecideFireOrAbsorb`
//! (`docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`). What these tests pin
//! is that a Pulse race reaches it: the odds are read off the disc, the law is
//! the one the race runs, and it changes what the field does. Measured
//! 2026-10-03 over seeds 1 to 240 of a parked-player Eliminator on `16_Track`:
//! a forward weapon is held about 1.9 s before it goes on the original's law
//! against 10 to 12 s on this project's older rule, and the field reaches five
//! kills in a median 75 s against 108 s.

use oag_game::race::{self, FireLaw};
use oag_gameplay::PlayerInputs;
use oag_tables::weapons::Weapon;

/// One game-minute at 60 Hz.
const TICKS: u64 = 60 * 60;

fn loaded(mode: oag_race::Mode, seed: u64) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        eliminator_kill_target: (mode == oag_race::Mode::Eliminator).then_some(5),
        seed: Some(seed),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

/// Pulse names its `WeaponAIstats.xml`, the loader reads it, and a race runs
/// on the original's law without being asked to - in both modes that arm the
/// pads.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pulse_race_fires_on_the_original_law() {
    for mode in [oag_race::Mode::Eliminator, oag_race::Mode::SingleRace] {
        let Some(race) = loaded(mode, 1) else {
            return;
        };
        assert_eq!(race.fire_law(), FireLaw::Original, "{mode:?}");
    }
}

/// Ticks a forward weapon was held, over how many were spent, across the
/// field, for one game-minute of a parked-player Eliminator.
fn forward_hold(seed: u64, law: FireLaw) -> Option<(u64, u64)> {
    let mut race = loaded(oag_race::Mode::Eliminator, seed)?;
    race.set_fire_law(law);
    let parked = PlayerInputs::single(oag_gameplay::InputSnapshot::new());
    let forward = |w: Option<Weapon>| {
        matches!(
            w,
            Some(
                Weapon::Rocket
                    | Weapon::Missile
                    | Weapon::Plasma
                    | Weapon::Shuriken
                    | Weapon::LeachBeam
                    | Weapon::Quake
            )
        )
    };
    let count = race.sim.world.ship_count as usize;
    let mut before: Vec<Option<Weapon>> = vec![None; count];
    let (mut held, mut spent) = (0u64, 0u64);
    for _ in 0..TICKS {
        race.tick(&parked);
        for (slot, was) in before.iter_mut().enumerate().skip(1) {
            let now = race.sim.world.ships[slot].pickup.weapon;
            if forward(now) {
                held += 1;
            }
            if forward(*was) && now != *was {
                spent += 1;
            }
            *was = now;
        }
    }
    Some((held, spent))
}

/// The original's law spends a forward weapon in well under half the time the
/// older rule holds one. Fails if the law is not wired: the two arms are then
/// the same rule and hold for the same time.
fn the_original_law_spends_a_forward_weapon_sooner(seed: u64) {
    let Some((held_ours, spent_ours)) = forward_hold(seed, FireLaw::Ours) else {
        return;
    };
    let (held_original, spent_original) =
        forward_hold(seed, FireLaw::Original).expect("loaded once already");
    let per = |held: u64, spent: u64| held as f32 / spent.max(1) as f32 / 60.0;
    let (ours, original) = (
        per(held_ours, spent_ours),
        per(held_original, spent_original),
    );
    println!(
        "seed {seed}: forward weapon held {original:.1} s a spend on the original's law \
         ({spent_original} spent), {ours:.1} s on ours ({spent_ours} spent)"
    );
    assert!(
        original < ours * 0.5,
        "seed {seed}: held {original:.1} s on the original's law against {ours:.1} s on ours"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_original_law_spends_a_forward_weapon_sooner_seed_1() {
    the_original_law_spends_a_forward_weapon_sooner(1);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_original_law_spends_a_forward_weapon_sooner_seed_2() {
    the_original_law_spends_a_forward_weapon_sooner(2);
}
