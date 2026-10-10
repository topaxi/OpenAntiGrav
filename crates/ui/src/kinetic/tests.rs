use super::*;

const DT: f32 = 1.0 / 60.0;

/// Drags `per_tick` units a tick for `ticks` ticks, the way a finger moving
/// at a steady speed reports, ticking between.
fn swipe(kinetic: &mut Kinetic, extent: &Extent, per_tick: f32, ticks: usize) {
    for _ in 0..ticks {
        kinetic.drag(per_tick, extent);
        kinetic.tick(DT, extent);
    }
}

/// Ticks until at rest, or `limit` seconds, returning the seconds taken.
fn run(kinetic: &mut Kinetic, extent: &Extent, limit: f32) -> f32 {
    let mut t = 0.0;
    while !kinetic.is_still() && t < limit {
        kinetic.tick(DT, extent);
        t += DT;
    }
    t
}

fn pressed() -> Pointer {
    Pointer {
        pressed: true,
        ..Pointer::default()
    }
}

#[test]
fn the_content_follows_the_finger_one_to_one() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(0.3, &extent);
    kinetic.drag(-0.05, &extent);
    assert!((kinetic.offset() - 0.25).abs() < 1e-6);
    assert!(kinetic.is_held());
}

#[test]
fn a_flick_coasts_and_lands_on_the_item_it_was_heading_for() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    // 0.1 items a tick is 6 items a second.
    swipe(&mut kinetic, &extent, 0.1, 10);
    let lifted_at = kinetic.offset();
    kinetic.release(&extent);
    let took = run(&mut kinetic, &extent, 5.0);
    assert!(kinetic.is_still(), "came to rest within {took}s");
    // Unaimed it would travel 6 / FRICTION = 3 items past the lift.
    assert_eq!(kinetic.offset(), (lifted_at + 6.0 / FRICTION).round());
    assert!(kinetic.offset() > lifted_at + 2.0, "{}", kinetic.offset());
    assert_eq!(kinetic.offset().fract(), 0.0);
}

#[test]
fn a_faster_flick_goes_further_and_the_fastest_is_capped() {
    let extent = Extent::wrapping();
    let distance = |per_tick: f32| {
        let mut kinetic = Kinetic::new(0.0);
        kinetic.press(&extent);
        swipe(&mut kinetic, &extent, per_tick, 6);
        kinetic.release(&extent);
        run(&mut kinetic, &extent, 20.0);
        kinetic.offset()
    };
    let (slow, fast) = (distance(0.05), distance(0.2));
    assert!(fast > slow, "{fast} vs {slow}");
    let capped = distance(5.0);
    let held = 5.0 * 6.0;
    let most = (held + MAX_FLING / FRICTION_RANGE.0).ceil();
    assert!(capped <= most, "{capped} past the cap's reach {most}");
}

#[test]
fn a_finger_that_stops_before_lifting_does_not_fling() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    swipe(&mut kinetic, &extent, 0.1, 10);
    // Held still for longer than the velocity window.
    swipe(&mut kinetic, &extent, 0.0, 8);
    kinetic.release(&extent);
    run(&mut kinetic, &extent, 5.0);
    assert_eq!(kinetic.offset(), 1.0, "settles on the nearest item to 1.0");
}

#[test]
fn a_slow_drag_settles_on_the_nearest_item() {
    let extent = Extent::wrapping();
    for (dragged, nearest) in [(0.4, 0.0), (0.6, 1.0), (-0.6, -1.0), (2.45, 2.0)] {
        let mut kinetic = Kinetic::new(0.0);
        kinetic.press(&extent);
        kinetic.drag(dragged, &extent);
        swipe(&mut kinetic, &extent, 0.0, 10);
        kinetic.release(&extent);
        let took = run(&mut kinetic, &extent, 2.0);
        assert_eq!(kinetic.offset(), nearest, "dragged {dragged}");
        assert!(took < 0.6, "the settle took {took}s");
    }
}

/// Critically damped: a settle approaches its item from one side and never
/// passes it.
#[test]
fn a_settle_never_overshoots() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(0.45, &extent);
    swipe(&mut kinetic, &extent, 0.0, 10);
    kinetic.release(&extent);
    while !kinetic.is_still() {
        kinetic.tick(DT, &extent);
        assert!(kinetic.offset() >= 0.0, "{}", kinetic.offset());
    }
}

#[test]
fn a_long_tick_cannot_make_the_spring_ring() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(0.45, &extent);
    swipe(&mut kinetic, &extent, 0.0, 10);
    kinetic.release(&extent);
    kinetic.tick(DT, &extent);
    kinetic.tick(1.0, &extent);
    assert!(kinetic.offset().abs() < 1e-3, "{}", kinetic.offset());
    assert!(kinetic.offset().is_finite() && kinetic.velocity().is_finite());
}

#[test]
fn pulling_past_an_end_gives_less_the_further_it_goes() {
    let extent = Extent::rows(5.0);
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(-1.0, &extent);
    let one = -kinetic.offset();
    kinetic.drag(-1.0, &extent);
    let two = -kinetic.offset();
    kinetic.drag(-100.0, &extent);
    let far = -kinetic.offset();
    assert!(one > 0.0 && one < 1.0, "{one}");
    assert!(
        two - one < one,
        "the second unit gives less: {one} then {two}"
    );
    assert!(far < 2.0, "never past the band: {far}");
    // And let go, it springs back to the end.
    kinetic.release(&extent);
    run(&mut kinetic, &extent, 2.0);
    assert_eq!(kinetic.offset(), 0.0);
}

