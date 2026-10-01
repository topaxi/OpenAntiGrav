//! The views, the field of view and the boost kick: what the race points at
//! the craft and how wide.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`. The rest of the scene composition is in `scene.rs`.

use super::*;
use oag_gameplay::PlayerInputs;

/// The authored kick widens the view while the boost runs and closes again
/// afterwards, and **turning it off leaves the projection bit-identical** to
/// what it was before the effect existed. That second half is the one that
/// matters: it is what makes the setting usable for a comparison against a
/// capture of the original.
#[test]
fn the_boost_kick_widens_the_view_and_turning_it_off_changes_nothing() {
    use oag_display::display::{BoostFovKick, Fov};

    fn x_scale(m: Mat4) -> f32 {
        m.to_cols_array()[0]
    }

    let mut kicked = race_with_pads(Mode::TimeTrial, enveloping_pad());
    let mut off = race_with_pads(Mode::TimeTrial, enveloping_pad());
    off.set_boost_fov_kick(BoostFovKick::OFF);

    // **Compared at the same tick, not against tick 0.** The original's own
    // speed widen (`SPEED_FOV_GAIN_DEG`) also opens the field as the craft
    // accelerates down the pad, so a tick-0 reference would credit the kick
    // with a widening it did not do - and would make the bit-identical leg
    // below fail for a reason that has nothing to do with the kick. Both
    // races take identical input and the kick feeds nothing back into the
    // simulation, so at any tick they are in the same physical state and the
    // only difference between these two matrices is the kick itself.
    for _ in 0..30 {
        kicked.tick(&PlayerInputs::none());
        off.tick(&PlayerInputs::none());
    }

    let open = kicked.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED);
    let plain = off.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED);
    assert!(
        x_scale(open) < x_scale(plain),
        "a wider field means a smaller x scale, and this one did not move"
    );

    // The other half, and the one that matters: with the kick off the
    // projection is *exactly* the chain that would exist if the effect had
    // never been written - authored fov plus the recovered speed term,
    // fitted to the window, and nothing else. Built here rather than
    // captured, so the assertion cannot be satisfied by both sides drifting
    // together.
    let body = &off.ship().physics.body;
    let expected_deg =
        off.view.chase_params.fov + SPEED_FOV_GAIN_DEG * body.forward().dot(body.linear_velocity);
    let expected = oag_render::camera::projection(
        oag_render::camera::fit_vertical_fov(
            expected_deg.to_radians(),
            AUTHORED_ASPECT,
            16.0 / 9.0,
        ),
        16.0 / 9.0,
        1.0,
        1000.0,
    );
    assert_eq!(
        plain.to_cols_array(),
        expected.to_cols_array(),
        "with the kick off the matrix must be exactly the pre-effect one"
    );

    // Take the pad away and let the boost expire; the kick must return to
    // *exactly* the un-kicked matrix rather than to something near it. Both
    // races are still in lockstep, so `off` is the right reference at
    // whatever speed they have both decayed to.
    kicked.sim.speedup_pads.clear();
    off.sim.speedup_pads.clear();
    for _ in 0..600 {
        kicked.tick(&PlayerInputs::none());
        off.tick(&PlayerInputs::none());
    }
    assert_eq!(
        kicked
            .projection(16.0 / 9.0, 1000.0, Fov::AUTHORED)
            .to_cols_array(),
        off.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED)
            .to_cols_array(),
        "the kick never fully closed"
    );
}

/// The recovered speed widen is live, additive in degrees, and driven by
/// **forward** velocity rather than speed - which is what a craft moving
/// backwards proves, because no speed magnitude can narrow the field below
/// the authored value and the original demonstrably does.
#[test]
fn the_field_widens_with_forward_speed_and_narrows_when_moving_backwards() {
    use oag_display::display::Fov;

    fn vertical_fov_deg(race: &Race) -> f32 {
        // m11 = 1 / tan(fov/2) at the authored aspect, which this window is.
        let m11 = race
            .projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED)
            .to_cols_array()[5];
        2.0 * (1.0 / m11).atan().to_degrees()
    }

    let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
    let authored = race.view.chase_params.fov;
    assert!(
        (vertical_fov_deg(&race) - authored).abs() < 1.0e-3,
        "at rest the field must be the authored one, got {}",
        vertical_fov_deg(&race)
    );

    for speed in [40.0_f32, 100.0, 150.0] {
        let body = &mut race.sim.world.ships[0].physics.body;
        body.linear_velocity = body.forward() * speed;
        let expected = authored + SPEED_FOV_GAIN_DEG * speed;
        assert!(
            (vertical_fov_deg(&race) - expected).abs() < 1.0e-2,
            "at {speed} units/s the field must be {expected}, got {}",
            vertical_fov_deg(&race)
        );
    }

    // The discriminating case, measured on the running original: a sample at
    // negative forward velocity read 54.26 degrees against an authored 60.
    let body = &mut race.sim.world.ships[0].physics.body;
    body.linear_velocity = body.forward() * -76.5;
    assert!(
        vertical_fov_deg(&race) < authored - 5.0,
        "moving backwards must *narrow* the field, got {}",
        vertical_fov_deg(&race)
    );
}

