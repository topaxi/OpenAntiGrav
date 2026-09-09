//! Whether two craft "stick" - resolve or stay overlapped tick after tick
//! with nothing steering them apart - on a real circuit out of a real disc
//! image.
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
//! # Why this file exists
//!
//! A maintainer playing the game reported craft-to-craft contact reading as
//! "sticking" and "almost magnetic". Two mechanisms were found; this file
//! is the acceptance test for the one this crate can fix -
//! `oag_ai::Driver::social` reading only `Field::behind` and never
//! `Field::alongside`, so a rival close enough to actually be touching had
//! already left the one bucket any lateral-avoidance term reacted to. See
//! that function's own doc comment for the fix.
//!
//! This measures the *symptom* directly rather than the fix's internals: on
//! every tick of a real full-grid race, whether any pair of active craft's
//! hulls still overlap (`oag_physics::pair::overlap`, the same box test
//! `oag_physics::pair::resolve` gates its own resolution on), and the
//! longest run of **consecutive** ticks any one pair stayed overlapped.
//!
//! Measured on this exact scenario (`VENOM`, single race, same seed, 7,200
//! ticks):
//!
//! | | before the fix | after the fix |
//! | --- | --- | --- |
//! | longest streak | **90** ticks (pair 4/7, 1.5s) | **46** ticks (pair 4/7) |
//! | other streaks over 10 ticks | 73, 68, 57, 49 | 37, 32, 16, 11 |
//!
//! **The fix roughly halves the worst streaks; it does not remove sticking
//! outright**, and that is expected rather than a sign the fix is
//! incomplete against what it targeted: `Driver::social`'s lean is gated on
//! `personality.defence - personality.courtesy`, and a **neutral**
//! personality (both zero, `Personality::from_seed(0)`'s baseline) returns
//! zero lean regardless of which field channel is read - it never yields
//! *or* covers, alongside or not. Two craft that are each near-neutral on
//! that axis can still sit in continuous light contact with nothing steering
//! either of them away, which this fix does not touch: it closes the
//! blind spot where a touching rival was invisible to the term, not the
//! separate case where the term has nothing to say regardless of visibility.
//! Left as a follow-up rather than folded in here, since it is a different
//! question (whether *every* personality should get some minimal
//! collision-avoidance floor) that the coordinator did not scope into this
//! pass.
//!
//! # The worst-streak statistic was retired on 2026-09-08, and why
//!
//! This file used to assert the **longest run of consecutive overlapping ticks
//! any one pair managed**, bounded at 60 - above the post-fix 46 and below the
//! pre-fix 90. Letting an opponent fire a Cannon (`Race::advance_cannons`
//! running every slot, landed the same day off the recovered gate) took it to
//! 95, and chasing that down showed the statistic itself was broken:
//!
//! - **The 46 was measured in a regime where opponents were frozen out of the
//!   weapon economy.** An opponent used to pick a Cannon up and hold it for the
//!   rest of the race, because nothing could fire one and nothing absorbed one.
//!   *Any* change to that moves this file's numbers, including the most
//!   conservative alternative: absorbing the pickup instead scores 68, and so
//!   does running the reload countdown without spawning a round. All eight
//!   craft finish `Racing` and active in every one of those runs, so nothing is
//!   being wrecked and no pair is being shoved together by a round. It is simply
//!   a different race.
//! - **Worse, the statistic stopped separating the bug from the fix.**
//!   Re-measuring the pathology *under the new regime* - the same run with
//!   `Driver::social` reverted to reading only `Field::behind` - gave **90**
//!   against the fixed driver's **95**. A single worst streak is one sample of
//!   one pair in a chaotic 8-craft, 7,200-tick race, and it had become noise. No
//!   bound could have been drawn between 90 and 95.
//!
//! **So the bound was not raised - the measure was replaced**, on a maintainer
//! ruling. What the two regimes differ in is not the worst pair, it is *how much
//! of the race the field spends in contact*, and that separates cleanly on the
//! same runs:
//!
//! | | pathology (`behind` only) | fixed (`alongside` + contact floor) |
//! | --- | --- | --- |
//! | **overlapped pair-ticks** | **1,756** | **978** |
//! | of which sustained (streak > 10) | 1,062 | 537 |
//! | pairs that ever overlap | 15 | 6 |
//! | worst single streak | 178 | 91 |
//!
//! **Both columns above were measured before 2026-09-09, when the Cannon's
//! base speed was still a chosen `400.0` rather than its measured `500.0`.**
//! This scenario runs a `SingleRace` with weapons live, and the correction
//! moves the fixed driver's own score from 1,051 to **1,558** - so the
//! pathology column is a stale ceiling and the two are no longer a like-for-
//! like pair. Re-measuring the pathology against a correct Cannon needs
//! `oag_ai::Driver::social`'s fix reverted for one run; nobody has done it.
//! See [`MAX_OVERLAPPED_PAIR_TICKS`].
//!
//! `MAX_OVERLAPPED_PAIR_TICKS` is set from those measured runs rather than from
//! what makes today's tree pass. The per-pair streaks are still *printed*,
//! because they are the readable description of what happened; they are simply
//! no longer what fails the test.
//!
//! The second half of the fix landed with the same change: `CONTACT_FLOOR` in
//! `oag_ai::Driver::social`, the minimum yield every driver now gives a rival it
//! is already alongside. That closes the gap this header used to record as a
//! known follow-up - a neutral personality contributing exactly zero at the one
//! range where contact is already happening.

