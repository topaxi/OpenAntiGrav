//! What [`super::GpuCost`] and [`super::CpuCost`] are asserted to say: which
//! rows appear at all, what the residual subtracts, and that the wall-clock
//! rows partition the frame time rather than merely sitting near it.
//!
//! Its own file rather than an inline `#[cfg(test)]` block for the reason
//! `perf/tests.rs` gives: past 200 lines a test module moves out. See
//! `scripts/check-file-size.py`.

use super::*;
use crate::perf::PassCosts;

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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
    };
    assert!(heavy.residual_ms(8.0).unwrap() < 0.0);
}

/// `CPU`, `PRESENT` and `OUTSIDE` add up to the frame time exactly - that is
/// what makes them a split rather than three more numbers beside it.
///
/// The example is the one a 120 Hz limiter produces on a machine with
/// headroom: an 8.33 ms wall clock, 5 ms of it inside the frame, 1 ms of
/// *that* waiting on the swapchain, and the rest outside the frame entirely -
/// asleep in `App::about_to_wait`, on a run with the headroom to be. Before
/// this split all 8.33 ms of it read as one `OTHER` row, which is exactly the
/// reading that cannot be acted on.
#[test]
fn the_wall_clock_rows_partition_the_frame_time() {
    let cost = CpuCost {
        frame: Some(0.005),
        present: Some(0.001),
    };
    assert_eq!(
        cost.rows(8.33),
        vec![
            "FRAME 8.3 MS".to_string(),
            "  CPU 4.00 MS".to_string(),
            "  PRESENT 1.00 MS".to_string(),
            "  OUTSIDE 3.3 MS".to_string(),
        ],
        "the parent, then its three parts, each indented under it"
    );
    // The property the rows claim, asserted rather than left to the reader of
    // three rounded strings: 4.00 + 1.00 + 3.33 is the 8.33 ms frame.
    let outside = cost.outside_ms(8.33).expect("a frame was measured");
    assert!((4.0 + 1.0 + outside - 8.33).abs() < 1e-4, "{outside}");
}

/// Nothing is claimed until a frame has been measured, the same discipline
/// every GPU row above already follows.
///
/// An `OUTSIDE 8.3 MS` row derived from no reading would say the loop spent
/// the whole frame somewhere it cannot see, which is the most misleading
/// thing this panel could print on a machine that has simply not finished its
/// first frame yet.
#[test]
fn the_wall_clock_rows_say_nothing_until_a_frame_has_been_measured() {
    assert_eq!(CpuCost::default().rows(8.33), Vec::<String>::new());
    assert_eq!(CpuCost::default().outside_ms(8.33), None);
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
        vec![
            "FRAME 8.3 MS".to_string(),
            "  CPU 5.00 MS".to_string(),
            "  OUTSIDE 3.3 MS".to_string(),
        ]
    );
}

/// The gap can read negative, and is passed through rather than clamped, for
/// the reason [`super::GpuCost::residual_ms`] can: the frame-time mean and
/// the CPU mean are one frame out of phase, so a frame time that has just
/// fallen can be shorter than the body measured before it.
#[test]
fn a_negative_gap_is_reported_rather_than_clamped() {
    let cost = CpuCost {
        frame: Some(0.010),
        present: None,
    };
    assert!(cost.outside_ms(8.33).expect("measured") < 0.0);
}

