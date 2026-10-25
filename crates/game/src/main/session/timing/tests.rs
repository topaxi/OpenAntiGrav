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
    assert!((c.scalable - 0.010).abs() < 1e-6, "scene + blur + bloom + scaled upscale");
    assert!((c.fixed - 0.0005).abs() < 1e-6, "the presented half is the fixed one");
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
    drawn.mark(11 + Drawn::FRAMES as u64, true);
    assert!(drawn.drawn(11));
    // Blur off for a whole run of frames: every frame a reading can still name reads as not
    // drawn, so blur off never skips a frame.
    let last = 100 + 3 * Drawn::FRAMES as u64;
    for frame in 100..last {
        drawn.mark(frame, false);
    }
    assert!((last - Drawn::FRAMES as u64..last).all(|frame| !drawn.drawn(frame)));
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
    assert!(window.cost(2, false, false).is_none(), "blur ran and is not measured yet");
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
    assert!((fed[0].1.scalable - 0.007).abs() < 1e-6, "window 1's plus the other's");
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
    assert_eq!(fed.iter().map(|(frame, _)| *frame).collect::<Vec<_>>(), [10]);
    assert_eq!(ahead.keys().copied().collect::<Vec<_>>(), [11]);
}

#[test]
fn a_younger_frame_is_not_fed_ahead_of_an_older_one_still_waiting() {
    let mut ahead = BTreeMap::from([(10, cost(0.004, 0.0)), (11, cost(0.004, 0.0))]);
    let fed = complete_frames(&mut ahead, |frame| (frame == 11).then(|| cost(0.001, 0.0)));
    assert!(fed.is_empty(), "the controller counts readings as frames, in order");
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
    assert_eq!(fed.first().map(|(frame, _)| *frame), Some(11), "10 was dropped, 11 on was fed");
    assert!(!ahead.contains_key(&10));
}