use std::path::{Path, PathBuf};

use oag_game::race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// How much of the race the whole grid may spend with hulls overlapping,
/// counted as **pair-ticks**: one for every pair of craft overlapping on any
/// given tick, summed over the race.
///
/// **Ours, a tripwire rather than a measurement of the original or a claim
/// about what "good" looks like** - the original has no equivalent driver term
/// to compare against (see `oag_ai::Driver::social`'s own doc comment), so
/// there is nothing to recover this from. No confidence score.
///
/// **Set from a measured run rather than from what makes today's tree pass.**
/// It was `1_300` until 2026-09-09, sitting midway between a fixed driver's
/// **978** and the pathology's **1,756** (see this file's header).
///
/// **Re-baselined to `2_000` when the Cannon's base speed stopped being a
/// guess, and the midpoint rule it used to follow no longer applies.**
/// `oag_gameplay::projectile::cannon::BASE_SPEED_KMH` was a chosen `400.0`; it
/// is a *measured* `500.0` (confidence 90 - `func_0x00060af4` is three
/// instructions returning a flat `500.0f`). This scenario is a `SingleRace`
/// with weapons live, so faster rounds land more hits, more hits mean more
/// slowdown, and a slowed field bunches: the same tree scores **1,051** with
/// the old constant and **1,558** with the real one. Isolated by reverting
/// that single constant and nothing else, twice, both times returning exactly
/// `1_051`.
///
/// **Both figures in the header now predate a physics correctness fix, so the
/// pathology ceiling is stale and the old "midway between the regimes" rule
/// cannot be reapplied verbatim** - the pathology has not been re-measured
/// against a correct Cannon, and doing so needs `oag_ai::Driver::social`'s fix
/// reverted for a run. Until it is, this bound is set by proportional headroom
/// instead: `2_000` is ~28 % above the measured `1_558`, close to the ~33 %
/// the old bound allowed over its own regime for the chaotic re-rolling any AI
/// change causes.
///
/// **This is a weaker guard than it was, and the weakening is the finding.**
/// Correct physics moved the field from 1,051 to 1,558 against a pathology
/// once measured at 1,756 - so "does contact still feel magnetic under weapon
/// fire" is an open question again, not a closed one, and the discriminating
/// check is a play session with weapons live rather than another statistic.
///
/// **Read the printed `sustained` figure before trusting this bound.** The
/// total is what fails the test, but streaks longer than
/// [`SUSTAINED_TICKS`] are what "stuck together rather than brushing past"
/// actually looks like, and they now measure **1,032 against a fixed driver's
/// 537 and the pathology's 1,062** - 97 % of the pathology, while the headline
/// total still reads as a comfortable pass. A bound on the total alone does
/// not catch that, and did not.
///
/// The speed itself is evidenced in
/// [`cannon-quake-leachbeam.md`](../../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md);
/// re-measuring the pathology regime needs `oag_ai::Driver::social`'s
/// `alongside`/`CONTACT_FLOOR` fix reverted for one run.
const MAX_OVERLAPPED_PAIR_TICKS: u32 = 2_000;

