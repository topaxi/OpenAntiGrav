//! The field on de Konstruct, which the lone-Ace gates cannot see.
//!
//! **White is the forward `05_Track`** (`track.vex`) and **Black is the reversed
//! one**, `21_Track` (`track_reversed.vex`) - the maintainer's convention
//! (2026-10-03), not a reading of `Definition.xml`'s entry order, which two lanes
//! took the wrong way round. Both layouts have their own bound. The file name
//! still says "black" from that mistake. The worst field measured is White; the
//! player's report named Black, whose field was always the better of the two.
//!
//! The lone-Ace gates (`race_ground_truth.rs`, `ai_clean_lap_gate.rs`) race slot
//! 1 alone, and slot 1 is the pole position: the one craft that arrives at the
//! first jump fast enough. A field of seven reaches it behind a leader, a few
//! units a second short, and a craft that leaves the ground short of the lip is
//! destroyed or beached. That is what a player saw ("the AI really struggles
//! with de Konstruct Black") and what no lone row measured.
//!
//! Seven AI Aces, the player's slot parked, weapons as the mode ships them.
//! One `#[test]` per (speed class, seed), so the matrix is the test axis and a
//! failure names its cell. `BOUND` is a ceiling on **craft destroyed**, frozen
//! from `print_bounds` (`OAG_SWEEP=1`, release); it may only fall.

use oag_game::{catalogue, race};
use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;

/// Five game-minutes, the window every other AI board uses: a later lap kills too.
const TICKS: u64 = 18_000;

fn entry(reversed: bool) -> Option<String> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with("05_Track") && track.reversed == reversed)
        .map(|track| track.entry_name())
}

/// How many of the seven craft ended the run in any state but racing.
fn destroyed(class: &str, seed: u64, reversed: bool) -> Option<u32> {
    let entry = entry(reversed)?;
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(seed),
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading 05_Track at {class}: {error}"));
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let mut dead = vec![false; count];
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        for (slot, flag) in dead.iter_mut().enumerate().skip(1) {
            if race.sim.world.ships[slot].physics.craft_state != CraftState::Racing {
                *flag = true;
            }
        }
    }
    Some(dead.iter().filter(|flag| **flag).count() as u32)
}

/// Ceilings on craft destroyed, `(class, seed, white, black)`: White is the
/// forward `05_Track`, Black the reversed `21_Track`.
///
/// **Regenerated 2026-10-03 for the AI speed plan** (`oag_ai::SpeedPlan`): the
/// reversed layout (Black) now drives a verified plan, White does not (its plan
/// does not verify, so its column is unchanged). Black destroyed over the twelve
/// cells 11 -> 10: four cells fell by one, two rose by one (RAPIER seed 3,
/// PHANTOM seed 2), which is the per-seed spread of a field with weapons on,
/// not a column getting worse.
const BOUND: &[(&str, u64, u32, u32)] = &[
    ("VENOM", 1, 6, 0),
    ("VENOM", 2, 4, 0),
    ("VENOM", 3, 5, 1),
    ("FLASH", 1, 4, 0),
    ("FLASH", 2, 5, 0),
    ("FLASH", 3, 4, 0),
    ("RAPIER", 1, 1, 0),
    ("RAPIER", 2, 6, 0),
    ("RAPIER", 3, 4, 3),
    ("PHANTOM", 1, 2, 1),
    ("PHANTOM", 2, 4, 4),
    ("PHANTOM", 3, 4, 1),
];

fn check(class: &str, seed: u64) {
    let Some(white) = destroyed(class, seed, false) else {
        return;
    };
    let black = destroyed(class, seed, true).expect("the reversed entry is on the disc");
    let (_, _, white_bound, black_bound) = BOUND
        .iter()
        .find(|(c, s, _, _)| *c == class && *s == seed)
        .expect("a bound for every cell");
    println!("{class} seed {seed}: White (forward) {white} destroyed, Black (reversed) {black}");
    assert!(
        white <= *white_bound,
        "{class} seed {seed}: {white} of 7 destroyed on de Konstruct White (forward 05_Track), bound {white_bound}"
    );
    assert!(
        black <= *black_bound,
        "{class} seed {seed}: {black} of 7 destroyed on de Konstruct Black (reversed 21_Track), bound {black_bound}"
    );
}

macro_rules! cell {
    ($name:ident, $class:expr, $seed:expr) => {
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            check($class, $seed);
        }
    };
}

cell!(venom_seed_1, "VENOM", 1);
cell!(venom_seed_2, "VENOM", 2);
cell!(venom_seed_3, "VENOM", 3);
cell!(flash_seed_1, "FLASH", 1);
cell!(flash_seed_2, "FLASH", 2);
cell!(flash_seed_3, "FLASH", 3);
cell!(rapier_seed_1, "RAPIER", 1);
cell!(rapier_seed_2, "RAPIER", 2);
cell!(rapier_seed_3, "RAPIER", 3);
cell!(phantom_seed_1, "PHANTOM", 1);
cell!(phantom_seed_2, "PHANTOM", 2);
cell!(phantom_seed_3, "PHANTOM", 3);

/// Regenerates [`BOUND`]. `OAG_SWEEP=1`, release, `--no-capture`.
#[test]
#[ignore = "a scratch generator: set OAG_SWEEP, paste the rows"]
fn print_bounds() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for class in ["VENOM", "FLASH", "RAPIER", "PHANTOM"] {
        for seed in 1..=3 {
            let (Some(white), Some(black)) =
                (destroyed(class, seed, false), destroyed(class, seed, true))
            else {
                return;
            };
            println!("    (\"{class}\", {seed}, {white}, {black}),");
        }
    }
}
