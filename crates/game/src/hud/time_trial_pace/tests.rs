//! [`TimeTrialPace::evaluate`]'s own ladder, pinned tick-by-tick rather
//! than centisecond-by-centisecond, since ticks are what a `Race` actually
//! hands it.

use oag_tables::race_campaign::{Cell, Mode};

use super::{PaceTier, RecordTarget, TimeTrialPace};

/// `grid0_3_2`'s own authored targets, `Data\Plugins\grids\grid_00.xml` -
/// Talon's Junction Venom Time Trial, `Locked="false"` - read live off
/// `pulse-psp-usa.chd` while measuring `PlayerStatus_Update`'s own tier
/// ladder. Real numbers rather than round ones on purpose: a bug that only
/// shows up against `100`/`200`/`300` stays hidden with those.
fn grid0_3_2() -> Cell {
    Cell {
        name: "grid0_3_2".to_string(),
        track: Some("16_Track".to_string()),
        mode: Mode::TimeTrial,
        class: "Venom".to_string(),
        weapons: false,
        damage: false,
        locked: Some(false),
        status: None,
        ai_count: None,
        skill: None,
        skill_easy: None,
        skill_hard: None,
        laps: Some(3),
        ship: Some("None".to_string()),
        ship_choice: Some(true),
        gold: 11500,
        silver: 11800,
        bronze: 12300,
        tournament_tracks: Vec::new(),
        difficulty_targets: None,
        nitro_elimination_targets: None,
    }
}

/// `Talon's Junction`'s Venom figure, `stats.xml`'s `<RaceTimes Venom="117">`
/// (11700 cs), with no stored best - the fresh-profile case.
const VENOM_RACE: RecordTarget = RecordTarget {
    personal_best_centis: None,
    authored_centis: Some(11_700),
};

fn ladder_only(elapsed_ticks: u64) -> TimeTrialPace {
    TimeTrialPace::evaluate(elapsed_ticks, Some(&grid0_3_2()), &VENOM_RACE)
}

/// The whole race, ticks chosen so `elapsed_centis` lands on an exact
/// integer either side of every boundary - see the module doc for the
/// arithmetic.
#[test]
fn gold_from_the_green_flag() {
    let pace = ladder_only(0);
    assert_eq!(pace.tier, PaceTier::Gold);
    assert_eq!(pace.remaining_ticks, 6900);
    assert!(!pace.missed);
}

