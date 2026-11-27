//! HD's pickup roster per mode, off the disc's own odds through a loaded race's own gate.
//!
//! The Eliminator table weighs the Repulser and the Shuriken; HD's executable never hands
//! either out (`docs/ghidra/functions/ps3-hdfury-eu/pickup-draw.md`). The same load carries
//! the AI's repeat and the one-live exclusions.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_core::Rng;
use oag_raceplay as race;
use oag_tables::weapons::Weapon;
use oag_weapons::pickup::{Driver, draw};

/// Draws per class and driver: the rarest weapon in either table is well over one in a hundred.
const DRAWS: usize = 6_000;

const ELIMINATOR: &[Weapon] = &[
    Weapon::Rocket,
    Weapon::Missile,
    Weapon::Quake,
    Weapon::Cannon,
    Weapon::Plasma,
    Weapon::Bomb,
    Weapon::Mine,
    Weapon::LeachBeam,
];

const RACE: &[Weapon] = &[
    Weapon::Rocket,
    Weapon::Missile,
    Weapon::Quake,
    Weapon::Turbo,
    Weapon::Shield,
    Weapon::Cannon,
    Weapon::Autopilot,
    Weapon::Plasma,
    Weapon::Bomb,
    Weapon::Mine,
    Weapon::LeachBeam,
];

/// Loads an HD race in `mode` and returns, per driver, the share of draws each weapon took
/// over all four classes.
fn census(mode: oag_race::Mode) -> Option<[Vec<(Weapon, f32)>; 2]> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load_sim(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        ..race::Options::default()
    })
    .expect("loading the race");
    let setup = loaded.setup;
    let weapons = setup.weapons.as_ref().expect("HD's weapon table reads");
    let allowed = (!setup.allowed_weapons.is_empty()).then_some(&*setup.allowed_weapons);
    check_the_one_live_rules(&setup, weapons, allowed);
    let drivers = [Driver::HUMAN_UNPLACED, Driver::Ai];
    Some(drivers.map(|driver| {
        let mut counts: Vec<(Weapon, f32)> = Vec::new();
        let mut total = 0.0;
        for (seed, class) in ["VENOM", "FLASH", "RAPIER", "PHANTOM"].iter().enumerate() {
            let table = weapons.pickups_for(class).expect("every class has odds");
            let mut rng = Rng::new(seed as u64 + 1);
            for _ in 0..DRAWS {
                let drawn = draw(
                    &mut rng,
                    table,
                    driver,
                    None,
                    allowed,
                    setup.pickup_roster.map(|roster| roster.player_order),
                    &[],
                )
                .expect("an HD table always grants");
                total += 1.0;
                match counts.iter_mut().find(|(weapon, _)| *weapon == drawn) {
                    Some((_, count)) => *count += 1.0,
                    None => counts.push((drawn, 1.0)),
                }
            }
        }
        counts.iter().map(|&(w, c)| (w, c / total)).collect()
    }))
}

/// The loaded race carries HD's AI draw with no repeat refusal and the three one-live
/// exclusions, and with all three out the disc's odds still grant, never one of them.
fn check_the_one_live_rules(
    setup: &race::SimSetup,
    weapons: &oag_tables::weapons::WeaponStats,
    allowed: Option<&[Weapon]>,
) {
    let roster = setup.pickup_roster.expect("HD's race carries its roster");
    assert!(
        !roster.ai_refuses_repeat,
        "HD's AI draw does not refuse a repeat"
    );
    let rules: Vec<(Weapon, u8)> = roster
        .exclusions
        .iter()
        .map(|exclusion| (exclusion.weapon, exclusion.limit))
        .collect();
    assert_eq!(
        rules,
        [
            (Weapon::LeachBeam, 1),
            (Weapon::Quake, 1),
            (Weapon::Cannon, 2)
        ]
    );
    let excluded: Vec<Weapon> = rules.iter().map(|&(weapon, _)| weapon).collect();
    let table = weapons.pickups_for("VENOM").expect("Venom has odds");
    let mut rng = Rng::new(7);
    for driver in [Driver::HUMAN_UNPLACED, Driver::Ai] {
        for _ in 0..2_000 {
            let drawn = draw(
                &mut rng,
                table,
                driver,
                None,
                allowed,
                Some(roster.player_order),
                &excluded,
            )
            .expect("HD's odds grant with the three out");
            assert!(!excluded.contains(&drawn), "{driver:?} drew {drawn:?}");
        }
    }
}

fn roster(shares: &[(Weapon, f32)]) -> Vec<String> {
    let mut names: Vec<String> = shares.iter().map(|(w, _)| format!("{w:?}")).collect();
    names.sort();
    names
}

fn named(weapons: &[Weapon]) -> Vec<String> {
    roster(&weapons.iter().map(|&w| (w, 0.0)).collect::<Vec<_>>())
}

fn share(shares: &[(Weapon, f32)], weapon: Weapon) -> f32 {
    shares
        .iter()
        .find(|(w, _)| *w == weapon)
        .map_or(0.0, |&(_, s)| s)
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_eliminator_draws_eight_weapons_and_no_repulser_or_shuriken() {
    let Some([person, ai]) = census(oag_race::Mode::Eliminator) else {
        return;
    };
    assert_eq!(roster(&person), named(ELIMINATOR), "a person's roster");
    assert_eq!(roster(&ai), named(ELIMINATOR), "an AI's roster");
    // The disc weighs 90 in all, 18 of it on the two weapons HD never hands out: a person's
    // Rocket takes theirs (28 of 90), an AI's keeps its own weight over what is left (10 of 72).
    let (person, ai) = (share(&person, Weapon::Rocket), share(&ai, Weapon::Rocket));
    println!("Rocket share: person {person:.3}, AI {ai:.3}");
    assert!((person - 28.0 / 90.0).abs() < 0.02, "person {person}");
    assert!((ai - 10.0 / 72.0).abs() < 0.02, "AI {ai}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_single_race_draws_eleven_weapons() {
    let Some([person, ai]) = census(oag_race::Mode::SingleRace) else {
        return;
    };
    assert_eq!(roster(&person), named(RACE), "a person's roster");
    assert_eq!(roster(&ai), named(RACE), "an AI's roster");
}
