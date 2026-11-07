//! What the recovered reticle law in [`super`] is asserted to do.
//!
//! Every number here comes off `HudSight_Update` (`0x0881dbcc`); see
//! `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`. Written as literals
//! rather than as the constants they pin, so a constant that moves fails a test
//! instead of moving it too.

#![expect(
    clippy::disallowed_methods,
    reason = "test fixtures and reference values: poses and angles built with glam and the \
              platform libm, which no simulation path calls"
)]

use super::*;
use oag_core::math::camera::{look_at, perspective};

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
    let projection = perspective(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let view_projection = projection * view;

    let ahead = project(view, view_projection, Vec3::new(0.0, 0.0, -40.0), SCREEN);
    assert!(ahead.is_some(), "a craft straight ahead did not project");

    let behind = project(view, view_projection, Vec3::new(0.0, 0.0, 40.0), SCREEN);
    assert_eq!(
        behind, None,
        "a craft behind the camera projected on screen"
    );
}

/// And one past the draw range does not either, however square-on it is.
#[test]
fn a_target_past_the_draw_range_does_not_project() {
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = perspective(1.0, 480.0 / 272.0, 0.1, 4000.0);
    let view_projection = projection * view;

    assert!(project(view, view_projection, Vec3::new(0.0, 0.0, -249.0), SCREEN).is_some());
    assert_eq!(
        project(view, view_projection, Vec3::new(0.0, 0.0, -251.0), SCREEN),
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
    let projection = perspective(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let projected =
        project(view, projection * view, Vec3::new(0.0, 0.0, -40.0), SCREEN).expect("ahead");
    assert!((projected.screen[0] - 240.0).abs() < 0.5, "{projected:?}");
    assert!((projected.screen[1] - 136.0).abs() < 0.5, "{projected:?}");
}

/// A craft above the camera draws its reticle in the *upper* half.
#[test]
fn a_craft_above_the_camera_draws_above_the_middle() {
    let view = look_at(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let projection = perspective(1.0, 480.0 / 272.0, 0.1, 1000.0);
    let projected = project(view, projection * view, Vec3::new(0.0, 8.0, -40.0), SCREEN)
        .expect("above and ahead");
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

/// The reticle starts on the Missile's art, which is what a title with no
/// LeachBeam draws for every lock it ever takes.
#[test]
fn a_fresh_reticle_wears_the_missiles_art() {
    assert_eq!(Sight::default().held(), Held::Missile);
}

/// And the art is **sticky**: it holds through the frames the brackets spend
/// easing back open after a target is lost.
///
/// The original keeps all four widgets up while the extent is still moving -
/// [`Sight::visible`] is that rule - so a set swapped out the moment the pickup
/// slot emptied would finish a LeachBeam's closing arrowheads as the Missile's
/// brackets. Nothing sets `held` back; `Race::update_sight` only ever writes it
/// for a weapon that locks.
#[test]
fn the_art_holds_while_the_brackets_ease_back_open() {
    let mut sight = Sight::default();
    sight.set_held(Held::LeachBeam);

    let target = Projected {
        screen: [240.0, 136.0],
        distance: 60.0,
    };
    for _ in 0..120 {
        sight.update(DT, Some(target));
    }
    assert_eq!(sight.held(), Held::LeachBeam);

    // Target gone, and the extent has not finished opening: still drawn, and
    // still drawn as arrowheads.
    sight.update(DT, None);
    assert!(sight.visible(), "the brackets stopped being drawn at once");
    assert_eq!(
        sight.held(),
        Held::LeachBeam,
        "the art changed under a reticle that is still closing"
    );
}

/// A reticle that follows a lock takes it from the caller and not from its own
/// hold, on both laws, and only while it has a target on screen.
#[test]
fn a_following_reticle_shows_the_lock_it_is_handed() {
    for leach in [false, true] {
        let mut sight = Sight::default();
        sight.set_leach_law(leach);
        if leach {
            sight.set_held(Held::LeachBeam);
        }
        // Well inside the hold of either law.
        assert_eq!(
            sight.follow(DT, Some(centred()), true),
            State::Locked,
            "leach law {leach}: a lock handed in was not shown"
        );
        assert!(sight.locked());
        assert_eq!(
            sight.follow(DT, None, true),
            State::Absent,
            "leach law {leach}: a lock was shown with nothing on screen"
        );
        assert!(!sight.locked());
        // And handed `false` it stays unlocked however long it holds.
        let mut sight = Sight::default();
        sight.set_leach_law(leach);
        if leach {
            sight.set_held(Held::LeachBeam);
        }
        for _ in 0..120 {
            sight.follow(DT, Some(centred()), false);
        }
        assert!(!sight.locked(), "leach law {leach}: it locked on its own");
    }
}

/// The lock's hash covers what decides the next lock and not what only draws, so
/// two reticles that hash equal lock on the same tick as each other: the same
/// hold gives the same hash whatever alpha, blink and spin say, and a different
/// hold gives a different one.
#[test]
fn the_lock_hash_covers_the_hold_and_not_the_picture() {
    let hash = |sight: &Sight| {
        let mut hasher = oag_core::hash::StateHasher::new();
        sight.hash_lock(&mut hasher);
        hasher.finish()
    };
    let mut a = Sight::default();
    let mut b = Sight::default();
    for _ in 0..10 {
        a.update(DT, Some(centred()));
        b.update(DT, Some(centred()));
    }
    assert_eq!(hash(&a), hash(&b));
    b.alpha = 17.0;
    b.blink = !b.blink;
    b.blink_timer = 0.05;
    b.spin = 1.0;
    assert_eq!(hash(&a), hash(&b), "the hash reads what only draws");
    for _ in 0..5 {
        a.update(DT, Some(centred()));
        b.update(DT, Some(centred()));
    }
    assert_eq!(a.locked(), b.locked());
    b.update(DT, Some(centred()));
    assert_ne!(hash(&a), hash(&b), "the hash ignores the hold");
}

/// The PS2 runs the law in a 512x512 buffer: a locked reticle's brackets close
/// to `6.0` of those units, which the 640x448 grid stretches by 1.25 across and
/// 0.875 down, and the idle chase point is the literal `(240, 136)` in it.
///
/// Measured on PCSX2 against the original (`docs/ui/hud.md`): the brackets of a
/// locked reticle sit `+-6.0` px of the 512 buffer around the inner box.
#[test]
fn a_ps2_reticle_runs_in_its_own_512_square_and_is_stretched_into_the_grid() {
    let mut sight = Sight::new([640.0, 448.0]).in_span([512.0, 512.0]);
    assert_eq!(sight.unit(), [1.25, 0.875]);
    assert!(!sight.tinted());
    assert_eq!(sight.centre(), [300.0, 119.0], "idle is (240, 136) in 512");

    let target = Projected {
        screen: [320.0, 224.0],
        distance: 60.0,
    };
    let mut ticks = 0;
    while !sight.locked() {
        sight.update(DT, Some(target));
        ticks += 1;
        assert!(ticks < 900, "the reticle never locked");
    }
    for _ in 0..120 {
        sight.update(DT, Some(target));
    }
    let [cx, cy] = sight.centre();
    assert!(
        (cx - 320.0).abs() < 0.01 && (cy - 224.0).abs() < 0.01,
        "{cx} {cy}"
    );
    let zoom = (CHASE_STEP / 60.0_f32).clamp(1.0, 5.0);
    let e = EXTENT_LOCKED * zoom;
    let pieces = sight.brackets();
    assert!(
        (pieces[0].centre[0] - (320.0 - e * 1.25)).abs() < 0.01,
        "{pieces:?}"
    );
    assert!(
        (pieces[0].centre[1] - (224.0 - e * 0.875)).abs() < 0.01,
        "{pieces:?}"
    );
    assert_eq!(sight.tint(), [1.0, 1.0, 1.0]);
}

/// Wipeout HD's Missile reticle on a 1920x1080 grid, with its chase gain.
fn hd_missile_reticle() -> Sight {
    let mut sight = Sight::new([1920.0, 1080.0]);
    sight.set_chase_gain(4.0);
    sight
}

/// One reticle step from a recorded centre and target, as `(centre, target)` after it.
fn step_from(
    mut sight: Sight,
    previous_target: [f32; 2],
    centre: [f32; 2],
    target: [f32; 2],
) -> [f32; 2] {
    sight.previous = Some(previous_target);
    sight.centre = centre;
    sight.update(
        1.0 / 30.0,
        Some(Projected {
            screen: target,
            distance: 100.0,
        }),
    );
    sight.centre
}

/// HD's chase step is the PSP's times `4.0`, frame for frame.
///
/// Each row is one frame read off the running original (RPCS3, `hud+0xd4/0xd8` the
/// chase centre, `hud+0xe0/0xe4` the target's projection, a sighting closing in at
/// `1/30` s a frame with the target past 30 units so the zoom is 1): the target one
/// frame earlier, the centre one frame earlier, the target now, and the centre the
/// original left. On the chasing rows the original moved `4.00` times the step the
/// PSP's law takes (`docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`, "Moving
/// pursuit"; the table is rounded to a thousandth and the frame time taken as exactly
/// `1/30` s, hence the quarter pixel allowed); on the two arrived rows it snapped to the target from `12.8` and `9.0`
/// px away, past the `3.3` and `2.8` px a gain of one allows.
#[test]
fn hd_chases_at_four_times_the_psp_step_as_the_original_does() {
    // (previous target, previous centre, target, centre after)
    let frames = [
        // dist 65.5 chasing
        (
            [1157.822, 511.476],
            [1045.617, 579.133],
            [1073.543, 521.434],
            [1010.186, 549.117],
        ),
        // dist 23.0 chasing
        (
            [1073.543, 521.434],
            [1010.186, 549.117],
            [1030.398, 536.266],
            [991.490, 550.731],
        ),
        // dist 16.7 chasing
        (
            [1030.398, 536.266],
            [991.490, 550.731],
            [1004.103, 550.279],
            [978.843, 563.266],
        ),
        // dist 12.8 arrived
        (
            [1004.103, 550.279],
            [978.843, 563.266],
            [985.955, 561.432],
            [967.807, 572.585],
        ),
        // dist 9.0 arrived
        (
            [985.955, 561.432],
            [967.807, 572.585],
            [972.877, 570.006],
            [959.799, 578.579],
        ),
        // dist 116.8 chasing
        (
            [1179.347, 610.959],
            [1109.813, 639.837],
            [1200.749, 602.510],
            [1175.474, 613.081],
        ),
        // dist 19.8 chasing
        (
            [1223.445, 554.239],
            [1215.169, 541.853],
            [1216.868, 561.778],
            [1210.951, 565.610],
        ),
        // dist 95.9 chasing
        (
            [1076.123, 613.279],
            [1075.306, 614.809],
            [1027.811, 611.732],
            [1018.124, 612.049],
        ),
    ];
    for (previous_target, centre, target, after) in frames {
        let hd = step_from(hd_missile_reticle(), previous_target, centre, target);
        let psp = step_from(
            Sight::new([1920.0, 1080.0]),
            previous_target,
            centre,
            target,
        );
        let miss = |at: [f32; 2]| (at[0] - after[0]).hypot(at[1] - after[1]);
        assert!(
            miss(hd) < 0.25,
            "HD's gain left the centre {hd:?} from {centre:?}; the original left {after:?}"
        );
        assert!(
            miss(psp) > 1.0,
            "the PSP's step reaches {psp:?}, which would make the gain unmeasured"
        );
    }
}

/// A target crossing the screen at a few pixels a tick is caught on HD and trails
/// the PSP's reticle for good.
///
/// `3.7` px a tick is the `7.4` px a frame the original's target moved at the lock
/// in the live run. The PSP's arrival radius is under a pixel at 60 Hz and its lag
/// behind such a target tens of pixels, so the hold ends with the brackets still
/// on their way.
#[test]
fn a_target_crossing_the_screen_locks_with_the_hd_gain_and_never_without_it() {
    let lock_tick = |mut sight: Sight| {
        (0..300u32).find(|tick| {
            let state = sight.update(
                DT,
                Some(Projected {
                    screen: [400.0 + 3.7 * *tick as f32, 540.0],
                    distance: 60.0,
                }),
            );
            state == State::Locked
        })
    };
    let hd = lock_tick(hd_missile_reticle()).expect("HD never caught a crossing target");
    assert!((48..80).contains(&hd), "HD locked at tick {hd}");
    assert_eq!(lock_tick(Sight::new([1920.0, 1080.0])), None);
}
