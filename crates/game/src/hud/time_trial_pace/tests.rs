//! [`TimeTrialPace::from_elapsed`]'s own ladder, pinned tick-by-tick rather
//! than centisecond-by-centisecond, since ticks are what a `Race` actually
//! hands it.

use oag_tables::race_campaign::{Cell, Medal, Mode};

use super::TimeTrialPace;

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

/// The whole race, ticks chosen so `elapsed_centis` lands on an exact
/// integer either side of every boundary - see the module doc for the
/// arithmetic.
#[test]
fn gold_from_the_green_flag() {
    let pace = TimeTrialPace::from_elapsed(0, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Gold);
    assert_eq!(pace.remaining_ticks, 6900);
    assert!(!pace.missed);
}

/// `elapsed <= gold` is inclusive: the tick that exactly meets the target is
/// still gold pace, not silver.
#[test]
fn gold_holds_through_its_own_boundary_tick() {
    let pace = TimeTrialPace::from_elapsed(6900, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Gold);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

#[test]
fn one_tick_past_gold_is_silver() {
    let pace = TimeTrialPace::from_elapsed(6901, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Silver);
    assert_eq!(pace.remaining_ticks, 179);
    assert!(!pace.missed);
}

#[test]
fn silver_holds_through_its_own_boundary_tick() {
    let pace = TimeTrialPace::from_elapsed(7080, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Silver);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

#[test]
fn one_tick_past_silver_is_bronze() {
    let pace = TimeTrialPace::from_elapsed(7081, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Bronze);
    assert_eq!(pace.remaining_ticks, 299);
    assert!(!pace.missed);
}

/// The last tick bronze is still makeable - the medal a race finishing here
/// would actually earn, [`Cell::evaluate_medal`]'s own `<=` rule mirrored.
#[test]
fn bronze_holds_through_its_own_boundary_tick() {
    let pace = TimeTrialPace::from_elapsed(7380, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(!pace.missed);
}

/// Past every target: **stuck at bronze, reddened, nothing left to show** -
/// not `None`. This is the one tick the "stateful vs. stateless" proof in
/// [`TimeTrialPace::from_elapsed`]'s own doc comment is about: the original
/// would have frozen its tier field on the last tick bronze was still in
/// reach, which this function re-derives instead of remembering.
#[test]
fn past_every_target_stays_bronze_and_reddens() {
    let pace = TimeTrialPace::from_elapsed(7381, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(pace.missed);
}

/// Long after the fact, still bronze and reddened - not a panic on an
/// underflow and not some other medal a naive "last one before this"
/// re-derivation might drift onto.
#[test]
fn staying_missed_forever_after() {
    let pace = TimeTrialPace::from_elapsed(60_000, &grid0_3_2());
    assert_eq!(pace.medal, Medal::Bronze);
    assert_eq!(pace.remaining_ticks, 0);
    assert!(pace.missed);
}
