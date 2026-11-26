use super::*;
use std::collections::BTreeMap;

fn cost(scalable: f32, fixed: f32) -> drs::Cost {
    drs::Cost {
        scalable,
        fixed,
        blur: 0.0,
    }
}

const ALL: Expects = Expects {
    blur: true,
    hd_bloom: true,
    upscale: true,
};

#[test]
fn a_frame_with_every_part_adds_them_up() {
    let c = assemble(0.004, Some(0.001), Some(0.002), Some((0.003, 0.0005)), ALL)
        .expect("every part came back");
    assert!(
        (c.scalable - 0.010).abs() < 1e-6,
        "scene + blur + bloom + scaled upscale"
    );
    assert!(
        (c.fixed - 0.0005).abs() < 1e-6,
        "the presented half is the fixed one"
    );
    assert!((c.blur - 0.001).abs() < 1e-6);
}

#[test]
fn a_chain_that_did_not_run_is_free_and_one_not_measured_is_a_skipped_frame() {
    // No blur, no HD chain in this scene, no temporal upscaler asked for: all zero.
    let c = assemble(0.004, None, None, None, Expects::default()).unwrap();
    assert_eq!((c.scalable, c.fixed, c.blur), (0.004, 0.0, 0.0));
    // The scene has an HD bloom chain and its reading has not come back: not a cheap frame.
    let bloom = Expects {
        hd_bloom: true,
        ..Expects::default()
    };
    assert!(assemble(0.004, None, None, None, bloom).is_none());
    // A temporal upscaler is resolving and its pair has not come back.
    let upscale = Expects {
        upscale: true,
        ..Expects::default()
    };
    assert!(assemble(0.004, None, None, None, upscale).is_none());
}

#[test]
fn a_blur_reading_is_its_cost_and_a_missing_one_is_free_only_when_blur_did_not_run() {
    let ran = Expects {
        blur: true,
        ..Expects::default()
    };
    let off = Expects::default();
    // (Some, ran): the reading, in both sums.
    let c = assemble(0.004, Some(0.002), None, None, ran).unwrap();
    assert!((c.scalable - 0.006).abs() < 1e-6 && (c.blur - 0.002).abs() < 1e-6);
    // (Some, not ran): a reading is a reading, whatever the record says.
    let c = assemble(0.004, Some(0.002), None, None, off).unwrap();
    assert!((c.scalable - 0.006).abs() < 1e-6 && (c.blur - 0.002).abs() < 1e-6);
    // (None, ran): not measured, so a skipped frame and not a frame the blur made cheap.
    assert!(assemble(0.004, None, None, None, ran).is_none());
    // (None, not ran): blur is off, which is free and never a skipped frame.
    let c = assemble(0.004, None, None, None, off).unwrap();
    assert_eq!((c.scalable, c.blur), (0.004, 0.0));
}

#[test]
fn drawn_remembers_each_frame_and_reads_a_forgotten_one_as_drawn() {
    let mut drawn = Drawn::default();
    drawn.mark(10, true);
    drawn.mark(11, false);
    assert!(drawn.drawn(10));
    assert!(!drawn.drawn(11));
    // Never marked: pessimistic, a skipped frame rather than a free blur.
    assert!(drawn.drawn(12));
    // Overwritten by a frame a whole record later: frame 11 is forgotten, not "not drawn".
    drawn.mark(11 + DRAWN_FRAMES as u64, true);
    assert!(drawn.drawn(11));
    // Blur off for a whole run of frames: every frame a reading can still name reads as not
    // drawn, so blur off never skips a frame.
    let last = 100 + 3 * DRAWN_FRAMES as u64;
    for frame in 100..last {
        drawn.mark(frame, false);
    }
    assert!((last - DRAWN_FRAMES as u64..last).all(|frame| !drawn.drawn(frame)));
}

fn rings() -> Rings {
    Rings {
        pass: None,
        blur: None,
        hd_bloom: None,
        upscale: None,
        upscale_presented: None,
        seen: BTreeMap::new(),
        blur_drawn: Drawn::default(),
    }
}

fn scene_only(scene: f32) -> Seen {
    Seen {
        scene: Some(scene),
        ..Seen::default()
    }
}

