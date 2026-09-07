//! What [`super::GpuCost`] and [`super::CpuCost`] are asserted to say: which
//! rows appear at all, what the residual subtracts, and that the wall-clock
//! rows partition the frame time rather than merely sitting near it.
//!
//! Its own file rather than an inline `#[cfg(test)]` block for the reason
//! `perf/tests.rs` gives: past 200 lines a test module moves out. See
//! `scripts/check-file-size.py`.

use super::*;

/// The GPU cost rows say only what has actually been measured.
///
/// A reading arrives a frame or more after the frame it describes, so all
/// five are legitimately absent at the start of a run - and an
/// `FSR3 REN 0.00 MS` row on a frame the bilinear blit resolved would be a lie
/// that reads as "free" rather than as "not running". Each row appears only
/// once its own field has a number, in pipeline order, with `OTHER` last.
#[test]
fn the_gpu_cost_rows_show_only_what_was_measured() {
    assert_eq!(
        GpuCost::default().rows(11.76),
        Vec::<String>::new(),
        "nothing measured, no rows"
    );
    assert_eq!(
        GpuCost {
            scene: Some(0.004_2),
            blur: None,
            bloom: None,
            upscale: None,
            upscale_presented: None,
        }
        .rows(11.76),
        vec!["SCENE 4.20 MS".to_string(), "OTHER 7.6 MS".to_string()],
        "a scene reading alone must not imply blur, bloom or an upscaler ran"
    );
    assert_eq!(
        GpuCost {
            scene: Some(0.004_2),
            blur: Some(0.001_2),
            bloom: Some(0.003_1),
            upscale: Some(0.001_8),
            upscale_presented: None,
        }
        .rows(11.76),
        vec![
            "SCENE 4.20 MS".to_string(),
            "BLOOM 3.10 MS".to_string(),
            "BLUR 1.20 MS".to_string(),
            "FSR3 REN 1.80 MS".to_string(),
            "OTHER 1.5 MS".to_string(),
        ]
    );
    // **Both halves of the chain, which is what a reader compares.** The two
    // rings are claimed together, so in a running race both rows appear or
    // neither does - and the `OTHER` residual has to subtract both, or the
    // presentation half reads as unaccounted-for cost.
    assert_eq!(
        GpuCost {
            scene: Some(0.004_2),
            blur: None,
            bloom: None,
            upscale: Some(0.001_8),
            upscale_presented: Some(0.003_0),
        }
        .rows(11.76),
        vec![
            "SCENE 4.20 MS".to_string(),
            "FSR3 REN 1.80 MS".to_string(),
            "FSR3 OUT 3.00 MS".to_string(),
            "OTHER 2.8 MS".to_string(),
        ]
    );
    // The upscaler can report before the scene pass does: the four rings are
    // independent, and a slot is claimed per ring per frame.
    assert_eq!(
        GpuCost {
            scene: None,
            blur: None,
            bloom: None,
            upscale: Some(0.001_8),
            upscale_presented: None,
        }
        .rows(11.76),
        vec!["FSR3 REN 1.80 MS".to_string(), "OTHER 10.0 MS".to_string()]
    );
}
/// The residual is the frame-time row minus whatever the GPU row itself adds
/// up to - not a fraction of a target, and not clamped at zero.
#[test]
fn the_residual_is_the_frame_time_minus_the_gpu_row() {
    let cost = GpuCost {
        scene: Some(0.002_5),
        blur: Some(0.001_2),
        bloom: Some(0.003_1),
        upscale: Some(0.002_8),
        upscale_presented: None,
    };
    // 2.5 + 1.2 + 3.1 + 2.8 = 9.6 ms accounted for out of an 11.76 ms frame.
    assert!((cost.residual_ms(11.76).unwrap() - 2.16).abs() < 1e-4);

    // A field that never reported does not count as zero cost accidentally -
    // it is simply left out of the sum, the same way `rows` leaves it out of
    // the panel.
    let partial = GpuCost {
        scene: Some(0.002_5),
        blur: None,
        bloom: None,
        upscale: None,
        upscale_presented: None,
    };
    assert!((partial.residual_ms(11.76).unwrap() - 9.26).abs() < 1e-4);

    // Nothing measured yet: no row, no residual either - there is nothing to
    // subtract from.
    assert_eq!(GpuCost::default().residual_ms(11.76), None);

    // A moment where the rolling mean has fallen faster than the GPU readings
    // (which lag a frame or more) can go negative, and that is reported
    // rather than clamped - zero would claim nothing is missing.
    let heavy = GpuCost {
        scene: Some(0.010_0),
        blur: None,
        bloom: None,
        upscale: None,
        upscale_presented: None,
    };
    assert!(heavy.residual_ms(8.0).unwrap() < 0.0);
}

