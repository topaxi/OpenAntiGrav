use super::*;

fn input(own_lane: f32, lane_tick: bool) -> PlanInput {
    PlanInput {
        own_lane,
        lane_tick,
        contact_sides: 0,
        band: None,
    }
}

#[test]
fn the_ring_reads_the_slot_at_a_progress_and_drops_what_was_passed() {
    let mut ring = Ring::default();
    ring.advance(0.5);
    ring.reset(0.0);
    for i in 1..RING {
        ring.push(i as f32);
    }
    assert!(ring.is_full());
    assert_eq!(ring.at(0.5), 0.0);
    assert_eq!(ring.at(0.5 + 10.5 / 4000.0), 10.0);
    // Past the queue reads the newest slot, behind it the head.
    assert_eq!(ring.at(0.6), 79.0);
    assert_eq!(ring.at(0.49), 0.0);
    ring.advance(0.5 + 5.5 / 4000.0);
    assert_eq!((ring.at(0.5 + 5.5 / 4000.0), ring.count), (5.0, 75));
}

#[test]
fn the_ring_wraps_across_the_start_line() {
    let mut ring = Ring::default();
    ring.advance(0.999);
    ring.reset(0.25);
    for i in 1..RING {
        ring.push(i as f32);
    }
    // 0.999 is slot 3996; 0.0005 is slot 2, six slots on.
    assert_eq!(ring.at(0.0005), 6.0);
    ring.advance(0.0005);
    assert_eq!((ring.at(0.0005), ring.count), (6.0, 74));
}

#[test]
fn passing_the_whole_queue_leaves_one_zero_slot() {
    let mut ring = Ring::default();
    ring.advance(0.1);
    ring.reset(0.7);
    ring.advance(0.2);
    assert_eq!((ring.at(0.2), ring.count), (0.0, 1));
}

#[test]
fn a_bump_peaks_at_its_weight_and_reaches_nothing_past_twice_its_width() {
    let mut scores = [0.0; 9];
    add_bump(&mut scores, 0.0, 0.25, 0.5);
    assert_eq!(scores[4], 0.5, "full weight at the centre");
    assert!(scores[3] > 0.0 && scores[3] < 0.5);
    assert_eq!(scores[2], 0.0, "two half-widths out");
    assert_eq!(scores[0], 0.0);
    let mut none = [0.0; 9];
    add_bump(&mut none, 0.0, 1.0, 0.001);
    assert_eq!(none, [0.0; 9], "a weight at or under 0.001 adds nothing");
}

#[test]
fn the_first_plan_starts_the_ring_on_the_craft_and_fills_it_rate_limited() {
    let mut planner = Planner::default();
    planner.ring.advance(0.3);
    planner.plan(&input(0.8, false), &mut Rng::new(1));
    assert!(planner.ring.is_full());
    assert_eq!(planner.ring.at(0.3), 0.8);
    let mut previous = 0.8;
    for slot in 1..RING {
        let lane = planner.ring.at(0.3 + (slot as f32 + 0.5) / 4000.0);
        assert!(
            (lane - previous).abs() <= 0.015 + 1e-6,
            "slot {slot}: {previous} -> {lane}"
        );
        previous = lane;
    }
    // Until its first choice the target is where the craft is; the fill
    // dithers about it by the brake step, as the original's does.
    assert!((previous - 0.8).abs() < 0.002, "{previous}");
    // A choice moves the target to the line; the newest slots head for it.
    planner.ring.advance(0.3 + 40.5 / 4000.0);
    planner.plan(&input(0.8, true), &mut Rng::new(1));
    assert_eq!(planner.target, 0.0);
    assert!(planner.ring.last() < 0.8);
}

#[test]
fn the_choice_settles_on_the_racing_line_with_no_preferred_lane() {
    let mut planner = Planner::default();
    let mut rng = Rng::new(5);
    planner.ring.advance(0.0);
    planner.plan(&input(0.6, true), &mut rng);
    assert_eq!(
        planner.target, 0.0,
        "the line and a preferred lane of 0 agree"
    );
}

