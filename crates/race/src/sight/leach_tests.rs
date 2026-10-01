//! The LeachBeam's own reticle law (`FUN_0881e8c8`), asserted tick by tick.

use super::*;

const DT: f32 = 1.0 / 60.0;

fn target(x: f32, y: f32) -> Option<Projected> {
    Some(Projected {
        screen: [x, y],
        distance: 80.0,
    })
}

fn leach() -> Sight {
    let mut sight = Sight::new(SCREEN);
    sight.set_held(Held::LeachBeam);
    sight.set_leach_law(true);
    sight
}

/// Ticks until `done`, up to a second, and says how many it took.
fn ticks_until(sight: &mut Sight, aim: Option<Projected>, done: impl Fn(&Sight) -> bool) -> usize {
    for n in 1..=60 {
        sight.update(DT, aim);
        if done(sight) {
            return n;
        }
    }
    panic!("not reached in a second");
}

#[test]
fn a_first_sighting_closes_in_from_open_and_locks_on_arrival() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    // 30 -> 6 at `dt * 50 * 1.4` a tick: 21 ticks, `0.35` s - no hold timer, so
    // against the Missile's `0.8` s.
    let n = ticks_until(&mut sight, aim, Sight::locked);
    assert!((20..=22).contains(&n), "locked after {n} ticks");
    assert_eq!(sight.extent, LEACH_EXTENT_CLOSED);
    assert_eq!(sight.update(DT, aim), State::Locked);
}

#[test]
fn the_centre_is_the_raw_projection_with_no_chase() {
    let mut sight = leach();
    sight.update(DT, target(400.0, 50.0));
    assert_eq!(sight.centre(), [400.0, 50.0]);
}

#[test]
fn off_screen_is_not_seen_and_the_centre_stays() {
    let mut sight = leach();
    sight.update(DT, target(300.0, 100.0));
    // The Missile's test is on `2 * new - old`; this one is on the point itself,
    // so a point just inside the right edge is still held.
    assert_eq!(sight.update(DT, target(479.5, 100.0)), State::Seeking);
    assert_eq!(sight.update(DT, target(480.5, 100.0)), State::Absent);
    assert_eq!(
        sight.centre(),
        [479.5, 100.0],
        "the centre is left where it was"
    );
}

#[test]
fn losing_the_target_opens_out_and_hides_in_a_quarter_second() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    ticks_until(&mut sight, aim, Sight::locked);
    // `6 -> 9.6` at `dt * 75`, `9.6 -> 30` at `dt * 105`: `0.24` s.
    let n = ticks_until(&mut sight, None, |s| !s.visible());
    assert!((13..=16).contains(&n), "gone after {n} ticks");
}

#[test]
fn the_figure_spins_two_radians_a_second_seeking_and_four_locked() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    sight.update(DT, aim);
    assert!(
        (sight.spin - 2.0 * DT).abs() < 1e-6,
        "seeking: {}",
        sight.spin
    );
    ticks_until(&mut sight, aim, Sight::locked);
    let before = sight.spin;
    sight.update(DT, aim);
    assert!(
        (sight.spin - before - 4.0 * DT).abs() < 1e-5,
        "locked: {}",
        sight.spin - before
    );
}

#[test]
fn the_spin_runs_back_while_the_target_is_let_go_and_stays_wrapped() {
    let mut sight = leach();
    sight.update(DT, target(300.0, 100.0));
    sight.update(DT, None);
    sight.update(DT, None);
    assert!(sight.spin < 0.0, "{}", sight.spin);
    for _ in 0..600 {
        sight.update(DT, None);
        assert!(sight.spin.abs() < std::f32::consts::TAU);
    }
}

#[test]
fn the_pieces_turn_about_the_centre_and_each_carries_the_spin() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    ticks_until(&mut sight, aim, Sight::locked);
    for _ in 0..40 {
        sight.update(DT, aim);
    }
    let pieces = sight.brackets();
    let [cx, cy] = sight.centre();
    for (i, piece) in pieces.iter().enumerate() {
        let d = ((piece.centre[0] - cx).powi(2) + (piece.centre[1] - cy).powi(2)).sqrt();
        assert!(
            (d - 6.0 * 2f32.sqrt()).abs() < 1e-3,
            "piece {i} is {d} from the centre"
        );
        let rotation = BRACKET_ROTATIONS[i] + sight.spin;
        assert!((piece.rotation - rotation).abs() < 1e-6);
    }
    assert_ne!(
        pieces[0].centre,
        [cx - 6.0, cy - 6.0],
        "the corners did not turn"
    );
}

#[test]
fn the_fade_rides_the_extent_and_the_tint_is_yellow_then_red() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    sight.update(DT, aim);
    // Just taken: `6 / extent` is a fifth, and it brightens as the arrowheads close.
    let first = sight.alpha();
    assert!(first < 0.3, "{first}");
    assert_eq!(sight.tint(), [1.0, 1.0, 0.0]);
    ticks_until(&mut sight, aim, Sight::locked);
    assert_eq!(sight.alpha(), 1.0);
    assert_eq!(sight.tint(), [1.0, 0.0, 0.0]);
    // Let go: `1 - extent / 30` from `0.8`, falling to nothing.
    sight.update(DT, None);
    assert!(
        sight.alpha() > 0.5 && sight.alpha() < 0.8,
        "{}",
        sight.alpha()
    );
}

#[test]
fn a_held_missile_and_an_unflagged_leachbeam_keep_the_missiles_law() {
    let aim = target(300.0, 100.0);
    let mut missile = Sight::new(SCREEN);
    missile.set_leach_law(true);
    missile.update(DT, aim);
    assert_eq!(missile.spin, 0.0, "the Missile does not spin");

    let mut hd = Sight::new(SCREEN);
    hd.set_held(Held::LeachBeam);
    hd.update(DT, aim);
    assert_eq!(hd.spin, 0.0, "HD's LeachBeam keeps the Missile's law");
    assert!(hd.hold_progress() > 0.0);
}

/// The opening steps as the original's own frames show them: off a locked `6.0`
/// at 60 Hz the extent reads `7.25`, `8.5`, `9.75` and then `11.5` - three steps
/// of `dt * 50 * 1.5`, and a fourth `1.4` times that once it is past `9.6`.
/// (PPSSPP, the unlocked arm fired at a pinned target: `11.5044` four frames on at
/// the emulator's own `dt`; the whole 298-step capture replays with no error above
/// `1e-5`, `scripts/psp-leach-sight-capture.py replay`.)
#[test]
fn the_arrowheads_open_by_the_steps_the_original_takes() {
    let mut sight = leach();
    let aim = target(300.0, 100.0);
    ticks_until(&mut sight, aim, Sight::locked);
    let opened: Vec<f32> = (0..4)
        .map(|_| {
            sight.update(DT, None);
            sight.extent
        })
        .collect();
    for (got, want) in opened.iter().zip([7.25, 8.5, 9.75, 11.5]) {
        assert!((got - want).abs() < 1e-4, "{opened:?}");
    }
}
