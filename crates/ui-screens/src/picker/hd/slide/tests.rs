use super::*;

const DT: f32 = 1.0 / 60.0;

fn ticks(slide: &mut Slide, n: usize, target: usize) {
    for _ in 0..n {
        slide.tick(DT, target, 12);
    }
}

/// Frame 0 is the tick the step lands on: the columns have not moved. Half a pitch at frame 3, and
/// at frame 6 the slide is over (the columns are back in their slots with the new window).
#[test]
fn a_step_slides_a_pitch_over_six_ticks_and_then_snaps() {
    let mut slide = Slide::at_rest(2);
    slide.tick(DT, 3, 12);
    assert!(slide.moving());
    assert_eq!(
        (slide.previous(), slide.shown(), slide.direction()),
        (2, 3, -1)
    );
    assert_eq!(slide.progress(), 0.0);
    ticks(&mut slide, 3, 3);
    assert!(slide.moving());
    assert!(
        (slide.progress() - 0.5).abs() < 1e-4,
        "{}",
        slide.progress()
    );
    ticks(&mut slide, 3, 3);
    assert!(!slide.moving(), "frame 6: snapped");
}

#[test]
fn a_backward_step_slides_the_other_way_and_the_wrap_is_forward() {
    let mut slide = Slide::at_rest(3);
    slide.tick(DT, 2, 12);
    assert_eq!(slide.direction(), 1);
    let mut wrap = Slide::at_rest(11);
    wrap.tick(DT, 0, 12);
    assert_eq!(wrap.direction(), -1);
    let mut back = Slide::at_rest(0);
    back.tick(DT, 11, 12);
    assert_eq!(back.direction(), 1);
}

/// A selection that moves while a slide runs is not dropped: the screen compares its focused entry
/// with the one last stepped to each frame, so the step fires on the first frame after the timer
/// ends, and the slide shows the previous window until then.
#[test]
fn a_step_during_a_slide_waits_for_its_end() {
    let mut slide = Slide::at_rest(2);
    slide.tick(DT, 3, 12);
    ticks(&mut slide, 2, 4);
    assert_eq!(
        (slide.previous(), slide.shown()),
        (2, 3),
        "still the first step"
    );
    ticks(&mut slide, 4, 4);
    assert!(!slide.moving(), "the first slide ended");
    assert_eq!(slide.shown(), 3);
    slide.tick(DT, 4, 12);
    assert!(slide.moving(), "the deferred step fires");
    assert_eq!(
        (slide.previous(), slide.shown(), slide.progress()),
        (3, 4, 0.0)
    );
}

/// An even team's column sits half a row higher than an odd one's: stepping onto an odd team leaves
/// the ring a half row above its target, and its offset halves on every tick.
#[test]
fn the_ring_closes_half_its_distance_every_tick() {
    let mut slide = Slide::at_rest(2);
    slide.tick(DT, 3, 12);
    assert_eq!(slide.ring_offset(), -1.0);
    ticks(&mut slide, 1, 3);
    assert_eq!(slide.ring_offset(), -0.5);
    ticks(&mut slide, 2, 3);
    assert_eq!(slide.ring_offset(), -0.125);
    let mut odd = Slide::at_rest(3);
    odd.tick(DT, 4, 12);
    assert_eq!(odd.ring_offset(), 1.0);
}

/// A still `age` seconds after a forward step is that far into the slide.
#[test]
fn a_still_is_seeded_mid_slide() {
    let slide = Slide::stepped(3, 12, 3.0 * DT);
    assert!(slide.moving());
    assert_eq!((slide.previous(), slide.direction()), (2, -1));
    assert!((slide.progress() - 0.5).abs() < 1e-4);
    assert_eq!(slide.ring_offset(), -0.125);
}