/// The three non-zero tiers are the point of turning the toggle into a
/// magnitude: a player who found [`BoostFovKick::DEFAULT`] too subtle to
/// notice needs the stronger rows to actually be stronger, not just
/// differently labelled.
#[test]
fn a_stronger_tier_widens_the_view_further() {
    use oag_display::display::{BoostFovKick, Fov};

    fn x_scale(m: Mat4) -> f32 {
        m.to_cols_array()[0]
    }

    let widen_at = |tier: BoostFovKick| {
        let mut race = race_with_pads(Mode::TimeTrial, enveloping_pad());
        race.set_boost_fov_kick(tier);
        for _ in 0..30 {
            race.tick(&PlayerInputs::none());
        }
        x_scale(race.projection(16.0 / 9.0, 1000.0, Fov::AUTHORED))
    };

    let scales: Vec<f32> = BoostFovKick::OFFERED
        .iter()
        .copied()
        .map(widen_at)
        .collect();
    // A wider field is a *smaller* x scale, so ascending tiers descend here.
    assert!(
        scales.windows(2).all(|pair| pair[0] > pair[1]),
        "the tiers must widen monotonically: {scales:?}"
    );
}

/// **The load-bearing test of this whole feature.** Cycling the camera is
/// presentation, and the determinism rules say presentation may not reach the
/// simulation - so the proof has to be a hash, not a reading of the code.
///
/// Two races from one seed, driven with identical input for identical tick
/// counts. One of them walks the full camera cycle in the middle, twice
/// round, including in and out of the cockpit view. The exhaustive state hash
/// (`oag_physics::probe::hash_state`, which destructures `ShipState` field by
/// field on purpose) has to come out bit-identical at every tick, and so does
/// the seeded generator.
#[test]
fn cycling_the_camera_changes_no_simulation_state() {
    use oag_core::hash::StateHasher;
    use oag_physics::probe::hash_state;

    fn hash(race: &Race) -> u64 {
        let mut hasher = StateHasher::new();
        hash_state(&mut hasher, &race.ship().physics);
        hasher.write_u64(race.sim.world.tick);
        hasher.finish()
    }

    // Thrust held, so the craft is moving rather than parked: a stationary
    // ship would hash the same however badly the camera behaved.
    let held = oag_gameplay::input::Button::Cross.bit();

    let mut control = Race::start(setup(Handling::default()));
    let mut cycled = Race::start(setup(Handling::default()));

    let mut control_input = HeldButtons::new(held);
    let mut cycled_input = HeldButtons::new(held);

    for tick in 0..240u32 {
        control.tick(&PlayerInputs::single(control_input.snapshot()));
        cycled.tick(&PlayerInputs::single(cycled_input.snapshot()));
        // Eleven changes over the run, so every view is entered and left
        // several times while the craft flies - and eleven is deliberately
        // not a multiple of three, so the run does not end back on the view
        // it started from and the assertions below have something to see.
        if tick > 0 && tick % 20 == 0 {
            cycled.set_camera_view(cycled.camera_view().next());
        }
        assert_eq!(hash(&control), hash(&cycled), "diverged at tick {tick}");
        // The seeded generator too, by value: a camera that drew a random
        // number from the *simulation's* generator - rather than from the
        // separate ones `Race` keeps for the exhaust and the sparks - would
        // desynchronise every later tick, and this catches it on the first.
        assert_eq!(
            control.sim.world.rng, cycled.sim.world.rng,
            "the simulation's generator moved at tick {tick}"
        );
    }

    // The cycle really did move: otherwise this test passes by doing nothing,
    // which is the failure mode a test of this shape is prone to.
    assert_eq!(
        control.camera_view(),
        oag_display::display::CameraView::default()
    );
    assert_ne!(cycled.camera_view(), control.camera_view());
    // And the two cameras really are looking at different things, so the
    // hashes above are equal despite a genuinely different picture.
    assert_ne!(control.view(), cycled.view());
}

