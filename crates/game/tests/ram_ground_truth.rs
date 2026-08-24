//! What an opponent's ram does on a real circuit, out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! # Why this is not in `race_ground_truth.rs`
//!
//! That file is frozen at its own length by `scripts/check-file-size.py` and may
//! not grow. This is a subject of its own anyway: everything here is about one
//! decision, `oag_ai::Driver::ram`, watched through the only surface a race
//! exposes it on - the physics' own `sideshift_timers`.
//!
//! # What is asserted, and what deliberately is not
//!
//! The **gate** is asserted exactly: a shift never fires without
//! `oag_ai`'s own clearance on the side it goes, measured from where the craft
//! is. That is a property of the code and holds or does not.
//!
//! The **outcome** is asserted as a bound and not as a property, because it is
//! not one. A shift covers a median 8.2 units across the corridor and the craft
//! travels a hundred units downtrack while it plays out, into a corridor that
//! may have narrowed; and a shove that connects hands the rammer whatever
//! `oag_physics::pair::resolve` gives it, which is not the driver's to gate. So
//! some shifts still end up outside, and the bound here is "no worse than
//! today" rather than a target. Measured 2026-08-24 over the six races below:
//! **11 of 138**, against 54 of 208 before the gate was fixed. See
//! `oag_ai::driver`'s `RAM_CLEARANCE` for the sweep.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

/// Ticks per race: over three minutes at the fixed 60 Hz.
///
/// Long rather than short, because the rate this measures is not stationary: a
/// grid is packed for the first minute and strung out afterwards, so a one-lap
/// window measures the start rather than the race. Six races of a minute give
/// 31 shifts and a rate of 19%; six of this length give 138 and 8%.
const TICKS: usize = 12_000;

/// How many seeded races the measurement runs over.
///
/// One race is not a measurement here: a ram is a rare event gated on a roll,
/// and the change of a single gate reshuffles the whole field's trajectory. Six
/// is enough that the rate is stable across a re-run and still under ten
/// seconds.
const RACES: u64 = 6;

/// How long after a shift its consequences are watched for. A second, which is
/// comfortably past the point the craft's own grip has taken the lateral
/// velocity back.
const WATCH: usize = 60;

/// What the ram gate asks for. Kept in step with `oag_ai::driver`'s own
/// constant by the assertion below rather than by hope: this test measures the
/// room a shift actually had, so a mismatch shows up as a failure and not as a
/// test that stopped asking.
const CLEARANCE: f32 = 12.0;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-eu.chd");

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Where a craft sits across the AI corridor, in the corridor's own lateral
/// axis, and how far the corridor reaches either side of the line there.
fn across(race: &race::Race, slot: usize) -> Option<(f32, f32, f32)> {
    let ship = &race.world.ships[slot];
    let aim = race.racing_line().aim(ship.driver.index as usize, 0.0);
    let frame = aim.corridor?;
    let offset = (ship.physics.body.position - aim.point).dot(frame.lateral);
    Some((offset, frame.left, frame.right))
}

/// One shift, and what became of the craft that threw it.
struct Shift {
    /// The room the craft itself had on the side it went, on the tick before
    /// the timer armed - which is the tick the driver decided on.
    room: f32,
    /// The furthest it ended up past that corridor edge within [`WATCH`].
    past_edge: f32,
}

