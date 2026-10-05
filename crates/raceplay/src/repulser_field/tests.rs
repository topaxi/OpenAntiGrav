use super::*;

const DT: f32 = 1.0 / 60.0;

/// Steps a field `ticks` times with the waves starting after `blast` seconds,
/// returning `(scale, drawn alpha, spin)` per tick.
fn run(ticks: usize, blast: f32) -> Vec<(f32, f32, f32)> {
    let mut field = Field::new();
    (1..=ticks)
        .map(|tick| {
            let age = tick as f32 * DT;
            field.step(age, age > blast);
            (field.scale(), field.drawn_alpha(), field.spin)
        })
        .collect()
}

/// `+0x204` eases 1.0 toward 0.6 at 0.1 while above 0.61, then `+0x1f8` takes
/// over from 0.7 toward 4.0 at 0.05 - a jump, not a blend.
#[test]
fn the_ring_shrinks_then_jumps_and_widens_toward_four() {
    let ticks = run(96, 0.8);
    assert!((ticks[0].0 - 0.96).abs() < 1e-6, "first step of the shrink");
    // The switch tick, derived with the same f32 arithmetic as the field.
    let mut shrink = SHRINK_START;
    let mut shrinking = 0;
    while shrink > SHRINK_UNTIL {
        shrink = SHRINK.step(shrink);
        shrinking += 1;
    }
    assert!((30..40).contains(&shrinking), "{shrinking}");
    for (tick, &(scale, _, _)) in ticks.iter().enumerate().take(shrinking) {
        assert!(scale > 0.6 && scale < 1.0, "tick {tick}: {scale}");
    }
    let first_grow = ticks[shrinking].0;
    assert!((first_grow - 0.865).abs() < 1e-6, "{first_grow}");
    let last = ticks.last().unwrap().0;
    assert!(last > 3.5 && last < 4.0, "{last}");
}

/// Fade in at 0.2; the wave-start tick still draws the fade-in's value, and
/// the next draws 1.0 eased once toward 0 at 0.1.
#[test]
fn the_ring_fades_in_then_snaps_to_full_and_fades_out_once_the_waves_start() {
    let ticks = run(60, 0.5);
    assert!((ticks[0].1 - 0.2).abs() < 1e-6);
    let wave_tick = (1..=60).position(|t| t as f32 * DT > 0.5).unwrap();
    let before = ticks[wave_tick].1;
    assert!(before > 0.99 && before < 1.0, "fade-in value, {before}");
    assert!((ticks[wave_tick + 1].1 - 0.9).abs() < 1e-6);
    assert!((ticks[wave_tick + 2].1 - 0.81).abs() < 1e-6);
}

/// `+0x21c` holds at zero until the entity is older than 0.4 s, then eases
/// toward -2pi at 0.02.
#[test]
fn the_spin_waits_for_four_tenths_of_a_second() {
    let ticks = run(60, 0.8);
    for (tick, &(_, _, spin)) in ticks.iter().enumerate() {
        let age = (tick + 1) as f32 * DT;
        if age <= SPIN_AFTER_SECONDS {
            assert_eq!(spin, 0.0, "tick {tick}");
        } else {
            assert!(spin < 0.0 && spin > -std::f32::consts::TAU, "tick {tick}");
        }
    }
}

/// Up is the negated `down`, laying the ring's own `Y` on the road normal,
/// at the firer, scaled uniformly.
#[test]
fn the_matrix_lays_the_ring_on_the_road_at_the_firer() {
    let lateral = Vec3::new(1.0, 0.0, 0.0);
    let down = Vec3::new(0.0, -1.0, 0.0);
    let at = Vec3::new(5.0, 2.0, -3.0);
    let matrix = field_matrix(at, lateral, down, 2.0);
    assert!((matrix.y_axis.truncate() - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-6);
    assert!((matrix.w_axis.truncate() - at).length() < 1e-6);
    assert!((matrix.x_axis.truncate().length() - 2.0).abs() < 1e-6);
    assert!((matrix.z_axis.truncate().length() - 2.0).abs() < 1e-6);
    assert!(matrix.determinant() > 0.0, "not mirrored");
}