/// The three views have to be three *different* cameras. A dispatch that fell
/// through to the far block for all of them would satisfy every other test
/// here.
#[test]
fn each_view_frames_the_craft_from_its_own_block() {
    use oag_display::display::{CameraView, Fov};

    let mut race = Race::start(setup(Handling::default()));
    let mut eyes = Vec::new();
    let mut fovs = Vec::new();
    for view in CameraView::ALL {
        race.set_camera_view(view);
        eyes.push((view, race.camera_position()));
        fovs.push((
            view,
            race.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED),
        ));
    }

    for (index, (view, eye)) in eyes.iter().enumerate() {
        for (other, other_eye) in &eyes[index + 1..] {
            assert!(
                (*eye - *other_eye).length() > 1e-3,
                "{view} and {other} share an eye at {eye}"
            );
        }
    }
    // Each block authors its own fov in the fixture, so the projections have
    // to differ too - which is what catches a `view()` that dispatches beside
    // a `projection()` that does not.
    for (index, (view, projection)) in fovs.iter().enumerate() {
        for (other, other_projection) in &fovs[index + 1..] {
            assert_ne!(projection, other_projection, "{view} and {other}");
        }
    }

    // The geometry, not just the difference: the cockpit eye is *ahead* of the
    // craft and both chase eyes are behind it. A sign error here is the single
    // most likely mistake in the whole feature.
    let craft = race.ship().physics.body.position;
    let forward = race.ship().physics.body.forward();
    race.set_camera_view(CameraView::Internal);
    assert!((race.camera_position() - craft).dot(forward) > 0.0);
    for view in [CameraView::Close, CameraView::Far] {
        race.set_camera_view(view);
        assert!(
            (race.camera_position() - craft).dot(forward) < 0.0,
            "{view} is in front of the craft"
        );
    }
    // And close is closer than far, which is what the two names mean.
    race.set_camera_view(CameraView::Close);
    let close = (race.camera_position() - craft).length();
    race.set_camera_view(CameraView::Far);
    assert!(close < (race.camera_position() - craft).length());

    // The hull is drawn in both external views and in neither of the other
    // states, which is the one thing `Scene::render` branches on.
    for view in [CameraView::Close, CameraView::Far] {
        race.set_camera_view(view);
        assert!(race.draws_own_ship(), "{view}");
    }
    race.set_camera_view(CameraView::Internal);
    assert!(!race.draws_own_ship());
}

/// A race starts on the nearer chase block, as a fresh profile of the original
/// does: two cold boots read the player camera 11.644 units from the craft, which
/// is `<ExternalCameraClose>` at the 0.75 scale (`Far` is 14.56). With the round
/// fixture numbers (`chase_close` offsets `(4, 1)`, scale 1) that is `hypot(4, 1)`
/// from the craft and not the far block's `hypot(8, 2)`.
#[test]
fn a_race_starts_on_the_close_chase_block() {
    use oag_display::display::CameraView;

    let race = Race::start(setup(Handling::default()));
    assert_eq!(race.camera_view(), CameraView::Close);
    let craft = race.ship().physics.body.position;
    let distance = (race.camera_position() - craft).length();
    assert!(
        (distance - 4.0_f32.hypot(1.0)).abs() < 0.01,
        "eye is {distance} from the craft, the close block's is {}",
        4.0_f32.hypot(1.0)
    );
}

/// The craft's global `0.75` scale reaches the external rig and none of the
/// internal one. Both halves are measurements off the original - see
/// [`chase_params`] and `oag_render::camera::internal` - and both are cheap to
/// undo by accident.
///
/// It reaches the rig as [`ChaseParams::craft_scale`] and **not** as four
/// pre-multiplied offsets, which is the correction this test pins: the two
/// differ once the spring has state, by `0.121` RMS against `0.008` over the
/// capture. Asserting the four offsets come through *unscaled* is what stops
/// somebody folding the scale back in and double-applying it.
#[test]
fn the_external_blocks_carry_the_crafts_global_scale() {
    let scale = oag_physics::hover::TARGET_GLOBAL_SCALE;
    let block = handling::ExternalCamera {
        fov: 60.0,
        lookat_height: 4.0,
        lookat_length: 8.0,
        pos_height: 4.0,
        pos_length: -16.0,
        spring_horiz: 3.0,
        spring_vert: 5.0,
    };
    let params = chase_params(block);

    assert_eq!(params.craft_scale, scale);
    // Authored, unscaled: the rig applies `craft_scale` itself, at the end,
    // about the craft, which is where the original applies it.
    assert_eq!(params.lookat_height, 4.0);
    assert_eq!(params.lookat_length, 8.0);
    assert_eq!(params.pos_height, 4.0);
    // Negated but not scaled: the sign flip is [`chase_pos_length`]'s.
    assert_eq!(params.pos_length, 16.0);
    // Not lengths, so never scaled wherever the scale is applied.
    assert_eq!(params.fov, 60.0);
    assert_eq!(params.spring_horiz, 3.0);
    assert_eq!(params.spring_vert, 5.0);

    // The number three independent measurements of the original agree on: an
    // authored `(-15, +4)` close block puts the eye 11.64 units from the
    // craft. See [`chase_params`] for all three. Measured off the settled
    // camera the rig actually produces, so it covers the scale wherever the
    // scale now lives.
    let close = chase_params(handling::ExternalCamera {
        pos_height: 4.0,
        pos_length: -15.0,
        ..block
    });
    let target = oag_render::camera::chase::Target {
        position: Vec3::ZERO,
        forward: Vec3::X,
        up: Vec3::Y,
    };
    let distance = (oag_render::camera::chase::anchor(target, &close) - target.position).length();
    assert!((distance - 11.643).abs() < 0.01, "{distance}");
}

