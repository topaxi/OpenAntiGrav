//! What the recovered reticle law in [`super`] is asserted to do.
//!
//! Every number here comes off `HudSight_Update` (`0x0881dbcc`); see
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`. Written as literals
//! rather than as the constants they pin, so a constant that moves fails a test
//! instead of moving it too.

use super::*;
use oag_core::math::camera::look_at;

const DT: f32 = 1.0 / 60.0;

/// A target parked at the screen centre, which is the simplest thing to hold.
fn centred() -> Projected {
    Projected {
        screen: [240.0, 136.0],
        distance: 60.0,
    }
}

/// Holding a target for less than `0.8` seconds does not lock it.
///
/// **The rule this file exists for.** Until 2026-08-26 this engine had no
/// acquisition time at all: `Ship_AcquireLock`'s window, cone and along-track
/// screen were the whole lock. They are not - `HudSight_Update` is what writes
/// `entity+0x860 & 1`, and it wants `0.8` seconds of the target being on screen
/// *and* the brackets to have caught up.
#[test]
fn a_lock_takes_eight_tenths_of_a_second_of_holding_it() {
    let mut sight = Sight::default();

    let mut ticks = 0;
    let mut state = State::Absent;
    while state != State::Locked {
        state = sight.update(DT, Some(centred()));
        ticks += 1;
        assert!(ticks < 600, "the reticle never locked on a parked target");
    }

    let held = ticks as f32 * DT;
    assert!(
        held >= 0.8,
        "it locked after {held}s, inside the recovered 0.8s hold"
    );
    // Generous on the upper side: the brackets have to *arrive* as well as the
    // timer expire, and how long that takes is the chase law's business rather
    // than this assertion's.
    assert!(
        held < 1.2,
        "it took {held}s to lock a target that never moved"
    );
}

/// A target that leaves the screen resets the hold, so the next one starts over.
#[test]
fn losing_the_target_resets_the_hold() {
    let mut sight = Sight::default();
    for _ in 0..40 {
        sight.update(DT, Some(centred()));
    }
    sight.update(DT, None);
    assert!(!sight.locked());

    // One tick back on target must not be enough.
    let state = sight.update(DT, Some(centred()));
    assert_eq!(
        state,
        State::Seeking,
        "the hold survived the target going away"
    );
}

/// The brackets close from 30 to 9.6 while seeking and to 6.0 once locked.
#[test]
fn the_brackets_close_as_the_lock_is_taken() {
    let mut sight = Sight::default();
    let opening = sight.brackets();
    let open_extent = opening[3].centre[0] - opening[2].centre[0];
    assert!(
        (open_extent - 60.0).abs() < 0.01,
        "the idle box is {open_extent} across, not 2 * 30"
    );

    while sight.update(DT, Some(centred())) != State::Locked {}
    // Let the extent finish easing to the locked value.
    for _ in 0..60 {
        sight.update(DT, Some(centred()));
    }
    let locked = sight.brackets();
    let locked_extent = locked[3].centre[0] - locked[2].centre[0];
    assert!(
        (locked_extent - 12.0).abs() < 0.5,
        "the locked box is {locked_extent} across, not 2 * 6.0"
    );
}

/// The four brackets sit at the four corners, each rotated a quarter turn on.
///
/// Four instances of one corner-bracket model make a box no other way, and the
/// original writes the angles out as literals rather than mirroring the quad.
#[test]
fn the_four_brackets_are_the_four_corners_at_four_quarter_turns() {
    let mut sight = Sight::default();
    sight.update(DT, Some(centred()));
    let pieces = sight.brackets();

    let cx = (pieces[0].centre[0] + pieces[1].centre[0]) * 0.5;
    let cy = (pieces[0].centre[1] + pieces[2].centre[1]) * 0.5;
    let signs = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];
    for (piece, (sx, sy)) in pieces.iter().zip(signs) {
        assert!(
            (piece.centre[0] - cx).signum() == sx && (piece.centre[1] - cy).signum() == sy,
            "{piece:?} is not at the ({sx}, {sy}) corner"
        );
    }

    let mut turns: Vec<f32> = pieces
        .iter()
        .map(|p| p.rotation / std::f32::consts::FRAC_PI_2)
        .collect();
    turns.sort_by(f32::total_cmp);
    for (index, turn) in turns.iter().enumerate() {
        assert!(
            (turn - index as f32).abs() < 1e-4,
            "the rotations are {turns:?}, not the four quarter turns"
        );
    }
}

/// The inner box leads the brackets, by at most 0.4 of the extent.
#[test]
fn the_inner_box_leads_the_brackets_and_is_clamped() {
    let mut sight = Sight::default();
    // A target far from where the reticle starts, so the centre is still
    // chasing and the inner has somewhere to lead to.
    let far = Projected {
        screen: [420.0, 40.0],
        distance: 60.0,
    };
    sight.update(DT, Some(far));
    sight.update(DT, Some(far));

    let brackets = sight.brackets();
    let extent = (brackets[3].centre[0] - brackets[2].centre[0]) * 0.5;
    let inner = sight.inner();
    let centre = [
        (brackets[0].centre[0] + brackets[1].centre[0]) * 0.5,
        (brackets[0].centre[1] + brackets[2].centre[1]) * 0.5,
    ];
    let lead =
        ((inner.centre[0] - centre[0]).powi(2) + (inner.centre[1] - centre[1]).powi(2)).sqrt();
    assert!(lead > 0.0, "the inner box did not lead at all");
    assert!(
        lead <= 0.4 * extent + 0.01,
        "the inner box led {lead} against a limit of 0.4 * {extent}"
    );
}

/// A target behind the camera projects to nothing rather than to a mirrored
/// point on screen.
///
/// **The original's own `w > 0` guard**, and the one bug it prevents is the kind
/// that survives review: a craft behind you lands somewhere entirely plausible.
#[test]
fn a_target_behind_the_camera_does_not_project() {
    // Looking down -Z from the origin, the standard right-handed setup.
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = oag_render::camera::projection(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let view_projection = projection * view;

    let ahead = project(view, view_projection, Vec3::new(0.0, 0.0, -40.0));
    assert!(ahead.is_some(), "a craft straight ahead did not project");

    let behind = project(view, view_projection, Vec3::new(0.0, 0.0, 40.0));
    assert_eq!(
        behind, None,
        "a craft behind the camera projected on screen"
    );
}

/// And one past the draw range does not either, however square-on it is.
#[test]
fn a_target_past_the_draw_range_does_not_project() {
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = oag_render::camera::projection(1.0, 480.0 / 272.0, 0.1, 4000.0);
    let view_projection = projection * view;

    assert!(project(view, view_projection, Vec3::new(0.0, 0.0, -249.0)).is_some());
    assert_eq!(
        project(view, view_projection, Vec3::new(0.0, 0.0, -251.0)),
        None,
        "a craft past the recovered 250-unit sight range still drew a reticle"
    );
}

/// A craft dead ahead lands in the middle of the screen, not the corner.
///
/// The cheap check that the projection's sign convention did not get flipped -
/// the original works in a `y`-down clip space and this engine does not.
#[test]
fn a_craft_dead_ahead_projects_to_the_middle_of_the_screen() {
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = oag_render::camera::projection(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let projected = project(view, projection * view, Vec3::new(0.0, 0.0, -40.0)).expect("ahead");
    assert!((projected.screen[0] - 240.0).abs() < 0.5, "{projected:?}");
    assert!((projected.screen[1] - 136.0).abs() < 0.5, "{projected:?}");
}

/// A craft above the camera draws its reticle in the *upper* half.
#[test]
fn a_craft_above_the_camera_draws_above_the_middle() {
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = oag_render::camera::projection(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let projected =
        project(view, projection * view, Vec3::new(0.0, 8.0, -40.0)).expect("above and ahead");
    assert!(
        projected.screen[1] < 136.0,
        "a craft above the camera drew at y={}, below the middle - the screen \
         y axis is flipped",
        projected.screen[1]
    );
}

/// Nothing to lock leaves the brackets open and the tone silent.
#[test]
fn with_no_target_the_reticle_opens_and_says_nothing() {
    let mut sight = Sight::default();
    for _ in 0..120 {
        assert_eq!(sight.update(DT, None), State::Absent);
    }
    let pieces = sight.brackets();
    let extent = (pieces[3].centre[0] - pieces[2].centre[0]) * 0.5;
    assert!(
        (extent - 30.0).abs() < 0.01,
        "the idle extent is {extent}, not the recovered 30"
    );
    assert!(!sight.locked());
    assert_eq!(
        sight.alpha(),
        0.0,
        "a reticle with no target is still drawn"
    );
}

/// The reticle is exactly on its target on the frame it locks, never near it.
///
/// **An invariant rather than a tolerance**, and it falls out of the law: a lock
/// is only taken inside the branch that assigns `centre = aim`, so the two
/// cannot disagree on a locking frame. Worth pinning because the obvious
/// "close enough" refactor of that branch - easing all the way and testing the
/// distance afterwards - would break it silently, and the symptom would be a
/// reticle that locks a pixel or two off the craft.
#[test]
fn the_reticle_sits_exactly_on_its_target_when_it_locks() {
    let mut sight = Sight::default();
    let target = Projected {
        screen: [300.0, 90.0],
        distance: 45.0,
    };
    let mut checked = false;
    for _ in 0..600 {
        if sight.update(DT, Some(target)) == State::Locked {
            assert_eq!(
                Some(sight.centre()),
                sight.aim(),
                "the reticle locked without being on its target"
            );
            checked = true;
            break;
        }
    }
    assert!(checked, "the reticle never locked");
}

/// And it closes on a distant target over several frames rather than jumping.
///
/// The chase is what the player sees; a reticle that snapped would show the
/// brackets already shut. Asserted as "it moved, it did not arrive at once, and
/// it got there", which is the property, rather than against the rate itself.
#[test]
fn the_reticle_closes_on_a_distant_target_over_several_frames() {
    let mut sight = Sight::default();
    let target = Projected {
        screen: [440.0, 30.0],
        distance: 200.0,
    };
    let start = sight.centre();
    sight.update(DT, Some(target));
    let after_one = sight.centre();
    assert_ne!(after_one, start, "the reticle did not move at all");
    assert!(
        (after_one[0] - 440.0).abs() > 1.0,
        "the reticle jumped straight onto a target 200 units away"
    );

    for _ in 0..600 {
        if sight.update(DT, Some(target)) == State::Locked {
            return;
        }
    }
    panic!("the reticle never caught up with a stationary target");
}
