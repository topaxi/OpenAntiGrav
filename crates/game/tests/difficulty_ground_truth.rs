//! The difficulty scale produces an ordered *race*, not just ordered multipliers.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! cargo nextest run -p oag-game --run-ignored all -E 'binary(difficulty_ground_truth)'
//! ```
//!
//! **The only place the scale can be checked.** `oag-ai`'s own tests assert the
//! multipliers are ordered, which is arithmetic; whether an ordered multiplier
//! produces an ordered race is a question about a real circuit, a real hull and a
//! real corridor. A level that looked harder on paper and lapped no faster would
//! be a setting that does nothing.
//!
//! Moved here from `race_ground_truth.rs` on 2026-08-17, when the measurement was
//! rewritten - see [`every_difficulty_is_quicker_than_the_one_below_it`] for why
//! one seed could not carry it.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The disc, or `None` on a checkout without one.
fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// How far below `elite` an `ace` leader may land before it means something.
///
/// Two per cent, against a measured swing of a percentage point either way
/// between runs that changed nothing about the scale. A real regression at
/// the top - `ace` losing its grip or its turn rate - reads as the 10 % gaps
/// the lower levels show, not as a point.
const CEILING_TOLERANCE: f32 = 0.02;

/// Race seeds to average a difficulty over.
///
/// **Five, and the count is the point of this test's rewrite.** See the test.
const SEEDS: [u64; 5] = [0x00C0_FFEE, 1, 7, 12_345, 99];

/// How far the leading opponent got in one minute, and whether anybody wrecked.
fn leader_distance(level: oag_ai::Difficulty, seed: u64) -> Option<(f32, usize)> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: level,
        seed: Some(seed),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);

    for _ in 0..3_600 {
        race.tick(&PlayerInputs::none());
    }

    let course = race.course().expect("a closed ring");
    let distance = (1..8)
        .map(|slot| race.sim.world.ships[slot].standing.distance(course))
        .fold(f32::NEG_INFINITY, f32::max);
    let wrecked = (1..8)
        .filter(|slot| {
            race.sim.world.ships[*slot].physics.craft_state != oag_physics::CraftState::Racing
        })
        .count();
    Some((distance, wrecked))
}

/// Each difficulty is measurably harder than the one below it, on real geometry.
///
/// # Measured on the leader, not on the field's average
///
/// The difference matters. Aggression rises with the level, and aggression is
/// largely spent on *each other*: a novice field never blocks or shoots, so all
/// seven flow, while a skilled field trades places and loses time doing it.
/// Averaged, that made novice look quicker than skilled and the test fail on a
/// scale that was working. What a player races is the craft in front, so the
/// honest question is how far the leading opponent got.
///
/// # And averaged over seeds, because one race cannot resolve the top two
///
/// **This test used to run one race per difficulty and it was passing by a
/// hair.** Measured on 2026-08-17, one race each: novice 6,704, skilled 7,199,
/// elite 7,766, ace 7,810. The first two gaps are 495 and 567; the last is
/// **44**, which is half a percent, and a race is not repeatable to half a
/// percent - seven drivers trading places is a chaotic system, and any change to
/// the field's dynamics reshuffles who gets clear.
///
/// It duly broke on a change that made every difficulty *faster*: a stall rescue
/// that frees a wedged craft moved elite to 8,078 and ace to 8,075, and the test
/// failed on three units in eight thousand while reporting an improvement.
///
/// Five seeds resolve it. Means over [`SEEDS`], same day: novice **6,477**,
/// skilled **7,095**, elite **7,637**, ace **7,751** - ordered throughout, with
/// the top gap at 1.5 %. Per seed, elite beats ace on two of the five, which is
/// exactly the variance a single run cannot see past. **If this fails again,
/// print the spread before touching the scale**: an ordering that holds on the
/// mean and not on one seed is the measurement being noisy, not the setting
/// being broken.
///
/// # A wreck is no longer evidence of a bug
///
/// This used to assert `wrecked == 0` on every seed, back when nothing an
/// opponent did to another opponent could destroy one - a wreck meant a
/// physics fault corrupting the distance measurement, and it was right to
/// fail loudly on it. The AI gained working weapons on 2026-08-26 and now
/// aggressively uses them at `ace` - `1.00` on the appetite scale in
/// `docs/gameplay/ai.md#difficulty` - so a craft going down to a rival's
/// missile is the setting working as tuned, not a bug. Measured 2026-09-02:
/// 3 of 5 `ace` seeds put exactly one opponent out of the race, and the
/// ordering above still holds on the same run. A wreck is now printed rather
/// than asserted against, and it does not corrupt the leader-distance metric
/// either way: [`leader_distance`] takes the max over every slot's own
/// `Standing::distance`, and a wrecked craft's position (and so its distance)
/// simply stops advancing and falls out of contention for the lead, which is
/// what actually happens to it.
///
/// # `elite` and `ace` are at the pace ceiling, and the metric says so
///
/// `docs/gameplay/ai.md#difficulty` recorded it first: the top two are "within
/// one per cent" of each other on mean speed, because at that point the field
/// is at full throttle most of the time and what separates them is appetite,
/// not pace. This test held a strict `ace > elite` on the leader's distance
/// anyway, and on 2026-09-16 it stopped holding - bisected to the Rocket
/// spending its blast the way the original does (`lane/rocket-blast`, a
/// faithful port), which moved the pair from `6,745 / 6,795` to
/// `6,769 / 6,751`: a swing of 0.7 % one way to 0.3 % the other, both inside
/// the per-seed spread, with `ace` ahead on two seeds of five either side of
/// the change. The scale was not touched. What is asserted for that pair now
/// is what the metric can resolve: `ace` is **not slower** than `elite` by
/// more than [`CEILING_TOLERANCE`], while the three lower gaps - 11 %, 4 % -
/// stay strict.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_difficulty_is_quicker_than_the_one_below_it() {
    let mut measured: Vec<(&str, f32)> = Vec::new();
    for (name, level) in oag_ai::Difficulty::ALL {
        let mut runs = Vec::new();
        for seed in SEEDS {
            let Some((distance, wrecked)) = leader_distance(level, seed) else {
                return;
            };
            if wrecked > 0 {
                println!("{name} seed {seed}: {wrecked} craft out of the race");
            }
            runs.push(distance);
        }
        let mean = runs.iter().sum::<f32>() / runs.len() as f32;
        println!(
            "{name}: leader per seed {:?}, mean {mean:.0}",
            runs.iter().map(|d| d.round() as i32).collect::<Vec<_>>()
        );
        measured.push((name, mean));
    }

    for pair in measured.windows(2) {
        let (easier, harder) = (&pair[0], &pair[1]);
        // The top pair: see the doc comment. A tolerance rather than an
        // ordering, because the ordering is inside the noise.
        let floor = if harder.0 == "ace" {
            easier.1 * (1.0 - CEILING_TOLERANCE)
        } else {
            easier.1
        };
        assert!(
            harder.1 > floor,
            "{}'s leader covered no more ground than {}'s, averaged over {} seeds: \
             {:.0} against {:.0}",
            harder.0,
            easier.0,
            SEEDS.len(),
            harder.1,
            easier.1
        );
    }
}