/// `elapsed <= gold` is inclusive: the tick that exactly meets the target is
/// still gold pace, not silver.
#[test]
fn gold_holds_through_its_own_boundary_tick() {
    let pace = ladder_only(6900);
    assert_eq!(pace.tier, PaceTier::Gold);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

#[test]
fn one_tick_past_gold_is_silver() {
    let pace = ladder_only(6901);
    assert_eq!(pace.tier, PaceTier::Silver);
    assert_eq!(pace.remaining_ticks, 179);
    assert!(!pace.missed);
}

#[test]
fn silver_holds_through_its_own_boundary_tick() {
    let pace = ladder_only(7080);
    assert_eq!(pace.tier, PaceTier::Silver);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

#[test]
fn one_tick_past_silver_is_bronze() {
    let pace = ladder_only(7081);
    assert_eq!(pace.tier, PaceTier::Bronze);
    assert_eq!(pace.remaining_ticks, 299);
    assert!(!pace.missed);
}

/// The last tick bronze is still makeable - the medal a race finishing here
/// would actually earn, [`Cell::evaluate_medal`]'s own `<=` rule mirrored.
#[test]
fn bronze_holds_through_its_own_boundary_tick() {
    let pace = ladder_only(7380);
    assert_eq!(pace.tier, PaceTier::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

/// Past every target: **stuck at bronze, reddened, nothing left to show** -
/// not `None`. This is the one tick the "stateful vs. stateless" proof in
/// [`TimeTrialPace::evaluate`]'s own doc comment is about: the original
/// would have frozen its tier field on the last tick bronze was still in
/// reach, which this function re-derives instead of remembering.
#[test]
fn past_every_target_stays_bronze_and_reddens() {
    let pace = ladder_only(7381);
    assert_eq!(pace.tier, PaceTier::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(pace.missed);
}

/// Long after the fact, still bronze and reddened - not a panic on an
/// underflow and not some other medal a naive "last one before this"
/// re-derivation might drift onto.
#[test]
fn staying_missed_forever_after() {
    let pace = ladder_only(60_000);
    assert_eq!(pace.tier, PaceTier::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(pace.missed);
}

/// No ladder, no stored best: `RECORD` from the green flag, counting down
/// from the authored 117.0 s - the live frame's `record 1.57.0` at zero
/// (`117.0 - 23.7 = 93.3` read as `1.33.2` at 23.7 s on the measured frame).
#[test]
fn a_plain_run_chases_the_authored_record() {
    let pace = TimeTrialPace::evaluate(0, None, &VENOM_RACE);
    assert_eq!(pace.tier, PaceTier::Record);
    assert_eq!(pace.remaining_ticks, 7020);
    assert!(!pace.missed);
}

/// Past the record the caption stays `RECORD` and the number reddens at zero,
/// measured live: `tier=3, redden=1` with the target clamped to `0`.
#[test]
fn a_plain_run_past_the_record_reddens_and_keeps_the_caption() {
    let pace = TimeTrialPace::evaluate(7021, None, &VENOM_RACE);
    assert_eq!(pace.tier, PaceTier::Record);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(pace.missed);
}

/// A stored best faster than the authored figure becomes the target, and one
/// slower does not - `min`, which the original takes when the store answers.
#[test]
fn the_smaller_of_the_stored_best_and_the_authored_time_is_chased() {
    let faster = RecordTarget {
        personal_best_centis: Some(10_000),
        ..VENOM_RACE
    };
    assert_eq!(
        TimeTrialPace::evaluate(0, None, &faster).remaining_ticks,
        6000
    );
    let slower = RecordTarget {
        personal_best_centis: Some(13_000),
        ..VENOM_RACE
    };
    assert_eq!(
        TimeTrialPace::evaluate(0, None, &slower).remaining_ticks,
        7020
    );
}

/// On a campaign cell a stored best faster than gold, with the run still
/// ahead of it, shows `RECORD` instead of `GOLD`.
#[test]
fn a_stored_best_faster_than_gold_shows_record_on_a_campaign_cell() {
    let best = RecordTarget {
        personal_best_centis: Some(11_000),
        ..VENOM_RACE
    };
    let pace = TimeTrialPace::evaluate(0, Some(&grid0_3_2()), &best);
    assert_eq!(pace.tier, PaceTier::Record);
    assert_eq!(pace.remaining_ticks, 6600);
    // Level with the stored best it falls back to the ladder: gold.
    let level = TimeTrialPace::evaluate(6600, Some(&grid0_3_2()), &best);
    assert_eq!(level.tier, PaceTier::Gold);
    // A stored best slower than gold changes nothing.
    let slow = RecordTarget {
        personal_best_centis: Some(12_000),
        ..VENOM_RACE
    };
    assert_eq!(
        TimeTrialPace::evaluate(0, Some(&grid0_3_2()), &slow).tier,
        PaceTier::Gold
    );
}

/// [`RecordTarget::new`] picks `<RaceTimes>` for a Time Trial and `<LapTimes>`
/// for a Speed Lap, by class, and the stored best from the matching field.
#[test]
fn the_target_reads_the_figure_and_the_best_that_belong_to_the_mode() {
    use crate::records::Record;
    let stats = oag_tables::track_stats::TrackStats {
        race_times: [117.0, 138.0, 119.0, 128.0],
        lap_times: [38.0, 33.0, 29.0, 25.0],
        elimination_target: None,
        zone_target: None,
        length: None,
        skill_scale: [[1.0, 2.0, 3.0]; 4],
        mode_modifiers: Default::default(),
    };
    let best = Record {
        best_total_ticks: Some(6_000),
        best_lap_ticks: Some(1_800),
        ..Record::default()
    };
    let tt = RecordTarget::new(
        oag_race::Mode::TimeTrial,
        "Venom",
        Some(&stats),
        Some(&best),
    )
    .expect("a target");
    assert_eq!(tt.authored_centis, Some(11_700));
    assert_eq!(tt.personal_best_centis, Some(10_000));
    let lap = RecordTarget::new(oag_race::Mode::SpeedLap, "flash", Some(&stats), Some(&best))
        .expect("a target");
    assert_eq!(lap.authored_centis, Some(3_300));
    assert_eq!(lap.personal_best_centis, Some(3_000));
    assert!(RecordTarget::new(oag_race::Mode::SingleRace, "venom", Some(&stats), None).is_none());
    // A class the file has no figure for, or no file at all: no authored time.
    let unknown = RecordTarget::new(oag_race::Mode::TimeTrial, "unknown", Some(&stats), None);
    assert_eq!(unknown.expect("a target").authored_centis, None);
    let unread = RecordTarget::new(oag_race::Mode::TimeTrial, "venom", None, Some(&best));
    assert_eq!(unread.expect("a target").authored_centis, None);
}

/// Where `stats.xml` did not read, a plain race keeps the plain clock - the
/// original's null track record - and a campaign cell still races its ladder.
#[test]
fn without_stats_a_plain_race_has_no_pace_and_a_campaign_cell_still_does() {
    let mode = oag_race::Mode::TimeTrial;
    let unread = RecordTarget::new(mode, "venom", None, None);
    assert_eq!(super::pace_for(mode, 0, 0, None, unread.as_ref()), None);
    let cell = grid0_3_2();
    let pace = super::pace_for(mode, 0, 0, Some(&cell), unread.as_ref()).expect("a pace");
    assert_eq!(pace.tier, PaceTier::Gold);
    assert_eq!(super::pace_for(mode, 0, 0, Some(&cell), None), Some(pace));
    // A mode the cluster is not shown in has none either way.
    assert_eq!(
        super::pace_for(oag_race::Mode::SingleRace, 0, 0, Some(&cell), None),
        None
    );
}