#[test]
fn a_fling_into_an_end_bounces_back_and_rests_on_it() {
    let extent = Extent::rows(5.0);
    let mut kinetic = Kinetic::new(3.0);
    kinetic.press(&extent);
    swipe(&mut kinetic, &extent, 0.3, 5);
    kinetic.release(&extent);
    let mut furthest: f32 = 0.0;
    while !kinetic.is_still() {
        kinetic.tick(DT, &extent);
        furthest = furthest.max(kinetic.offset());
    }
    assert!(furthest > 5.0, "it overran the end: {furthest}");
    assert!(furthest < 6.0, "but not by much: {furthest}");
    assert_eq!(kinetic.offset(), 5.0);
}

#[test]
fn catching_a_banded_list_holds_it_where_it_is_drawn() {
    let extent = Extent::rows(5.0);
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(-3.0, &extent);
    kinetic.release(&extent);
    kinetic.tick(DT, &extent);
    kinetic.tick(DT, &extent);
    let drawn = kinetic.offset();
    kinetic.press(&extent);
    assert!((kinetic.offset() - drawn).abs() < 1e-5);
    kinetic.drag(0.0, &extent);
    assert!((kinetic.offset() - drawn).abs() < 1e-4, "{drawn}");
}

#[test]
fn wrapping_content_has_no_ends() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    kinetic.drag(-7.0, &extent);
    assert_eq!(kinetic.offset(), -7.0, "no band");
    // A caller folding whole items back out keeps the motion aimed.
    swipe(&mut kinetic, &extent, 0.0, 10);
    kinetic.drag(-0.3, &extent);
    swipe(&mut kinetic, &extent, 0.0, 10);
    kinetic.release(&extent);
    kinetic.tick(DT, &extent);
    kinetic.shift(7.0);
    run(&mut kinetic, &extent, 2.0);
    assert_eq!(kinetic.offset(), 0.0);
}

#[test]
fn a_tap_during_motion_stops_it_and_selects_nothing() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    swipe(&mut kinetic, &extent, 0.2, 6);
    kinetic.release(&extent);
    for _ in 0..5 {
        kinetic.tick(DT, &extent);
    }
    assert!(kinetic.velocity() > 1.0, "moving");
    let caught_at = kinetic.offset();
    let tap = Pointer {
        at: Some((10.0, 10.0)),
        moved: true,
        clicked: true,
        released: true,
        ..pressed()
    };
    let seen = kinetic.gesture(&tap, 0.0, &extent);
    assert!(
        !seen.clicked && !seen.moved && seen.at.is_none(),
        "{seen:?}"
    );
    assert_eq!(kinetic.offset(), caught_at, "stopped where it was");
    run(&mut kinetic, &extent, 2.0);
    assert_eq!(
        kinetic.offset(),
        caught_at.round(),
        "and settled from there"
    );
}

#[test]
fn a_tap_at_rest_still_selects() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    let down = kinetic.gesture(&pressed(), 0.0, &extent);
    assert!(down.pressed);
    let tap = Pointer {
        at: Some((10.0, 10.0)),
        moved: true,
        clicked: true,
        released: true,
        ..Pointer::default()
    };
    let seen = kinetic.gesture(&tap, 0.0, &extent);
    assert_eq!(seen, tap);
    run(&mut kinetic, &extent, 1.0);
    assert!(kinetic.is_still());
    assert_eq!(kinetic.offset(), 0.0);
}

#[test]
fn unsnapped_content_coasts_to_a_stop_wherever_it_stops() {
    let extent = Extent {
        pitch: 30.0,
        max_fling: MAX_FLING,
        friction: FRICTION,
        snap: false,
        bounds: Bounds::Clamp {
            min: 0.0,
            max: 1000.0,
            band: 200.0,
        },
    };
    let mut kinetic = Kinetic::new(100.0);
    kinetic.press(&extent);
    // 6 px a tick is 360 px a second, 12 items a second at this pitch.
    swipe(&mut kinetic, &extent, 6.0, 8);
    let lifted = kinetic.offset();
    kinetic.release(&extent);
    run(&mut kinetic, &extent, 10.0);
    let travelled = kinetic.offset() - lifted;
    assert!(kinetic.is_still());
    assert!(
        (travelled - 360.0 / FRICTION).abs() < 30.0,
        "travelled {travelled}"
    );
}

#[test]
fn the_same_ticks_give_the_same_motion() {
    let extent = Extent::rows(40.0);
    let trace = || {
        let mut kinetic = Kinetic::new(10.0);
        kinetic.press(&extent);
        swipe(&mut kinetic, &extent, 0.37, 7);
        kinetic.release(&extent);
        (0..90)
            .map(|_| {
                kinetic.tick(DT, &extent);
                kinetic.offset().to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(trace(), trace());
}

#[test]
fn a_pad_step_settles_a_scroll_it_interrupts() {
    let extent = Extent::wrapping();
    let mut kinetic = Kinetic::new(0.0);
    kinetic.press(&extent);
    swipe(&mut kinetic, &extent, 0.2, 6);
    kinetic.release(&extent);
    kinetic.tick(DT, &extent);
    kinetic.settle(&extent);
    assert_eq!(kinetic.velocity(), 0.0);
    run(&mut kinetic, &extent, 2.0);
    assert_eq!(kinetic.offset().fract(), 0.0);
}
