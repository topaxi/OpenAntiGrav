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
//! **2 of 27**, against 54 of 208 before the gate was fixed. See
//! `oag_ai::driver`'s `RAM_CLEARANCE` for the sweep.
//!
//! **Twenty-seven and not a hundred and thirty-eight**, because a ram may only
//! target the player - `oag_ai`'s `driver::ram::PLAYER_SLOT`, after AI-on-AI
//! shoving was reported to spiral in a clump. That is most of what this file
//! counts: the field shoves about a fifth as often as it did, and the sample
//! behind the bound below is correspondingly thinner.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Ticks per race: over three minutes at the fixed 60 Hz.
///
/// Long rather than short, because the rate this measures is not stationary: a
/// grid is packed for the first minute and strung out afterwards, so a one-lap
/// window measures the start rather than the race. Measured before the ram was
/// narrowed to the player: six races of a minute gave 31 shifts and a rate of
/// 19%, six of this length 138 and 8%.
const TICKS: usize = 12_000;

/// How many seeded races the measurement runs over.
///
/// One race is not a measurement here: a ram is a rare event gated on a roll,
/// and the change of a single gate reshuffles the whole field's trajectory.
///
/// **Six was not enough either, and that was found the hard way on
/// 2026-09-08.** Six races produced 27 shifts when the bound was written and
/// **9** on the tree that day - the ram was narrowed to the player slot in
/// between, and every pickup-pool change since has reshuffled the field's
/// trajectories - so the denominator collapsed and `2 of 9` tripped a bound
/// that reads "no worse than one in six". At forty races the same tree measures
/// **11 of 123, 8.9%**, against the 2 of 27 (7.4%) the bound was set from.
/// **The rate never moved; the sample had stopped being able to see it.** Ten
/// shifts cannot distinguish 9% from 17%; a hundred and twenty can.
///
/// Forty costs about 110 seconds in a debug build, which is what `just
/// test-data` runs. That is the price of a bound that means something.
const RACES: u64 = 40;

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
    oag_testdata::image("data/images/pulse-psp-eu.chd")
}

/// Where a craft sits across the AI corridor, in the corridor's own lateral
/// axis, and how far the corridor reaches either side of the line there.
fn across(race: &race::Race, slot: usize) -> Option<(f32, f32, f32)> {
    let ship = &race.sim.world.ships[slot];
    let aim = race.line_of(slot).aim(ship.driver.index as usize, 0.0);
    let frame = aim.corridor?;
    let offset = (ship.physics.body.position - aim.point).dot(frame.lateral);
    Some((offset, frame.left, frame.right))
}

/// One shift, and what became of the craft that threw it.
struct Shift {
    /// The room the craft itself had on the side it went: where it stood at the end of the
    /// tick before the timer armed - the state the driver decided from - against the
    /// corridor at the index the driver holds on the arming tick.
    ///
    /// **Not against the corridor at last tick's index**, which this used to read and which
    /// is two samples behind: `Driver::drive` sets `self.index` from the body's position at
    /// the top of the tick (`crates/ai/src/driver.rs`, `self.index = index as u32`, the only
    /// write to it) and only then calls `self.ram`, which asks `aim(self.index)`, so the
    /// corridor it measured is the arming tick's and `ship.driver.index` read after that tick
    /// is the index it used. On a corridor that
    /// narrows that is a few tenths: on 2026-10-02 a Pulse grid walk that moved every
    /// starting slot a fraction of a unit put one shove at seed 24 on a tapered stretch,
    /// read `11.68` against last tick's index and `12.05` against the index the driver
    /// used, and the 0.1 slack below was never meant to carry a taper.
    room: f32,
    /// The furthest it ended up past that corridor edge within [`WATCH`].
    past_edge: f32,
}

