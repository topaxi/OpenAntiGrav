use super::*;
use std::collections::BTreeMap;

fn cost(scalable: f32, fixed: f32) -> drs::Cost {
    drs::Cost {
        scalable,
        fixed,
        blur: 0.0,
    }
}

#[test]
fn a_frame_with_every_part_adds_them_up() {
    let c = assemble(0.004, Some(0.001), Some(0.002), Some((0.003, 0.0005)), true, true)
        .expect("every part came back");
    assert!((c.scalable - 0.010).abs() < 1e-6, "scene + blur + bloom + scaled upscale");
    assert!((c.fixed - 0.0005).abs() < 1e-6, "the presented half is the fixed one");
}

#[test]
fn a_chain_that_did_not_run_is_free_and_one_not_measured_is_a_skipped_frame() {
    // No blur, no HD chain in this scene, no temporal upscaler asked for: all zero.
    let c = assemble(0.004, None, None, None, false, false).unwrap();
    assert_eq!((c.scalable, c.fixed), (0.004, 0.0));
    // The scene has an HD bloom chain and its reading has not come back: not a cheap frame.
    assert!(assemble(0.004, None, None, None, true, false).is_none());
    // A temporal upscaler is resolving and its pair has not come back.
    assert!(assemble(0.004, None, None, None, false, true).is_none());
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