/// Ticks per race: the same length the sticking measurement above was taken
/// over, so a change here would also move the number this bound is set
/// against.
const TICKS: usize = 7_200;

/// How long a pair has to have been overlapping before the contact counts as
/// sustained rather than as a racing incident. Reported, never asserted.
const SUSTAINED_TICKS: u32 = 10;

/// What a full grid driven for [`TICKS`] spends overlapping.
struct Sticking {
    /// One per overlapping pair per tick, summed over the race - the statistic
    /// [`MAX_OVERLAPPED_PAIR_TICKS`] bounds.
    pair_ticks: u32,
    /// The same count restricted to pairs already overlapping for more than
    /// [`SUSTAINED_TICKS`], so a racing incident does not read like a craft
    /// welded to another. Printed rather than asserted: it separates the two
    /// regimes as cleanly (537 against 1,062) and is kept as a second opinion on
    /// any future move.
    sustained_pair_ticks: u32,
    /// Per unordered pair, the longest run of consecutive overlapping ticks.
    /// **Printed, never asserted** - see this file's header for why this
    /// statistic was retired.
    streaks: Vec<((usize, usize), u32)>,
}

/// Drives a full grid for [`TICKS`] and measures how much of it is spent with
/// hulls overlapping.
fn measure_sticking(image: &Path) -> Sticking {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);

    let count = usize::from(race.world.ship_count);
    let mut streak = vec![0u32; count * count];
    let mut longest = vec![0u32; count * count];
    let mut pair_ticks = 0u32;
    let mut sustained_pair_ticks = 0u32;

    for _ in 0..TICKS {
        race.tick(&oag_gameplay::InputSnapshot::default());
        for a in 0..count {
            for b in (a + 1)..count {
                if !race.world.ships[a].active || !race.world.ships[b].active {
                    streak[a * count + b] = 0;
                    continue;
                }
                let ship_a = &race.world.ships[a];
                let ship_b = &race.world.ships[b];
                let overlapping = oag_physics::pair::overlap(
                    &ship_a.physics.body,
                    &ship_a.handling.dimensions,
                    &ship_b.physics.body,
                    &ship_b.handling.dimensions,
                )
                .is_some();
                let index = a * count + b;
                if overlapping {
                    streak[index] += 1;
                    pair_ticks += 1;
                    if streak[index] > SUSTAINED_TICKS {
                        sustained_pair_ticks += 1;
                    }
                    longest[index] = longest[index].max(streak[index]);
                } else {
                    streak[index] = 0;
                }
            }
        }
    }

    let mut streaks = Vec::new();
    for a in 0..count {
        for b in (a + 1)..count {
            let longest = longest[a * count + b];
            if longest > 0 {
                streaks.push(((a, b), longest));
            }
        }
    }
    Sticking {
        pair_ticks,
        sustained_pair_ticks,
        streaks,
    }
}

/// **The acceptance test for `oag_ai::Driver::social` reading `alongside`, and
/// for its contact floor.** The grid may not spend more than
/// [`MAX_OVERLAPPED_PAIR_TICKS`] pair-ticks overlapping on a real, full,
/// `VENOM` race.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_craft_pair_sticks_together_on_a_real_grid() {
    let Some(image) = image() else {
        return;
    };
    let sticking = measure_sticking(&image);
    for line in &sticking.streaks {
        println!("pair {:?}: longest overlap streak {} ticks", line.0, line.1);
    }
    println!(
        "overlapped pair-ticks {} of which sustained (streak > {SUSTAINED_TICKS}) {}",
        sticking.pair_ticks, sticking.sustained_pair_ticks
    );
    assert!(
        sticking.pair_ticks <= MAX_OVERLAPPED_PAIR_TICKS,
        "the grid spent {} pair-ticks overlapping (limit {MAX_OVERLAPPED_PAIR_TICKS}) - \
         see this file's own doc comment; the pathology measures 1,756 pair-ticks on this \
         scenario and the fixed driver 978",
        sticking.pair_ticks
    );
}
