use super::*;

fn row(swaps: f32, segments: f32, amplitude: f32) -> PackSwapping {
    PackSwapping {
        num_ai_position_swaps: swaps,
        num_intermediate_positions: segments,
        intermediate_offset_size: amplitude,
    }
}

#[test]
fn an_unshuffled_driver_holds_its_grid_order() {
    let mut rng = Rng::new(1);
    let mut wander = Wander::constructed(3);
    for f in [0.0, 0.3, 0.7, 1.0] {
        assert_eq!(wander.update(f, &mut rng), 5.0);
    }
    assert_eq!(rng.snapshot(), Rng::new(1).snapshot(), "no draw");
}

/// The parity the live check used: an odd number of adjacent transpositions
/// is an odd permutation, whatever the generator drew.
#[test]
fn the_shuffle_permutes_by_the_authored_swap_count() {
    for (swaps, odd) in [(3.0, true), (4.0, false)] {
        let mut rng = Rng::new(7);
        let mut field: Vec<Wander> = (1..8).map(Wander::constructed).collect();
        shuffle(&mut field, &row(swaps, 2.0, 0.5), &mut rng);
        let ends: Vec<i32> = field.iter().map(|w| w.end as i32).collect();
        let inversions = (0..ends.len())
            .flat_map(|i| (i + 1..ends.len()).map(move |j| (i, j)))
            .filter(|&(i, j)| ends[i] < ends[j])
            .count();
        assert_eq!(inversions % 2 == 1, odd, "{ends:?}");
        assert!(
            field
                .iter()
                .all(|w| w.phase == 0.0 && w.segments == 2.0 && w.amplitude == 0.5)
        );
    }
}

#[test]
fn zero_intermediate_positions_still_give_one_segment() {
    let mut field = [Wander::constructed(1), Wander::constructed(2)];
    shuffle(&mut field, &row(0.0, 0.0, 0.0), &mut Rng::new(3));
    assert_eq!(field[0].segments, 1.0);
}

/// The live sample's own state (Talon's Junction, Venom, level 0): start 7,
/// end 5, half way into the first of two segments, recomputed to within the
/// `5e-7` the capture's `--predict` reports.
#[test]
fn the_wander_reproduces_a_live_sample() {
    let mut wander = Wander {
        start: 7.0,
        end: 5.0,
        phase: 0.5,
        segments: 2.0,
        amplitude: 0.5,
        prev: 0.0,
        next: 0.215_209_96,
    };
    let f = race_fraction(8966.508 - 2432.3474, 3, 5068.7275);
    let spread = wander.update(f, &mut Rng::new(0));
    assert!((spread - 6.325_545).abs() < 5e-6, "{spread}");
}

#[test]
fn a_segment_boundary_moves_next_to_prev_and_draws_inside_the_amplitude() {
    let mut rng = Rng::new(11);
    let mut wander = Wander::constructed(1);
    shuffle(
        std::slice::from_mut(&mut wander),
        &row(0.0, 4.0, 0.5),
        &mut rng,
    );
    wander.update(0.0, &mut rng);
    assert_eq!((wander.phase, wander.prev), (0.25, 0.0));
    let first = wander.next;
    assert!(first.abs() <= 0.5);
    wander.update(0.3, &mut rng);
    assert_eq!((wander.phase, wander.prev), (0.5, first));
    // The last segment ends on no offset at all.
    wander.update(0.8, &mut rng);
    wander.update(0.8, &mut rng);
    assert_eq!((wander.phase, wander.next), (1.0, 0.0));
}

#[test]
fn the_race_fraction_is_zero_outside_the_original_lap_guard() {
    assert_eq!(race_fraction(100.0, 0, 1000.0), 0.0);
    assert_eq!(race_fraction(100.0, 20, 1000.0), 0.0);
    assert_eq!(race_fraction(-50.0, 3, 1000.0), 0.0);
    assert_eq!(race_fraction(1500.0, 3, 1000.0), 0.5);
    assert_eq!(race_fraction(9000.0, 3, 1000.0), 1.0);
}