#[test]
fn a_further_window_reads_its_blur_by_the_same_rule() {
    let mut window = rings();
    let off = oag_present::perf::SceneStats::default();
    let on = oag_present::perf::SceneStats {
        blur_encoded: true,
        ..oag_present::perf::SceneStats::default()
    };
    // Frame 1 drew no blur and frame 2 did; neither blur reading has come back.
    window.give_back(1, &off, true);
    window.give_back(2, &on, true);
    window.seen.insert(1, scene_only(0.004));
    window.seen.insert(2, scene_only(0.004));
    let c = window.cost(1, false, false).expect("blur off is free");
    assert_eq!((c.scalable, c.blur), (0.004, 0.0));
    assert!(
        window.cost(2, false, false).is_none(),
        "blur ran and is not measured yet"
    );
    // Its reading arrives: frame 2 is assembled with it.
    window.seen.get_mut(&2).unwrap().blur = Some(0.003);
    let c = window.cost(2, false, false).unwrap();
    assert!((c.scalable - 0.007).abs() < 1e-6 && (c.blur - 0.003).abs() < 1e-6);
}

#[test]
fn a_frame_is_fed_once_every_window_has_come_back_for_it() {
    let mut ahead = BTreeMap::from([(10, cost(0.004, 0.0))]);
    let fed = complete_frames(&mut ahead, |_| Some(cost(0.003, 0.001)));
    assert_eq!(fed.len(), 1);
    assert_eq!(fed[0].0, 10);
    assert!(
        (fed[0].1.scalable - 0.007).abs() < 1e-6,
        "window 1's plus the other's"
    );
    assert!((fed[0].1.fixed - 0.001).abs() < 1e-6);
    assert!(ahead.is_empty(), "a frame fed is forgotten");
}

#[test]
fn a_window_a_frame_late_holds_the_frame_instead_of_dropping_it() {
    let mut ahead = BTreeMap::from([(10, cost(0.004, 0.0)), (11, cost(0.004, 0.0))]);
    // Window 2 has come back for neither yet: both wait, in order.
    assert!(complete_frames(&mut ahead, |_| None).is_empty());
    assert_eq!(ahead.len(), 2, "within the lag, nothing is given up on");
    // It arrives for frame 10 only: that one is fed and 11 still waits.
    let fed = complete_frames(&mut ahead, |frame| (frame == 10).then(|| cost(0.001, 0.0)));
    assert_eq!(
        fed.iter().map(|(frame, _)| *frame).collect::<Vec<_>>(),
        [10]
    );
    assert_eq!(ahead.keys().copied().collect::<Vec<_>>(), [11]);
}

#[test]
fn a_younger_frame_is_not_fed_ahead_of_an_older_one_still_waiting() {
    let mut ahead = BTreeMap::from([(10, cost(0.004, 0.0)), (11, cost(0.004, 0.0))]);
    let fed = complete_frames(&mut ahead, |frame| (frame == 11).then(|| cost(0.001, 0.0)));
    assert!(
        fed.is_empty(),
        "the controller counts readings as frames, in order"
    );
    assert_eq!(ahead.len(), 2);
}

#[test]
fn a_frame_a_window_never_comes_back_for_is_given_up_on() {
    let mut ahead = BTreeMap::from([(10, cost(0.004, 0.0))]);
    // Frames go by; window 2 never reads back for 10 (it closed, or its ring was full).
    for frame in 11..=10 + MAX_LAG + 1 {
        ahead.insert(frame, cost(0.004, 0.0));
    }
    let fed = complete_frames(&mut ahead, |frame| (frame > 10).then(|| cost(0.0, 0.0)));
    assert_eq!(
        fed.first().map(|(frame, _)| *frame),
        Some(11),
        "10 was dropped, 11 on was fed"
    );
    assert!(!ahead.contains_key(&10));
}

fn reading(frame: u64, label: &'static str, seconds: f32) -> oag_gpu::timing::PassReading {
    oag_gpu::timing::PassReading {
        frame,
        label,
        seconds,
    }
}

/// A drained set is recorded a whole frame at a time, so a label absent from a frame costs it
/// nothing, and frames up to a stall are dropped.
#[test]
fn drained_pass_readings_are_recorded_a_frame_at_a_time_and_a_stall_drops_its_frames() {
    let mut costs = perf::PassCosts::new();
    record_passes(
        &mut costs,
        &[
            reading(2, "glass", 0.500),
            reading(4, "glass", 0.002),
            reading(4, "hud and ui", 0.001),
            reading(5, "glass", 0.004),
        ],
        3,
    );
    let rows = costs.rows().ordered();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0].0, "GLASS");
    assert!(
        (rows[0].1 - 0.003).abs() < 1e-6,
        "frame 2 is at or before the stall and frames 4 and 5 average: {rows:?}"
    );
    assert_eq!(rows[1].0, "HUD AND UI");
    assert!(
        (rows[1].1 - 0.000_5).abs() < 1e-6,
        "frame 5 had no HUD reading and so cost the frame nothing: {rows:?}"
    );
}