fn watch_one_race(image: &Path, seed: u64) -> Vec<Shift> {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        mode: oag_race::Mode::SingleRace,
        // The rung that leaves every appetite at full, so a ram is not gated
        // down to nothing before this can look at it.
        difficulty: oag_ai::Difficulty::Elite,
        seed: Some(seed),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    // Slot 0 flown too, so the grid stays together and craft actually end up
    // level with each other. A parked player is a race with seven craft in it.
    race.set_autopilot(true);

    let ships = race.ship_count() as usize;
    let mut shifting = [false; 8];
    // Last tick's `across`, which is what the driver saw when it decided.
    let mut previous: [Option<(f32, f32, f32)>; 8] = [None; 8];
    let mut watching: Vec<(usize, usize, f32, f32, Shift)> = Vec::new();
    let mut done: Vec<Shift> = Vec::new();

    for _ in 0..TICKS {
        race.tick(&oag_gameplay::InputSnapshot::default());

        for (slot, _, toward, _, shift) in &mut watching {
            if let Some((offset, left, right)) = across(&race, *slot) {
                let over = if *toward > 0.0 {
                    offset - right
                } else {
                    left - offset
                };
                shift.past_edge = shift.past_edge.max(over);
            }
        }
        for watch in &mut watching {
            watch.1 -= 1;
        }
        while let Some(index) = watching.iter().position(|watch| watch.1 == 0) {
            done.push(watching.remove(index).4);
        }

        for slot in 0..ships {
            let ship = &race.world.ships[slot];
            let on = ship.physics.sideshift_timers.iter().any(|t| *t > 0.0);
            if on && !shifting[slot] {
                // Which way it went, from the timer that armed. Index 1 is the
                // right-hand shift - see `oag_physics::airbrake`.
                let toward = if ship.physics.sideshift_timers[1] > 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let room = previous[slot].map_or(f32::INFINITY, |(offset, left, right)| {
                    let edge = if toward > 0.0 { right } else { -left };
                    edge - offset * toward
                });
                watching.push((
                    slot,
                    WATCH,
                    toward,
                    0.0,
                    Shift {
                        room,
                        past_edge: f32::NEG_INFINITY,
                    },
                ));
            }
            shifting[slot] = on;
            previous[slot] = across(&race, slot);
        }
    }
    done
}

fn watch_the_field() -> Option<Vec<Shift>> {
    let image = image()?;
    let mut all = Vec::new();
    for seed in 0..RACES {
        all.extend(watch_one_race(&image, seed));
    }
    println!("{} shifts over {RACES} races", all.len());
    Some(all)
}

/// The gate itself, on real geometry: a craft never throws a shift toward an
/// edge it is already up against.
///
/// **This is the one that would have caught the bug it was written for.**
/// `Driver::ram` used to ask `Frame::room`, which answers how far the *line*
/// may go that way - so a craft drifted off its line by a corner or by the last
/// shove it took would read the corridor's own width and fire into the wall.
/// Measured before the fix, on this scenario: a shove fired with 3.13 units of
/// corridor to its left while the craft was already 11.67 units past that edge.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ram_never_fires_toward_an_edge_the_craft_has_no_room_for() {
    let Some(shifts) = watch_the_field() else {
        return;
    };
    assert!(
        !shifts.is_empty(),
        "no opponent shifted at all in {RACES} races, so this asserts nothing - \
         the gate is either closed or unreachable"
    );
    // A tenth of a unit of slack and no more: `room` is read on the tick before
    // the timer armed, and a craft covers about two units downtrack in one.
    let worst = shifts
        .iter()
        .map(|shift| shift.room)
        .fold(f32::INFINITY, f32::min);
    assert!(
        worst >= CLEARANCE - 0.1,
        "a shift fired with {worst:.2} units of room on the side it went, \
         against a clearance of {CLEARANCE}"
    );
}

/// And the outcome, as a bound rather than a property - see the module docs for
/// why it cannot be one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ram_rarely_throws_the_rammer_out_of_the_corridor() {
    let Some(shifts) = watch_the_field() else {
        return;
    };
    let outside = shifts.iter().filter(|shift| shift.past_edge > 0.0).count();
    println!(
        "{outside} of {} shifts ended outside the corridor",
        shifts.len()
    );
    // 11 of 138 measured 2026-08-24, against 54 of 208 before the gate was
    // fixed. A sixth is "no worse than today" with room for the noise six races
    // carry, not a target.
    assert!(
        outside * 6 <= shifts.len(),
        "{outside} of {} shifts ended past the corridor edge they went toward, \
         which is worse than the 11 of 138 this was measured at",
        shifts.len()
    );
}