/// A camera override must move every reader of the camera at once: the
/// view matrix, and through it the derived eye that PVS culling and fog
/// sampling read. A consumer left on the chase camera would frame the
/// geometry from one place and cull it from another - wrong picture, no
/// error - which is why `Race::view` is the single seam.
#[test]
fn a_camera_override_moves_the_view_and_the_derived_eye_together() {
    let eye = Vec3::new(12.0, 34.0, -56.0);
    let orientation = Quat::from_rotation_y(0.83);
    let mut setup = setup(Handling::default());
    setup.camera_override = Some(CameraOverride {
        eye,
        orientation,
        fov_deg: None,
    });
    let race = Race::start(setup);

    assert!((race.camera_position() - eye).length() < 1e-4);
    // The view maps the eye to the origin and world axes into camera axes:
    // a point one unit along the camera's own -Z lands on (0, 0, -1).
    let ahead = eye + orientation * Vec3::NEG_Z;
    let mapped = race.view().transform_point3(ahead);
    assert!((mapped - Vec3::NEG_Z).length() < 1e-4, "{mapped}");
}

/// The fov half of the override slots in exactly where the authored value
/// sits, so a calibrated value and the disc's own go through the same
/// setting and the same aspect fit.
#[test]
fn a_camera_fov_override_stands_in_for_the_authored_fov() {
    use oag_display::display::Fov;

    let mut with_override = setup(Handling::default());
    with_override.camera_override = Some(CameraOverride {
        eye: Vec3::ZERO,
        orientation: Quat::IDENTITY,
        fov_deg: Some(75.0),
    });
    let overridden = Race::start(with_override);

    let mut same_authored = setup(Handling::default());
    // Both external blocks, so the test does not care which one the default view
    // flies with (it is the close one since 2026-10-01).
    same_authored.chase = ChaseParams {
        fov: 75.0,
        ..same_authored.chase
    };
    same_authored.chase_close = ChaseParams {
        fov: 75.0,
        ..same_authored.chase_close
    };
    let authored = Race::start(same_authored);

    assert_eq!(
        overridden.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED),
        authored.projection(AUTHORED_ASPECT, 1000.0, Fov::AUTHORED)
    );
}

/// The field-of-view setting has to reach the matrix, and its default has
/// to leave that matrix exactly as it was before the setting existed - the
/// ground-truth captures under `data/traces/` were taken against it.
#[test]
fn the_field_of_view_setting_widens_the_projection_and_defaults_to_the_authored_one() {
    use oag_display::display::Fov;

    let race = Race::start(setup(Handling::default()));
    let aspect = AUTHORED_ASPECT;
    let authored = race.projection(aspect, 1000.0, Fov::AUTHORED);

    // The projection's first column scales x by `1 / (tan(fov/2) * aspect)`,
    // so a wider field is a *smaller* number there. Read off the matrix
    // rather than recomputed, which would only restate `Fov::apply`.
    let x_scale = |m: Mat4| m.col(0).x;
    let wide: Fov = "150".parse().expect("parse");
    let narrow: Fov = "75".parse().expect("parse");
    assert!(
        x_scale(race.projection(aspect, 1000.0, wide)) < x_scale(authored),
        "150 % has to show more"
    );
    assert!(
        x_scale(race.projection(aspect, 1000.0, narrow)) > x_scale(authored),
        "75 % has to show less"
    );

    // And the untouched setting is the matrix the fit alone produces.
    let unchanged = oag_render::camera::projection(
        oag_render::camera::fit_vertical_fov(
            race.view.chase_params.fov.to_radians(),
            AUTHORED_ASPECT,
            aspect,
        ),
        aspect,
        1.0,
        1000.0,
    );
    assert_eq!(authored, unchanged);
}