/// What the resolution controller is fed does not depend on the labelled passes, and the overlay's
/// `OTHER` is exactly the controller's own `frame - cost` reading less them.
///
/// The controller learns its reserve from `frame - cost.total()` where `cost` is
/// [`assemble`]'s; the overlay prints `GpuCost::residual_ms`. The two definitions agreeing but
/// for the passes is what keeps "OTHER is what the controller's reserve is made of" true: a pass
/// moved into [`assemble`] (the controller's cost) or out of the overlay's subtraction breaks it.
#[test]
fn the_overlays_other_is_the_controllers_reading_less_the_labelled_passes() {
    let (scene, blur, bloom) = (0.004_f32, 0.001_f32, 0.002_f32);
    let upscale = (0.003_f32, 0.000_5_f32);
    let cost = assemble(scene, Some(blur), Some(bloom), Some(upscale), ALL).expect("measured");
    let frame_ms = 16.0_f32;
    let controllers_reading_ms = frame_ms - cost.total() * 1000.0;

    let mut passes = perf::PassRows::default();
    for (label, seconds) in [
        ("glass", 0.000_6),
        ("shadow map", 0.001_4),
        ("hud and ui", 0.000_8),
    ] {
        passes.add(label, seconds);
    }
    let overlay = perf::GpuCost {
        scene: Some(scene),
        blur: Some(blur),
        bloom: Some(bloom),
        upscale: Some(upscale.0),
        upscale_presented: Some(upscale.1),
        passes,
        reflection: None,
    };
    let other_ms = overlay.residual_ms(frame_ms).expect("measured");
    assert!(
        (other_ms + passes.total() * 1000.0 - controllers_reading_ms).abs() < 1e-3,
        "OTHER {other_ms} + passes {} != the controller's reading {controllers_reading_ms}",
        passes.total() * 1000.0
    );
    // And with the passes gone the overlay is the controller's reading, which is what it was.
    let without = perf::GpuCost {
        passes: perf::PassRows::default(),
        ..overlay
    };
    assert!(
        (without.residual_ms(frame_ms).expect("measured") - controllers_reading_ms).abs() < 1e-3
    );
    // The cost the controller is fed is the same function of the five readings with or without.
    assert_eq!(
        cost,
        assemble(scene, Some(blur), Some(bloom), Some(upscale), ALL).expect("measured")
    );
}

/// The reflection rung is fed the newest frame drawn at its step that timed a glass pass, with
/// every labelled pass of that same frame; a frame drawn at another step (one in flight across a
/// step, a photo's full target) and a frame from before a load are not its readings.
#[test]
fn the_reflection_reading_is_the_newest_frame_drawn_at_the_rungs_step() {
    use oag_display::display::ReflectionResolution;
    let readings = [
        reading(40, "glass", 0.009),
        reading(40, "hud and ui", 0.001),
        reading(41, "glass", 0.013),
        reading(41, "shadow map", 0.002),
        reading(42, "glass", 0.020),
        reading(42, "hud and ui", 0.001),
        reading(43, "hud and ui", 0.001),
    ];
    let mut drawn: Drawn<Option<ReflectionResolution>> = Drawn::default();
    drawn.mark(40, Some(ReflectionResolution::Half));
    drawn.mark(41, Some(ReflectionResolution::Half));
    drawn.mark(42, Some(ReflectionResolution::Original));
    drawn.mark(43, Some(ReflectionResolution::Half));
    let at = |step| move |frame| drawn.get(frame) == Some(Some(step));
    let (glass, passes) =
        reflection_reading(&readings, 0, at(ReflectionResolution::Half)).expect("frame 41");
    assert!((glass - 0.013).abs() < 1e-7 && (passes - 0.015).abs() < 1e-7);
    let (glass, _) =
        reflection_reading(&readings, 0, at(ReflectionResolution::Original)).expect("frame 42");
    assert!((glass - 0.020).abs() < 1e-7);
    assert_eq!(
        reflection_reading(&readings, 41, at(ReflectionResolution::Half)),
        None,
        "frames up to a load are not readings"
    );
    assert_eq!(
        reflection_reading(&readings, 0, at(ReflectionResolution::Quarter)),
        None
    );
}

