use super::*;

#[test]
fn the_points_table_is_the_recovered_one() {
    // 1st through 8th: 8, 6, 5, 4, 3, 2, 1, 0.
    assert_eq!(POINTS_BY_POSITION, [8, 6, 5, 4, 3, 2, 1, 0]);
    for (place, &expected) in POINTS_BY_POSITION.iter().enumerate() {
        let place = u8::try_from(place + 1).unwrap();
        assert_eq!(points_for_finish(place, true), expected);
    }
}

#[test]
fn a_craft_that_did_not_finish_scores_nothing_regardless_of_place() {
    assert_eq!(points_for_finish(1, false), 0);
    assert_eq!(points_for_finish(8, false), 0);
}

#[test]
fn a_place_outside_the_grid_scores_nothing_rather_than_panicking() {
    assert_eq!(points_for_finish(0, true), 0);
    assert_eq!(points_for_finish(9, true), 0);
}

#[test]
fn totals_accumulate_across_legs() {
    let mut standings = Standings::<4>::new();
    standings.record_leg(0, &[1, 2, 3, 4], &[true, true, true, true]);
    standings.record_leg(1, &[4, 3, 2, 1], &[true, true, true, true]);
    // slot 0: 8 (1st) + 4 (4th) = 12; slot 3: 4 (4th) + 8 (1st) = 12 - a tie.
    assert_eq!(standings.totals(), [12, 11, 11, 12]);
}

#[test]
fn recording_the_same_leg_twice_overwrites_rather_than_double_counts() {
    let mut standings = Standings::<2>::new();
    standings.record_leg(0, &[1, 2], &[true, true]);
    assert_eq!(standings.totals(), [8, 6]);
    // The finish transition re-enters, or `EndRace` re-reads the same board
    // - recording leg 0 again must not add a second time.
    standings.record_leg(0, &[1, 2], &[true, true]);
    assert_eq!(standings.totals(), [8, 6]);
}

#[test]
fn a_tie_in_total_points_is_broken_by_slot_order_never_swapped() {
    let mut standings = Standings::<3>::new();
    // Slots 0 and 2 tie; slot 0 led coming in (lower slot index) and must
    // stay ahead - the original's bubble sort never swaps an exact tie.
    standings.record_leg(0, &[1, 3, 1], &[true, true, true]);
    let ranks = standings.ranks();
    assert!(
        ranks[0] < ranks[2],
        "slot 0 should rank ahead of slot 2 on a tie"
    );
    assert_eq!(ranks[1], 3, "slot 1 has the fewest points and ranks last");
}

#[test]
fn ranks_are_a_permutation_of_one_through_n() {
    let mut standings = Standings::<5>::new();
    standings.record_leg(0, &[3, 1, 5, 2, 4], &[true, true, true, true, true]);
    let mut ranks = standings.ranks();
    ranks.sort_unstable();
    assert_eq!(ranks, [1, 2, 3, 4, 5]);
}

#[test]
fn a_leg_recorded_out_of_order_backfills_the_gap_at_zero() {
    let mut standings = Standings::<2>::new();
    standings.record_leg(2, &[1, 2], &[true, true]);
    // Legs 0 and 1 were never recorded - they contribute nothing rather
    // than panicking on the gap.
    assert_eq!(standings.totals(), [8, 6]);
}