#[test]
fn a_contact_on_the_left_pushes_the_target_right_and_decays() {
    let mut planner = Planner::default();
    let mut rng = Rng::new(2);
    planner.ring.advance(0.0);
    planner.plan(
        &PlanInput {
            contact_sides: 1,
            ..input(0.0, false)
        },
        &mut rng,
    );
    assert!((planner.target - 0.2).abs() < 1e-6, "{}", planner.target);
    planner.plan(&input(0.0, false), &mut rng);
    assert!((planner.target - 0.19).abs() < 1e-6, "{}", planner.target);
}

/// The band keeps the target **out**: the decompile and the instructions at
/// `0x08855708..0x0885571c` cap a target below the band's middle at `lo`.
#[test]
fn the_authored_band_is_kept_out_of_on_the_side_first_seen() {
    let lo = 140_u8; // ~0.098
    let hi = 200_u8; // ~0.569
    let mut planner = Planner::default();
    let mut rng = Rng::new(3);
    planner.ring.advance(0.0);
    planner.plan(
        &PlanInput {
            contact_sides: 1,
            band: Some((lo, hi)),
            ..input(0.0, false)
        },
        &mut rng,
    );
    let lo_f = f32::from(lo) / 127.5 - 1.0;
    assert_eq!(
        planner.target, lo_f,
        "0.2 pushed, under the middle, capped at lo"
    );
    // Held below while the band lasts, even pushed past its middle.
    planner.plan(
        &PlanInput {
            contact_sides: 3,
            band: Some((lo, hi)),
            ..input(0.0, false)
        },
        &mut rng,
    );
    assert_eq!(planner.target, lo_f);
    // The shipped default disables it.
    planner.plan(
        &PlanInput {
            band: Some((0xff, 0x00)),
            ..input(0.0, false)
        },
        &mut rng,
    );
    assert!(planner.target > lo_f);
}

#[test]
fn a_target_starting_above_the_middle_is_lifted_to_hi() {
    let mut planner = Planner::default();
    let mut rng = Rng::new(3);
    planner.ring.advance(0.0);
    planner.plan(
        &PlanInput {
            contact_sides: -2,
            band: Some((10, 140)),
            ..input(0.0, false)
        },
        &mut rng,
    );
    assert_eq!(planner.target, 140.0 / 127.5 - 1.0);
}

#[test]
fn the_preferred_lane_is_drawn_rarely_and_leans_to_the_edges() {
    let mut planner = Planner::default();
    let mut rng = Rng::new(9);
    planner.ring.advance(0.0);
    let mut seen = Vec::new();
    for _ in 0..20_000 {
        let before = planner.preferred;
        planner.plan(&input(0.0, false), &mut rng);
        if planner.preferred != before {
            seen.push(planner.preferred);
        }
    }
    assert!(
        (25..=80).contains(&seen.len()),
        "about one in 400: {}",
        seen.len()
    );
    assert!(seen.iter().all(|l| l.abs() <= 1.0));
    let edges = seen.iter().filter(|l| l.abs() > 0.5).count();
    assert!(
        edges * 2 > seen.len(),
        "1 - u^2 leans outward: {edges} of {}",
        seen.len()
    );
}

#[test]
fn the_local_search_breaks_ties_toward_the_first_visited() {
    let planner = Planner {
        scores: [0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.3, 0.0, 0.0],
        ..Planner::default()
    };
    // From candidate 5 the upward pass meets 6 first.
    assert_eq!(planner.local_best(0.25), 6);
    // From candidate 8 only the downward pass runs, and meets 6 first.
    assert_eq!(planner.local_best(1.0), 6);
    // From 0 the upward pass meets 2 first.
    assert_eq!(planner.local_best(-1.0), 2);
}