/// An intermittent pass absent from this frame still leaves `OTHER` the controller's reading less
/// the passes' **per-frame** mean: the overlay row shows what the pass costs when it runs, and
/// `OTHER` takes off what it costs a frame on average, so `frame - timed` is not short by the
/// frames the pass sat out.
#[test]
fn an_intermittent_pass_absent_this_frame_leaves_other_on_per_frame_values() {
    let (scene, blur, bloom) = (0.004_f32, 0.001_f32, 0.002_f32);
    let upscale = (0.003_f32, 0.000_5_f32);
    let cost = assemble(scene, Some(blur), Some(bloom), Some(upscale), ALL).expect("measured");
    let frame_ms = 16.0_f32;
    let controllers_reading_ms = frame_ms - cost.total() * 1000.0;

    let mut costs = perf::PassCosts::new();
    let readings: Vec<_> = (1..=120_u64)
        .flat_map(|frame| {
            let ran = frame % 4 == 1 && frame < 120;
            let mut frame_readings = vec![reading(frame, "hud and ui", 0.001)];
            if ran {
                frame_readings.push(reading(frame, "hd zoom", 0.004));
            }
            frame_readings
        })
        .collect();
    record_passes(&mut costs, &readings, 0);
    let passes = costs.rows();
    let overlay = perf::GpuCost {
        scene: Some(scene),
        blur: Some(blur),
        bloom: Some(bloom),
        upscale: Some(upscale.0),
        upscale_presented: Some(upscale.1),
        passes,
        reflection: None,
    };
    let shown = overlay.rows(frame_ms);
    assert!(
        shown.contains(&"HD ZOOM 4.00 MS".to_string()),
        "30 frames of 120 at 4.0 ms shows 4.0: {shown:?}"
    );
    let per_frame_ms = passes.total() * 1000.0;
    assert!(
        (per_frame_ms - (1.0 + 4.0 * 30.0 / 120.0)).abs() < 1e-3,
        "{per_frame_ms}"
    );
    let other_ms = overlay.residual_ms(frame_ms).expect("measured");
    assert!(
        (other_ms + per_frame_ms - controllers_reading_ms).abs() < 1e-3,
        "OTHER {other_ms} + per-frame passes {per_frame_ms} != the controller's reading \
         {controllers_reading_ms}"
    );
}

/// TARGET FPS `original` throttles the width on the title that has a law, aimed at the rate the
/// loop is held to, and is no controller on a title without one.
#[test]
fn the_original_target_is_a_width_throttle_only_where_the_title_has_one() {
    let ceiling = (3840, 2160);
    let hd = original_limits(Some(oag_hd::TITLE), ceiling, Some(120));
    let spec = hd.width().expect("HD has a width throttle");
    assert_eq!(spec.threshold_hz(), 120);
    assert_eq!(hd.floor(), (2600, 2160));
    // The title's own two durations, at the rate they are authored against.
    let at_60 = original_limits(Some(oag_hd::TITLE), ceiling, Some(60));
    let mut drs = drs::Controller::new();
    for (gpu, level) in [(0.016_112_5, 1), (0.015_556_5, 1), (0.015_555_5, 0)] {
        drs.record(cost(gpu, 0.0), None, at_60);
        drs.finish_frame(None, at_60);
        assert_eq!(drs.width_level(), level, "{gpu} s");
    }
    for title in [
        Some(oag_pulse::TITLE),
        Some(oag_pure::TITLE),
        Some(oag_omega::TITLE),
        Some(oag_2048::TITLE),
        None,
    ] {
        let limits = original_limits(title, ceiling, Some(60));
        assert!(limits.width().is_none() && !limits.target().is_on());
        assert_eq!(drs::Controller::new().extent(limits), ceiling);
    }
}

/// The width throttle times the whole frame, so the labelled passes join its reading; the
/// budget loop's reading is the one it always was.
#[test]
fn only_the_width_throttle_is_handed_the_labelled_passes() {
    let base = cost(0.010, 0.001);
    let hd = original_limits(Some(oag_hd::TITLE), (1920, 1080), Some(60));
    let timed = whole_frame_cost(base, 0.004, hd);
    assert!((timed.total() - 0.015).abs() < 1e-6, "{timed:?}");
    let budget = drs::Limits::new((1920, 1080), (960, 540), drs::Target::OFFERED[2]);
    assert_eq!(whole_frame_cost(base, 0.004, budget), base);
}

#[test]
fn the_pass_sum_is_the_newest_frames_alone() {
    let reading = |frame, seconds| oag_gpu::timing::PassReading {
        frame,
        label: "glass",
        seconds,
    };
    let readings = [reading(4, 0.001), reading(5, 0.002), reading(5, 0.003)];
    let sum = newest_pass_sum(&readings, 3).expect("frame 5 came back");
    assert!((sum - 0.005).abs() < 1e-6, "{sum}");
    assert_eq!(
        newest_pass_sum(&readings, 5),
        None,
        "all from before the stall"
    );
    // The zoom ring runs inside the bloom chain's own bracket, so it is not counted again.
    let zoom = oag_gpu::timing::PassReading {
        frame: 5,
        label: "hd zoom",
        seconds: 0.5,
    };
    let sum = newest_pass_sum(&[readings[2], zoom], 3).expect("frame 5 came back");
    assert!((sum - 0.003).abs() < 1e-6, "{sum}");
}