fn passes(rows: &[(&'static str, f32)]) -> PassRows {
    let mut passes = PassRows::default();
    for &(label, seconds) in rows {
        passes.add(label, seconds);
    }
    passes
}

/// The per-pass rows sit after the chains' rows and before `OTHER`, in the table's order whatever
/// order they were added in, one row to a group of labels, and a row under a tenth of a
/// millisecond is left off. A label no row claims is drawn under its own text, last.
#[test]
fn the_pass_rows_are_ordered_grouped_and_hidden_under_a_tenth_of_a_millisecond() {
    let cost = GpuCost {
        scene: Some(0.004),
        passes: passes(&[
            ("hud and ui", 0.000_6),
            ("bloom composite", 0.000_2),
            ("bloom bright", 0.000_3),
            ("ps2 bloom composite", 0.000_4),
            ("shadow map", 0.000_05),
            ("glass", 0.000_55),
            ("mystery pass", 0.000_2),
        ]),
        ..Default::default()
    };
    assert_eq!(
        cost.rows(10.0),
        vec![
            "SCENE 4.00 MS".to_string(),
            "GLASS 0.55 MS".to_string(),
            "PS2 BLOOM 0.40 MS".to_string(),
            "PSP BLOOM 0.50 MS".to_string(),
            "HUD AND UI 0.60 MS".to_string(),
            "mystery pass 0.20 MS".to_string(),
            "OTHER 3.7 MS".to_string(),
        ],
        "shadow map is 0.05 ms and hidden here; the PSP bloom's two labels are one row"
    );
}

/// `OTHER` is the frame less **every** timed pass, the hidden rows included: hiding a row on the
/// overlay must not hand its time back to `OTHER`.
#[test]
fn the_residual_subtracts_every_pass_hidden_or_shown() {
    let timed = GpuCost {
        scene: Some(0.004),
        passes: PassRows::default(),
        ..Default::default()
    };
    let with_passes = GpuCost {
        passes: passes(&[
            ("glass", 0.000_5),
            ("shadow map", 0.000_05),
            ("smaa", 0.000_45),
        ]),
        ..timed
    };
    let before = timed.residual_ms(10.0).expect("measured");
    let after = with_passes.residual_ms(10.0).expect("measured");
    assert!((before - 6.0).abs() < 1e-4, "{before}");
    assert!(
        (after - 5.0).abs() < 1e-4,
        "frame 10 - scene 4 - passes 1.0 = 5.0, got {after}"
    );
    assert_eq!(
        GpuCost {
            passes: passes(&[("hud and ui", 0.001)]),
            ..Default::default()
        }
        .residual_ms(10.0),
        None,
        "a pass reading alone (a menu frame) does not start an OTHER row: it stays the \
         scene-and-chains residual the controller's own reading is defined on"
    );
}

/// A frame the pass did not run in records zero for it, so a pass that runs every other frame
/// reads as half its cost **per frame**, and a load drops what came before it.
#[test]
fn a_pass_that_did_not_run_costs_the_frame_nothing() {
    let mut costs = PassCosts::new();
    costs.record_frame(1, &[("glass", 0.002), ("hud and ui", 0.001)]);
    costs.record_frame(2, &[("hud and ui", 0.001)]);
    let rows = costs.rows().ordered();
    assert_eq!(rows[0].0, "GLASS");
    assert!((rows[0].1 - 0.001).abs() < 1e-6, "{rows:?}");
    assert!((rows[1].1 - 0.001).abs() < 1e-6, "{rows:?}");
    costs.stalled(2);
    costs.record_frame(2, &[("glass", 9.0)]);
    assert!(
        costs.rows().is_empty(),
        "a reading from the stalled frame is refused"
    );
    costs.record_frame(3, &[("glass", 0.002)]);
    assert!((costs.rows().total() - 0.002).abs() < 1e-6);
}

/// Every label a pass site stamps, or hands the shared pass helper, belongs to a named row: one
/// that did not would be drawn under its lowercase label, and `MAX_ROWS` would be spent on it.
///
/// Reads the sources: a label is a string literal at the site, and this is the one place that
/// can see all of them. The helper-fed labels (`pass::draw` and friends) are listed by hand
/// because their literal sits a call away from the stamp.
#[test]
fn every_stamped_label_has_a_row() {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut labels: Vec<String> = [
        "bloom bright",
        "bloom blur x",
        "bloom composite",
        "ps2 bloom composite",
        "wo composite gate",
        "wo composite blur",
        "wo composite clear",
        "omega tonemap luma",
        "omega tonemap halve",
        "omega tonemap apply",
        "omega tonemap clear",
        "omega bloom bright",
    ]
    .map(String::from)
    .to_vec();
    let mut stack = vec![crates.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("a crate directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name != "target") {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path
                    .strip_prefix(&crates)
                    .expect("under crates")
                    .to_string_lossy()
                    .contains("tests")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                for part in text.split("timing::stamp(\"").skip(1) {
                    labels.push(part.split('"').next().expect("a closing quote").to_string());
                }
            }
        }
    }
    assert!(
        labels.len() > 30,
        "found only {labels:?}: the scan stopped matching"
    );
    for label in labels {
        let mut rows = PassRows::default();
        rows.add(Box::leak(label.clone().into_boxed_str()), 0.001);
        let name = rows.ordered()[0].0.to_string();
        assert_ne!(name, label, "`{label}` has no row in perf/passes.rs's ROWS");
    }
}

/// A pass that runs on some frames only: the overlay row is its mean over the frames it ran in,
/// the per-frame mean (the frame log's and `OTHER`'s) stays diluted by the frames it did not.
#[test]
fn an_intermittent_pass_shows_its_mean_when_it_ran() {
    let mut costs = PassCosts::new();
    let mut ran = 0;
    for frame in 1..=120_u64 {
        if frame % 5 < 3 {
            ran += 1;
            costs.record_frame(frame, &[("hd zoom", 0.002), ("hud and ui", 0.001)]);
        } else {
            costs.record_frame(frame, &[("hud and ui", 0.001)]);
        }
    }
    assert_eq!(ran, 72);
    let cost = GpuCost {
        scene: Some(0.004),
        passes: costs.rows(),
        ..Default::default()
    };
    let overlay = cost.rows(10.0);
    assert!(
        overlay.contains(&"HD ZOOM 2.00 MS".to_string()),
        "72 of 120 frames at 2.0 ms shows 2.0 ms: {overlay:?}"
    );
    assert!(
        overlay.contains(&"HUD AND UI 1.00 MS".to_string()),
        "{overlay:?}"
    );
    let per_frame = costs.rows().ordered();
    assert_eq!(per_frame[0].0, "HD ZOOM");
    assert!(
        (per_frame[0].1 - 0.001_2).abs() < 1e-6,
        "the frame log's mean still counts the frames without it: {per_frame:?}"
    );
    // OTHER takes off the per-frame mean (1.2 + 1.0), not the 2.0 the row shows.
    assert!((cost.residual_ms(10.0).unwrap() - (10.0 - 4.0 - 1.2 - 1.0)).abs() < 1e-3);
}

/// A pass that ran only a few frames ago is drawn for as long as it is in the window, at its
/// own cost; once the window holds none of its frames it is hidden, and stays in the frame log.
#[test]
fn a_pass_that_has_not_run_in_the_window_is_hidden() {
    let mut costs = PassCosts::new();
    costs.record_frame(1, &[("glass", 0.003), ("hud and ui", 0.001)]);
    for frame in 2..=120_u64 {
        costs.record_frame(frame, &[("hud and ui", 0.001)]);
    }
    let shown = |costs: &PassCosts| {
        GpuCost {
            scene: Some(0.004),
            passes: costs.rows(),
            ..Default::default()
        }
        .rows(10.0)
    };
    assert!(
        shown(&costs).contains(&"GLASS 3.00 MS".to_string()),
        "its one frame is still in the 120: {:?}",
        shown(&costs)
    );
    costs.record_frame(121, &[("hud and ui", 0.001)]);
    let overlay = shown(&costs);
    assert!(
        !overlay.iter().any(|row| row.starts_with("GLASS")),
        "no frame of the window ran it: {overlay:?}"
    );
    assert!(
        overlay.contains(&"HUD AND UI 1.00 MS".to_string()),
        "{overlay:?}"
    );
    assert_eq!(
        costs.rows().ordered()[0].0,
        "GLASS",
        "the frame log keeps the row"
    );
}

/// Two labels of one row are one row: a frame either ran it or did not, and the lanes add.
#[test]
fn the_labels_of_one_row_share_their_frames_when_run() {
    let mut costs = PassCosts::new();
    costs.record_frame(1, &[("bloom bright", 0.001), ("bloom blur x", 0.002)]);
    costs.record_frame(2, &[("bloom bright", 0.001)]);
    costs.record_frame(3, &[]);
    costs.record_frame(4, &[("bloom bright", 0.001), ("bloom blur x", 0.002)]);
    let rows = costs.rows();
    let when_run = rows.ordered_when_run();
    assert_eq!(when_run.len(), 1, "{when_run:?}");
    assert_eq!(when_run[0].0, "PSP BLOOM");
    assert!(
        (when_run[0].1 - 0.007 / 3.0).abs() < 1e-6,
        "3 frames ran it, 3 + 1 + 3 ms between them: {when_run:?}"
    );
    assert!((rows.total() - 0.007 / 4.0).abs() < 1e-6);
}

/// The `GLASS` row names the step the target is drawn at (REFLECTION RESOLUTION, or the step
/// `auto` is at), and no other row does.
#[test]
fn the_glass_row_names_the_reflection_step() {
    let cost = GpuCost {
        scene: Some(0.004),
        passes: passes(&[("glass", 0.002), ("smaa", 0.000_5)]),
        reflection: Some("quarter"),
        ..Default::default()
    };
    let rows = cost.rows(10.0);
    assert!(
        rows.contains(&"GLASS QUARTER 2.00 MS".to_string()),
        "{rows:?}"
    );
    assert!(rows.contains(&"SMAA 0.50 MS".to_string()), "{rows:?}");
    let none = GpuCost {
        reflection: None,
        ..cost
    };
    assert!(none.rows(10.0).contains(&"GLASS 2.00 MS".to_string()));
}