fn watch_one_race(image: &Path, seed: u64) -> Vec<Shift> {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
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
    // Where each craft stood at the end of last tick, which is what the driver saw when it
    // decided.
    let mut previous: [Option<oag_core::math::Vec3>; 8] = [None; 8];
    let mut watching: Vec<(usize, usize, f32, f32, Shift)> = Vec::new();
    let mut done: Vec<Shift> = Vec::new();

    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());

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
            let ship = &race.sim.world.ships[slot];
            let on = ship.physics.sideshift_timers.iter().any(|t| *t > 0.0);
            if on && !shifting[slot] {
                // Which way it went, from the timer that armed. Index 1 is the
                // right-hand shift - see `oag_physics::airbrake`.
                let toward = if ship.physics.sideshift_timers[1] > 0.0 {
                    1.0
                } else {
                    -1.0
                };
                let room = previous[slot].map_or(f32::INFINITY, |position| {
                    let aim = race.line_of(slot).aim(ship.driver.index as usize, 0.0);
                    aim.corridor.map_or(f32::INFINITY, |frame| {
                        let offset = (position - aim.point).dot(frame.lateral);
                        let edge = if toward > 0.0 {
                            frame.right
                        } else {
                            -frame.left
                        };
                        edge - offset * toward
                    })
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
            previous[slot] = Some(ship.physics.body.position);
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

/// Both readings of the ram, over one shared sweep of [`RACES`] seeded races.
///
/// # The gate itself, on real geometry
///
/// A craft never throws a shift toward an edge it is already up against.
///
/// **This is the one that would have caught the bug it was written for.**
/// `Driver::ram` used to ask `Frame::room`, which answers how far the *line*
/// may go that way - so a craft drifted off its line by a corner or by the last
/// shove it took would read the corridor's own width and fire into the wall.
/// Measured before the fix, on this scenario: a shove fired with 3.13 units of
/// corridor to its left while the craft was already 11.67 units past that edge.
///
/// # And the outcome, as a bound rather than a property
///
/// See the module docs for why it cannot be one.
///
/// # Why the two readings share one test rather than one helper
///
/// They used to be two `#[test]` functions that both called
/// [`watch_the_field`]. `cargo nextest` runs **every test in its own process**,
/// so nothing is shared between them and those forty races ran *twice*.
///
/// It cost more than the doubled CPU, which is the part worth knowing: on
/// 2026-09-09 the two reported **226 s and 206 s** for one sweep's worth of
/// work, because they ran side by side and contended for the same cores. Merged
/// into one test the same sweep measures **116 s** - so removing the duplicate
/// halved the CPU *and* halved the wall clock, in a suite whose whole wall clock
/// was 587 s.
///
/// **And the sweep must not be split further**, which is the tempting next move
/// and the wrong one: the bound below is a *ratio*, and [`RACES`]'s own doc
/// comment records the 2026-09-08 day spent discovering that a sample of six
/// races could not carry it. Slicing forty races into blocks would rebuild
/// exactly the small-denominator flake that was just removed. The clearance
/// reading alone would slice cleanly - it is a `min` - but there is nothing to
/// gain by slicing half of a sweep the other half still needs whole.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ram_fires_only_where_there_is_room_and_rarely_throws_the_rammer_out() {
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

    let outside = shifts.iter().filter(|shift| shift.past_edge > 0.0).count();
    println!(
        "{outside} of {} shifts ended outside the corridor",
        shifts.len()
    );
    // 2 of 27 measured 2026-08-24, against 54 of 208 before the gate was fixed.
    // A sixth is "no worse than today" with room for noise, not a target - see
    // the module docs for why the outcome cannot be a property.
    //
    // **The bound is unchanged; the sample under it grew.** Re-measured
    // 2026-09-08 at 11 of 123 (8.9%) over `RACES` races - see that constant's
    // own doc comment for why six races stopped being able to support this and
    // what tripped it. Nothing about the ram moved.
    assert!(
        outside * 6 <= shifts.len(),
        "{outside} of {} shifts ended past the corridor edge they went toward, \
         which is worse than the 2 of 27 this was measured at",
        shifts.len()
    );
}