/// `CPU`, `PRESENT` and `SLEEP` add up to the frame time exactly - that is
/// what makes them a split rather than three more numbers beside it.
///
/// The example is the one a 120 Hz limiter produces on a machine with
/// headroom: an 8.33 ms wall clock, 5 ms of it inside the frame, 1 ms of
/// *that* waiting on the swapchain, and the rest asleep in
/// `App::about_to_wait`. Before this split all 8.33 ms of it read as one
/// `OTHER` row, which is exactly the reading that cannot be acted on.
#[test]
fn the_wall_clock_rows_partition_the_frame_time() {
    let cost = CpuCost {
        frame: Some(0.005),
        present: Some(0.001),
    };
    assert_eq!(
        cost.rows(8.33),
        vec![
            "CPU 4.00 MS".to_string(),
            "PRESENT 1.00 MS".to_string(),
            "SLEEP 3.3 MS".to_string(),
        ]
    );
    // The property the rows claim, asserted rather than left to the reader of
    // three rounded strings: 4.00 + 1.00 + 3.33 is the 8.33 ms frame.
    let sleep = cost.sleep_ms(8.33).expect("a frame was measured");
    assert!((4.0 + 1.0 + sleep - 8.33).abs() < 1e-4, "{sleep}");
}

/// Nothing is claimed until a frame has been measured, the same discipline
/// every GPU row above already follows.
///
/// A `SLEEP 8.3 MS` row derived from no reading would say the loop spent the
/// whole frame idle, which is the most misleading thing this panel could
/// print on a machine that has simply not finished its first frame yet.
#[test]
fn the_wall_clock_rows_say_nothing_until_a_frame_has_been_measured() {
    assert_eq!(CpuCost::default().rows(8.33), Vec::<String>::new());
    assert_eq!(CpuCost::default().sleep_ms(8.33), None);
}

/// A present reading that never came back is left out rather than counted as
/// zero, and the `CPU` row is then the whole frame body.
///
/// The two meters are fed on different paths - a frame that never got a
/// surface texture returns before `present_cost` is recorded - so "one has a
/// reading and the other does not" is a state the panel has to survive.
#[test]
fn a_present_that_never_reported_is_left_out_rather_than_zeroed() {
    let cost = CpuCost {
        frame: Some(0.005),
        present: None,
    };
    assert_eq!(
        cost.rows(8.33),
        vec!["CPU 5.00 MS".to_string(), "SLEEP 3.3 MS".to_string()]
    );
}

/// Sleep can read negative, and is passed through rather than clamped, for
/// the reason [`super::GpuCost::residual_ms`] can: the frame-time mean and
/// the CPU mean are one frame out of phase, so a frame time that has just
/// fallen can be shorter than the body measured before it.
#[test]
fn a_negative_sleep_is_reported_rather_than_clamped() {
    let cost = CpuCost {
        frame: Some(0.010),
        present: None,
    };
    assert!(cost.sleep_ms(8.33).expect("measured") < 0.0);
}
