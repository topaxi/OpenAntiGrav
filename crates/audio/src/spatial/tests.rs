use super::*;

/// The listener used everywhere below: at the origin, `+X` to its right.
fn ears() -> Listener {
    Listener::at_origin()
}

#[test]
fn a_source_on_top_of_the_listener_is_centred_and_loud() {
    let placed = Emitter::craft([0.0; 3]).place(&ears(), 1.0).unwrap();
    assert_eq!(placed.pan, 0.0);
    assert_eq!(placed.gain, 1.0);
}

#[test]
fn the_near_field_is_flat_out_to_a_fifth_of_the_radius() {
    // `atten * 1.25` clamps at one, so everything inside 0.2 * radius is full
    // volume. 200 * 0.2 = 40.
    let ears = ears();
    for x in [0.0, 10.0, 25.0, 39.0, 40.0] {
        let placed = Emitter::craft([x, 0.0, 0.0]).place(&ears, 1.0).unwrap();
        assert_eq!(placed.gain, 1.0, "{x} units away should still be full");
    }
    let just_outside = Emitter::craft([41.0, 0.0, 0.0]).place(&ears, 1.0).unwrap();
    assert!(
        just_outside.gain < 1.0,
        "past 40 units the falloff has to start"
    );
}

#[test]
fn past_the_radius_the_emitter_refuses_rather_than_going_quiet() {
    let ears = ears();
    assert!(
        Emitter::craft([Emitter::CRAFT_RADIUS - 0.5, 0.0, 0.0])
            .place(&ears, 1.0)
            .is_some()
    );
    assert!(
        Emitter::craft([Emitter::CRAFT_RADIUS + 0.5, 0.0, 0.0])
            .place(&ears, 1.0)
            .is_none(),
        "the original gates on d > radius separately from the attenuation"
    );
}

#[test]
fn the_engine_radius_is_a_quarter_of_the_crafts() {
    // Not a tuning choice: `ExhaustFlare_Init` writes 50.0 over the 200.0
    // default. A distance audible for a collision is silent for an engine.
    let ears = ears();
    let at = [120.0, 0.0, 0.0];
    assert!(Emitter::craft(at).place(&ears, 1.0).is_some());
    assert!(Emitter::engine(at).place(&ears, 1.0).is_none());
}

#[test]
fn the_falloff_is_linear_before_the_curve_is_applied() {
    // Halfway out, the raw attenuation is (200 - 100)/200 * 1.25 = 0.625, and
    // the curve is the only thing between that and the gain.
    let placed = Emitter::craft([100.0, 0.0, 0.0])
        .place(&ears(), 1.0)
        .unwrap();
    let expected = volume_curve(0.625);
    assert_eq!(placed.gain, expected);
    assert!(
        placed.gain > 0.625,
        "gamma 1.7 raises a partial level: {} should exceed the linear 0.625",
        placed.gain
    );
}

#[test]
fn the_volume_curve_is_quantised_to_256_truncated_steps() {
    // Two levels inside one step give the same answer, and the step boundary
    // moves it. This is the `(int)(v * 255)` truncation, not rounding.
    assert_eq!(volume_curve(0.5000), volume_curve(0.5010));
    assert_ne!(volume_curve(0.5000), volume_curve(0.5100));
    assert_eq!(volume_curve(0.0), 0.0);
    assert_eq!(volume_curve(1.0), 1.0);
    // Clamped, not wrapped or panicking.
    assert_eq!(volume_curve(-3.0), 0.0);
    assert_eq!(volume_curve(9.0), 1.0);
}

#[test]
fn a_source_to_the_right_pans_right() {
    let placed = Emitter::craft([10.0, 0.0, 0.0])
        .place(&ears(), 1.0)
        .unwrap();
    assert_eq!(placed.pan, 1.0);
    let [left, right] = pan_gains(placed.pan);
    assert_eq!(left, 0.0);
    assert_eq!(right, 1.0);
}

#[test]
fn a_source_to_the_left_pans_left() {
    let placed = Emitter::craft([-10.0, 0.0, 0.0])
        .place(&ears(), 1.0)
        .unwrap();
    assert_eq!(placed.pan, -1.0);
    let [left, right] = pan_gains(placed.pan);
    assert_eq!(left, 1.0);
    assert_eq!(right, 0.0);
}

#[test]
fn ahead_and_behind_are_both_centred() {
    // `acos` is even, so the original cannot tell front from back either. That
    // is correct for two speakers and is not an approximation introduced here.
    let ears = ears();
    let ahead = Emitter::craft([0.0, 0.0, -10.0]).place(&ears, 1.0).unwrap();
    let behind = Emitter::craft([0.0, 0.0, 10.0]).place(&ears, 1.0).unwrap();
    assert_eq!(ahead.pan, behind.pan);
    assert_eq!(ahead.pan, 0.0);
}