/// An armed shake must never move the camera's own world position - it is a
/// rotation of the view, not a translation of the eye, both by the original's
/// own reading (`collision-shake.md`'s "not a positional jitter") and by this
/// implementation's own composition side (`oag_render::camera::shake`'s
/// closing paragraph proves left-multiplying a rotation's inverse onto the
/// view always fixes the eye, for any rotation at all). This is the test
/// that would have caught composing the shake onto the wrong side of the
/// view matrix: get that wrong and the eye visibly orbits the world origin
/// the instant a shake arms.
#[test]
fn an_active_shake_never_moves_the_camera_eye() {
    let mut race = Race::start(setup(Handling::default()));
    let before = race.camera_position();

    race.view.shake.arm(
        1.0,
        oag_render::camera::shake::Side::Ahead,
        &mut Rng::new(1),
    );
    let just_armed = race.camera_position();
    assert!(
        (before - just_armed).length() < 1e-4,
        "before={before}, just_armed={just_armed}"
    );

    // And through the decay, not only at the moment of arming.
    for _ in 0..20 {
        race.view.shake.advance(1.0 / 60.0);
        let during = race.camera_position();
        assert!(
            (before - during).length() < 1e-4,
            "before={before}, during={during}"
        );
    }
}

/// The `Elsewhere` mode must move the camera at all once armed - otherwise
/// the eye-invariant test above would pass vacuously for a shake that
/// silently does nothing.
#[test]
fn an_active_shake_does_rotate_the_view() {
    let mut race = Race::start(setup(Handling::default()));
    let level = race.view();

    race.view.shake.arm(
        1.0,
        oag_render::camera::shake::Side::Elsewhere,
        &mut Rng::new(1),
    );
    assert_ne!(race.view(), level);
}

/// The map the motion blur is given must take the shake's own screen motion
/// out of the velocity a still camera writes: for a world point seen once
/// through last tick's view and once through this tick's shaken one, the
/// velocity less the map's prediction is zero. Run both from a camera that
/// had no shake the tick before and from one already mid-shake, because the
/// second is the usual case (the shake lasts 0.6 s) and needs the stored
/// previous rotation. A map that is the identity, or built from the wrong
/// side of either rotation, fails both.
#[test]
fn the_blur_is_given_the_shakes_own_screen_motion() {
    let mut race = Race::start(setup(Handling::default()));
    let projection = race.projection(1.7, 1000.0, oag_display::display::Fov::default());
    let still = race.view_unshaken();
    let forward = race.ship().physics.body.position + race.ship().physics.body.forward() * 20.0;
    let points = [
        forward,
        forward + Vec3::new(8.0, 3.0, -2.0),
        forward - Vec3::Y * 4.0,
    ];
    let ndc = |clip: oag_core::math::Vec4| clip.truncate().truncate() / clip.w;
    let mut previous_shake = race.view_shake_rotation();
    assert_eq!(previous_shake, Mat4::IDENTITY);
    race.view.shake.arm(
        1.0,
        oag_render::camera::shake::Side::Elsewhere,
        &mut Rng::new(1),
    );
    for tick in 0..4 {
        let previous_view = previous_shake * still;
        let current = projection * race.view();
        let map = race.shake_screen_motion(projection, previous_shake);
        assert_ne!(
            map,
            Mat4::IDENTITY,
            "tick {tick}: the shake did not reach the map"
        );
        for point in points {
            let now = current * point.extend(1.0);
            let before = projection * previous_view * point.extend(1.0);
            let velocity = ndc(now) - ndc(before);
            let predicted = ndc(now) - ndc(map * ndc(now).extend(0.5).extend(1.0));
            assert!(
                velocity.length() > 1e-4,
                "tick {tick}: no shake motion to take out at {point}"
            );
            assert!(
                (velocity - predicted).length() < 1e-4,
                "tick {tick}: velocity {velocity} against the map's {predicted} at {point}"
            );
        }
        previous_shake = race.view_shake_rotation();
        race.view.shake.advance(1.0 / 60.0);
    }
}

/// With no shake in either tick the map is exactly the identity, so the blur
/// of an unshaken frame is untouched to the bit.
#[test]
fn no_shake_gives_the_blur_exactly_the_identity() {
    let race = Race::start(setup(Handling::default()));
    let projection = race.projection(1.7, 1000.0, oag_display::display::Fov::default());
    assert_eq!(
        race.shake_screen_motion(projection, Mat4::IDENTITY),
        Mat4::IDENTITY
    );
}
