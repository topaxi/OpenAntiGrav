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
//! The bound below is therefore a **regression tripwire, not a target**: set
//! comfortably above the measured post-fix worst (46) so this still passes
//! today, and comfortably below the pre-fix pathology (90) so a change that
//! walks it back that far fails here rather than needing another play
//! report to notice.
//!
//! # 2026-09-08: this test is RED, and the bound was deliberately left alone
//!
//! Letting an opponent fire a Cannon - `Race::advance_cannons` running every
//! slot, landed with the recovered gate on the same day - puts the worst
//! streak at **95**, over the bound. **The cause was measured rather than
//! guessed, and it is not sticking.** Three controls on this exact scenario:
//!
//! | opponent's Cannon behaviour | worst streak | pairs sticking |
//! | --- | --- | --- |
//! | held forever, never fired, never absorbed (the old behaviour) | 46 | 4 |
//! | countdown runs, no round spawns | 68 | 6 |
//! | absorbed instead of held (the most conservative alternative) | 68 | 6 |
//! | fired (what landed) | 95 | 7 |
//!
//! All eight craft finish the run `Racing` and active in every case, so no
//! craft is being wrecked and no pair is being pushed together by a round.
//! **Any change that stops opponents sitting on a useless Cannon for the whole
//! race moves this number**, including absorbing it - the old 46 was measured
//! in a regime where several opponents were frozen out of the weapon economy.
//!
//! **The bound is not raised, because raising it would retire the tripwire.**
//! Re-measuring the pathology *under the new regime* - the same run with
//! `Driver::social` reverted to reading only `Field::behind` - gives **90**,
//! against 95 for the fixed driver. So on this statistic the fix now reads as
//! very slightly *worse* than the bug, and no bound can separate them. Where
//! the fix still shows plainly is the distribution: pre-fix leaves 10 sticking
//! pairs with five over 25 ticks (90, 48, 47, 46, 29, 27, 25, 19, 9, 4);
//! post-fix leaves 7 with two (95, 72, 37, 16, 11, 8, 5).
//!
//! **So the work this file needs is a better statistic, not a bigger number** -
//! total overlapped pair-ticks, or the count of pairs over some floor, both of
//! which separate the two regimes cleanly. Changing it was out of scope for the
//! Cannon pass that found this, and doing it from outside would be redesigning
//! an acceptance test to make a change pass. Left red and reported instead.

use std::path::{Path, PathBuf};

use oag_game::race;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
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

/// How many consecutive ticks a pair may stay overlapped before this test
/// fails.
///
/// **Ours, a tripwire rather than a measurement of the original or a claim
/// about what "good" looks like** - the original has no equivalent driver
/// term to compare against (see `oag_ai::Driver::social`'s own doc comment),
/// so there is nothing to recover this from, and this file's own header
/// records that the post-fix measurement (46) is not zero and why. Chosen
/// above the measured post-fix worst so this passes today, and below the
/// pre-fix pathology (90) so a change that walks it back that far is caught.
/// No confidence score - there is nothing recovered to score against.
const MAX_STICKING_STREAK_TICKS: u32 = 60;

/// Ticks per race: the same length the sticking measurement above was taken
/// over, so a change here would also move the number this bound is set
/// against.
const TICKS: usize = 7_200;

/// Drives a full grid for [`TICKS`] and returns, per unordered pair of
/// craft, the longest run of consecutive ticks their hulls stayed
/// overlapped.
fn longest_overlap_streaks(image: &Path) -> Vec<((usize, usize), u32)> {
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
                    longest[index] = longest[index].max(streak[index]);
                } else {
                    streak[index] = 0;
                }
            }
        }
    }

    let mut result = Vec::new();
    for a in 0..count {
        for b in (a + 1)..count {
            let longest = longest[a * count + b];
            if longest > 0 {
                result.push(((a, b), longest));
            }
        }
    }
    result
}

/// **The acceptance test for `oag_ai::Driver::social` reading `alongside`.**
/// No pair may stay overlapped for [`MAX_STICKING_STREAK_TICKS`] consecutive
/// ticks straight on a real, full, `VENOM` grid.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_craft_pair_sticks_together_on_a_real_grid() {
    let Some(image) = image() else {
        return;
    };
    let streaks = longest_overlap_streaks(&image);
    for line in &streaks {
        println!("pair {:?}: longest overlap streak {} ticks", line.0, line.1);
    }
    let worst = streaks.iter().map(|&(_, streak)| streak).max().unwrap_or(0);
    assert!(
        worst <= MAX_STICKING_STREAK_TICKS,
        "pair(s) stayed overlapped for up to {worst} consecutive ticks (limit {MAX_STICKING_STREAK_TICKS}) - \
         see this file's own doc comment; the pre-fix measurement on this exact scenario was 90"
    );
}