#[test]
fn a_centred_source_is_minus_three_decibels_in_each_channel() {
    // The original's own table says 0.7071 at its midpoint. Anything that
    // "fixes" this to 1.0 has left the recovered curve.
    let [left, right] = pan_gains(0.0);
    assert!((left - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert_eq!(left, right);
}

#[test]
fn the_pan_is_equal_power_everywhere() {
    // Sum of squares is one at every position, which is what "equal power"
    // means and what `cos^2 + sin^2` guarantees. The disc's table is exactly
    // this curve at half-degree steps.
    for step in 0..=200u32 {
        let pan = f32::from(step as u16) / 100.0 - 1.0;
        let [left, right] = pan_gains(pan);
        let power = left * left + right * right;
        assert!(
            (power - 1.0).abs() < 1e-5,
            "pan {pan} gives power {power}, not 1"
        );
    }
}

#[test]
fn the_pan_curve_matches_the_discs_own_table() {
    // The disc holds 180 pairs of `floor(16383 * cos(i / 2 degrees))` and the
    // matching sine. The chain's two 90-degree rotations cancel, so table entry
    // `i` is reached when the projection onto the right axis is `-cos(i
    // degrees)` - the half-angle this module's square roots compute. Every
    // entry is reproduced to within the one unit the table's own `floor` costs.
    //
    // This is the same check the evidence page ran over all 360 values, kept
    // here so a change to the pan law fails a test rather than only a reading.
    const SCALE: f32 = 16383.0;
    for index in 0..180u32 {
        let table = [
            (SCALE * (f32::from(index as u16) * 0.5).to_radians().cos()).floor(),
            (SCALE * (f32::from(index as u16) * 0.5).to_radians().sin()).floor(),
        ];
        let [left, right] = pan_gains(-f32::from(index as u16).to_radians().cos());
        for (ours, theirs) in [(left, table[0]), (right, table[1])] {
            let delta = ours * SCALE - theirs;
            assert!(
                (0.0..=1.05).contains(&delta),
                "index {index}: {} against the table's {theirs}",
                ours * SCALE
            );
        }
    }
}

#[test]
fn a_rotated_listener_pans_by_its_own_right_axis() {
    // The listener is the camera, so a camera looking the other way swaps the
    // channels. This is the only thing the `right` field does.
    let turned = Listener {
        position: [0.0; 3],
        right: [-1.0, 0.0, 0.0],
    };
    let placed = Emitter::craft([10.0, 0.0, 0.0])
        .place(&turned, 1.0)
        .unwrap();
    assert_eq!(placed.pan, -1.0);
}

#[test]
fn a_listener_away_from_the_origin_measures_from_itself() {
    let ears = Listener {
        position: [1000.0, 0.0, 0.0],
        right: [1.0, 0.0, 0.0],
    };
    assert!(
        Emitter::craft([1000.0, 0.0, 0.0])
            .place(&ears, 1.0)
            .is_some(),
        "a coincident source is in range wherever the listener stands"
    );
    assert!(Emitter::craft([0.0; 3]).place(&ears, 1.0).is_none());
}

#[test]
fn the_call_sites_volume_multiplies_before_the_curve() {
    // Not after: the original passes its volume into the attenuation and runs
    // the product through the table once. Halving the request is therefore not
    // the same as halving the result, and that difference is the curve.
    let full = Emitter::craft([100.0, 0.0, 0.0])
        .place(&ears(), 1.0)
        .unwrap();
    let half = Emitter::craft([100.0, 0.0, 0.0])
        .place(&ears(), 0.5)
        .unwrap();
    assert!(half.gain < full.gain);
    assert!(
        half.gain > full.gain * 0.5,
        "the gamma curve has to lift the halved level above a linear halving"
    );
}

#[test]
fn a_zero_radius_emitter_is_refused_rather_than_dividing_by_zero() {
    let silent = Emitter {
        position: [0.0; 3],
        radius: 0.0,
        cone: None,
    };
    assert!(silent.place(&ears(), 1.0).is_none());
    let nonsense = Emitter {
        position: [0.0; 3],
        radius: f32::NAN,
        cone: None,
    };
    assert!(nonsense.place(&ears(), 1.0).is_none());
}

#[test]
fn dead_ahead_of_the_cone_is_unattenuated_by_it() {
    // The axis points at `+Z`; a source on `+Z` from the emitter is on-axis,
    // so the cone term is `1 - 0/half_angle == 1` and changes nothing.
    let cone = Emitter {
        position: [0.0, 0.0, 10.0],
        radius: 200.0,
        cone: Some(Cone {
            axis: [0.0, 0.0, 1.0],
            half_angle: std::f32::consts::FRAC_PI_4,
        }),
    };
    let omni = Emitter {
        position: [0.0, 0.0, 10.0],
        radius: 200.0,
        cone: None,
    };
    assert_eq!(
        cone.place(&ears(), 1.0).unwrap().gain,
        omni.place(&ears(), 1.0).unwrap().gain,
        "on-axis, a cone should match the same emitter with none"
    );
}

#[test]
fn outside_the_cones_half_angle_it_is_silent() {
    // The axis points at `+Z`; the listener sits on `+X`, 90 degrees off-axis,
    // well past a 45-degree half-angle. `1 - angle/half_angle` goes negative
    // and the clamp inside `volume_curve` is what turns that into zero.
    let cone = Emitter {
        position: [10.0, 0.0, 0.0],
        radius: 200.0,
        cone: Some(Cone {
            axis: [0.0, 0.0, 1.0],
            half_angle: std::f32::consts::FRAC_PI_4,
        }),
    };
    assert_eq!(cone.place(&ears(), 1.0).unwrap().gain, 0.0);
}

#[test]
fn a_wider_cone_is_still_audible_where_a_narrower_one_is_not() {
    // The axis points at `+Z`; the listener sits 30 degrees off it, so a
    // 10-degree half-angle cone misses and an 80-degree one does not.
    let angle = 30.0_f32.to_radians();
    let listener = Listener {
        position: [-angle.sin() * 20.0, 0.0, -angle.cos() * 20.0],
        right: [1.0, 0.0, 0.0],
    };
    let axis = [0.0, 0.0, 1.0];
    let narrow = Emitter {
        position: [0.0; 3],
        radius: 200.0,
        cone: Some(Cone {
            axis,
            half_angle: 10.0_f32.to_radians(),
        }),
    };
    let wide = Emitter {
        position: [0.0; 3],
        radius: 200.0,
        cone: Some(Cone {
            axis,
            half_angle: 80.0_f32.to_radians(),
        }),
    };
    assert_eq!(narrow.place(&listener, 1.0).unwrap().gain, 0.0);
    assert!(wide.place(&listener, 1.0).unwrap().gain > 0.0);
}

/// `pitch = -(dd/dt) * 0.0005 * 1536`: closing sharpens, receding flattens,
/// by `2^(rate * 0.0005)` either way.
#[test]
fn doppler_sharpens_a_closing_source_and_flattens_a_receding_one() {
    let dt = 1.0 / 60.0;
    let mut doppler = Doppler::default();
    assert_eq!(
        doppler.ratio(100.0, dt, true),
        1.0,
        "the first frame has no change to read"
    );
    // 100 units closer in one frame is 6,000 units a second.
    let closing = doppler.ratio(0.0, dt, true);
    assert!(
        (closing - 2f32.powf(6000.0 * DOPPLER_SCALE)).abs() < 1e-4,
        "{closing}"
    );
    let receding = doppler.ratio(100.0, dt, true);
    assert!(
        (receding - 2f32.powf(-6000.0 * DOPPLER_SCALE)).abs() < 1e-4,
        "{receding}"
    );
}

/// A suppressed frame is unity but still records the distance, so the frame
/// after it measures one frame's change and not two.
#[test]
fn a_suppressed_frame_holds_unity_and_does_not_double_the_next() {
    let dt = 1.0 / 60.0;
    let mut doppler = Doppler::default();
    doppler.ratio(100.0, dt, true);
    assert_eq!(doppler.ratio(50.0, dt, false), 1.0);
    let next = doppler.ratio(0.0, dt, true);
    assert!(
        (next - 2f32.powf(3000.0 * DOPPLER_SCALE)).abs() < 1e-4,
        "{next}"
    );
    doppler.reset();
    assert_eq!(doppler.ratio(500.0, dt, true), 1.0);
}

/// `mgr+0x94`: a listener that moved more than 24 units in a frame cut, it
/// did not travel.
#[test]
fn a_listener_that_jumped_more_than_the_threshold_is_a_cut() {
    let before = Listener::at_origin();
    let mut after = Listener::at_origin();
    after.position = [23.0, 0.0, 0.0];
    assert!(!after.jumped_from(&before));
    after.position = [25.0, 0.0, 0.0];
    assert!(after.jumped_from(&before));
}
