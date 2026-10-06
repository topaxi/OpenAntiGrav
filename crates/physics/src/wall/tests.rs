//! What the wall-contact response in [`super`] is asserted to do. Split out of `wall.rs` under the
//! 200-line cap on inline `#[cfg(test)]` modules (`scripts/check-file-size.py`).

use super::*;
use crate::collide::{CollisionWorld, TriangleSoup};
use crate::params::Dimensions;
use crate::ship::Body;
use oag_core::math::Quat;

/// A ship two units wide, four long, one high, at the origin.
fn handling() -> Handling {
    Handling {
        dimensions: Dimensions {
            width: 2.0,
            height: 1.0,
            length: 4.0,
            ..Handling::ZERO.dimensions
        },
        ..Handling::ZERO
    }
}

/// A large quad in the plane `x = at`, wound so its **raw** normal is `-x`. The winding is
/// load-bearing for single-sided contacts: `segment_triangle` takes `(b - a) x (c - a)` and every
/// ship here approaches from `x < at`, so the triangles are indexed `[0, 2, 1]` / `[0, 3, 2]`.
/// The other way round the quad is a back face with no contact, as in the original.
fn wall(at: f32, surface: Surface) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [at, -100.0, -100.0],
            [at, 100.0, -100.0],
            [at, 100.0, 100.0],
            [at, -100.0, 100.0],
        ],
        vec![[0, 2, 1], [0, 3, 2]],
        Vec::new(),
        surface,
        0,
    ));
    world
}

/// A quad in the plane `x = at` narrow enough that exactly **one** hull sample point reaches it.
///
/// With [`handling`]'s box at the origin the five right-hand sample points share
/// `x = centre + 1.0`, so a full-width plane catches all five and applies the friction law five
/// times (the original's behaviour, pinned in
/// `every_penetrating_sample_point_is_resolved_not_just_the_deepest`, but not what the *law*
/// tests measure). The window is from the ray geometry: the right flank point at
/// `(centre + 1, 0, -0.5)` crosses `x = at` at `y = 0`, the four right corners at
/// `y = +-0.3, z = +-1.2`, so `|y| <= 0.2`, `-1.0 <= z <= 0.5` admits the flank point only.
fn narrow_wall(at: f32, surface: Surface) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [at, -0.2, -1.0],
            [at, 0.2, -1.0],
            [at, 0.2, 0.5],
            [at, -0.2, 0.5],
        ],
        vec![[0, 2, 1], [0, 3, 2]],
        Vec::new(),
        surface,
        0,
    ));
    world
}

/// A quad in the plane `x = at`, as one collider with a given surface,
/// pushed onto an existing world so several can be stacked along one probe.
fn push_quad(world: &mut CollisionWorld, at: f32, surface: Surface, collider: u32) {
    world.push(TriangleSoup::new(
        vec![
            [at, -100.0, -100.0],
            [at, 100.0, -100.0],
            [at, 100.0, 100.0],
            [at, -100.0, 100.0],
        ],
        vec![[0, 2, 1], [0, 3, 2]],
        Vec::new(),
        surface,
        collider,
    ));
}

fn ship_at(x: f32, vx: f32) -> ShipState {
    ShipState {
        body: Body {
            position: Vec3::new(x, 0.0, 0.0),
            linear_velocity: Vec3::new(vx, 0.0, 0.0),
            mass: 1.0,
            ..Body::default()
        },
        ..ShipState::default()
    }
}

/// The headline behaviour: a ship overlapping a wall and moving into it comes back out *and*
/// reverses a little. It must fail loudly if the hull box regresses to zero, so the dimensions
/// are set explicitly rather than left at `Handling::ZERO`.
///
/// The rebound is no longer `-(1 + e) * vn` exactly: with the angular term the denominator is
/// `invMass + n . ((I^-1 (r x n)) x r)`, which for this contact is
///
/// ```text
/// r      = (0.75, 0, -0.375)   the right flank point, relative to the centre
/// r x n  = (0, 0.375, 0)       with n = (-1, 0, 0)
/// I^-1   = (1/15.6, 1/21.6, 1/15.6)
/// term   = n . ((0, 0.01736, 0) x r) = 0.006510
/// j      = 1.4 * 10 / 1.006510 = 13.909
/// ```
///
/// so `10` in becomes `3.909` out, not `4.00` (a shorter lever than a corner's: the flank point
/// is `length/8` ahead of centre). Asserted to `1e-2` on a value derived from the constants, so
/// a regression to `4.0` (dropping the angular term) fails.
#[test]
fn a_ship_driven_into_a_wall_is_pushed_out_and_bounces_back() {
    // Half-width is 0.75 (`<Misc width>` 2.0, scaled by
    // TARGET_GLOBAL_SCALE), so a centre at 1.0 leaves the hull 0.15 past
    // x = 1.6.
    let mut state = ship_at(1.0, 10.0);
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );

    assert!(!response.hull_degenerate);
    assert_eq!(response.contacts, 1, "{response:?}");
    let contact = response.resolved.expect("a wall contact");
    assert_eq!(contact.surface, Surface::Wall);
    assert!((contact.depth - 0.15).abs() < 1e-4, "{contact:?}");
    // Normal points back at the ship, i.e. along -x.
    assert!(contact.normal.x < -0.9, "{contact:?}");

    assert!(
        (state.body.position.x - 0.85).abs() < 1e-4,
        "{:?}",
        state.body
    );
    // The velocity is purely along the normal, so friction has nothing to act
    // on and only the normal impulse shows.
    assert!(
        (state.body.linear_velocity.x + 3.909).abs() < 1e-2,
        "{:?}",
        state.body.linear_velocity
    );
    assert!((response.friction - 0.035).abs() < 1e-6);
}

/// [`WallResponse::impact`]/[`WallResponse::impact_speed`] read the inbound contact the sibling
/// above resolves: `vx = 10.0` into a wall with normal `-x` gives `normal_speed = -10.0`, so
/// `impact_speed` reports `10.0`.
#[test]
fn an_inbound_contact_reports_impact_and_its_speed() {
    let mut state = ship_at(1.0, 10.0);
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );
    assert!(response.impact);
    assert!((response.impact_speed - 10.0).abs() < 1e-2, "{response:?}");
}

/// The two-sided companion to [`an_inbound_contact_reports_impact_and_its_speed`]: a hull moving
/// *out* of an overlap is pulled back
/// ([`a_ship_leaving_a_wall_it_still_overlaps_is_pulled_back`]) but is not an impact, so a spark
/// consumer must not fire on it.
#[test]
fn an_outbound_contact_reports_no_impact() {
    let mut state = ship_at(1.0, -10.0);
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );
    assert!(!response.impact, "{response:?}");
    assert_eq!(response.impact_speed, 0.0);
}

/// **The measurement this module exists to reproduce.**
///
/// A craft in sustained wall contact loses a flat `3.5 %` of its speed every frame,
/// `(0.05 + 0.02) / 2`, the combined friction, and the loss is *multiplicative* (it scales with
/// speed), which is why `docs/physics/force-balance-ground-truth.md` mistook it for a drag linear
/// in `fs` at coefficient `2.28`.
///
/// The exact law, with the normal velocity held at zero.
/// `data/traces/talons-junction-standing-start.csv` gives the bound: from tick 66 the per-frame
/// loss runs `5.21 %`, `3.92 %`, `3.78 %`, `3.63 %`, `3.577 %`, `3.560 %`, monotone and **never
/// below `3.5 %` on any of its 230 contact ticks** (moving speed out of the tangent into the
/// normal only adds loss), so a friction of `0.036` would already have falsified the asymptote.
///
/// # Why the wall is narrow
///
/// The `3.5 %` is the loss for **one** contact, and the capture is a scrape with one flank behind
/// the plane. A full-width plane catches all five right-hand points and five applications cost
/// `1 - 0.965^5 = 16.3 %`, which the capture forbids. [`narrow_wall`] reproduces the capture's
/// geometry; the five-at-once case is pinned on its own below.
#[test]
fn a_pure_scrape_costs_exactly_the_contact_friction_per_frame() {
    for speed in [40.0f32, 10.0] {
        // A fresh state per speed: a scrape off a point not on the centre line also imparts a
        // small yaw, and carrying it forward would measure a *rotating* craft, not the friction
        // law. And it is a scale, not a decrement: a slower craft loses proportionally less,
        // which the trace's flat ratio proves.
        let mut state = ship_at(1.0, 0.0);
        // Moving along the wall's plane (+z) rather than into it (+x), so
        // `dot(v, n)` is zero and only the tangential term acts.
        state.body.linear_velocity = Vec3::new(0.0, 0.0, speed);

        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
        );

        assert_eq!(response.contacts, 1, "{response:?}");
        let expected = speed * (1.0 - 0.035);
        assert!(
            (state.body.linear_velocity.z - expected).abs() < 1e-4,
            "{:?} is not {expected}",
            state.body.linear_velocity
        );
    }
}

/// The multi-contact case, which the single-deepest-probe model could not produce: a hull driven
/// flat into a wall puts all five right-hand sample points (four corners and the right flank)
/// behind one plane and `Body_StepWorld` resolves every contact, so the tangential loss compounds,
/// `0.965^5`, and so does the push-out.
///
/// **The compounding push-out is the original's**: `Body_ResolveContact` ends with
/// `Body_Translate(body, n * depth)` on every contact with no shared-plane test, so a flat impact
/// is ejected several depths. Recorded, not corrected (an invention), and the geometry is a crash,
/// not a racing line.
#[test]
fn every_penetrating_sample_point_is_resolved_not_just_the_deepest() {
    let mut state = ship_at(1.0, 0.0);
    state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);

    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &wall(1.6, Surface::Wall),
    );

    assert_eq!(response.contacts, 5, "{response:?}");
    assert_eq!(response.resolved_count, 5, "{response:?}");
    // `0.965^5` to within a hundredth. Not exact, and the residual is real: the first contact
    // leaves a little angular velocity, so the next four measure the tangential velocity *of
    // their own contact point*, not of the centre of mass.
    let expected = 40.0 * (1.0f32 - 0.035).powi(5);
    assert!(
        (state.body.linear_velocity.z - expected).abs() < 1e-2,
        "{:?} is not {expected}",
        state.body.linear_velocity
    );
    // Five push-outs of 0.15 each (half-width 0.75, wall at 1.6), all
    // along -x.
    assert!(
        (response.escape.x + 0.75).abs() < 1e-3,
        "{:?}",
        response.escape
    );
}

/// The angular half of the response: a contact away from the centre of mass turns the ship. The
/// right flank point is `length/8` **ahead** of the centre, so a wall on the right pushing along
/// `-x` swings the nose left (a positive rotation about `up` in this crate's convention).
///
/// The magnitude is pinned too, as [`ANGULAR_IMPULSE_SCALE`] lives here:
///
/// ```text
/// p        = (-13.909, 0, 0)             the impulse from the test above
/// r x p    = (0, 5.216, 0)
/// I^-1     = 1/21.6 on up
/// omega   += 0.1 * 5.216 / 21.6 = 0.02415
/// ```
///
/// Without the `0.1` it would be `0.2415`, the difference between a craft that scrapes along a
/// wall and one that spins out on touching it.
#[test]
fn a_contact_off_the_centre_of_mass_yaws_the_ship() {
    let mut state = ship_at(1.0, 10.0);
    assert_eq!(state.body.angular_velocity, Vec3::ZERO);

    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );

    assert_eq!(response.contacts, 1, "{response:?}");
    let omega = state.body.angular_velocity;
    assert!((omega.y - 0.02415).abs() < 2e-3, "{omega:?}");
    assert!(omega.x.abs() < 1e-6 && omega.z.abs() < 1e-6, "{omega:?}");
    assert_eq!(response.angular_velocity_delta, omega);
}

/// The point velocity is the plain textbook `v + omega x r`, and this test stops it being
/// "corrected" into `v + cross(r, R^T omega)`.
///
/// Every other test here starts at `omega == 0`, where no convention is exercised; that gap cost
/// a day. `Body_ResolveContact` rotates `body+0x150` through the basis rows (`vtfm4.q C000,E100,
/// C200`, `0x0884ea58`) and crosses with the lever arm on the **left** (`vcrsp.t`, `0x0884ea9c`),
/// which reads as non-textbook and was implemented as such. It is not: `body+0x150` holds the
/// rotation rate **negated and in body coordinates** (`scripts/omega-column-reading-fit.py`: 2 %
/// residual against 200 % for a world-space reading), so `R^T(body+0x150) == -omega` and the
/// expression is exactly `v + omega x r` ([`Body::velocity_at`],
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`).
///
/// Same geometry as the tests above with a 1 rad/s yaw:
///
/// ```text
/// r        = (0.75, 0, -0.375)         the right flank point
/// n        = (-1, 0, 0)
/// v_p      = (10, 0, 0) + cross((0, 1, 0), r) = (9.625, 0, -0.75)
/// vn       = -9.625
/// j        = 1.4 * 9.625 / 1.006510 = 13.3879
/// v_t      = (0, 0, -0.75)
/// p        = (-13.3879, 0, 0.02625)
/// ```
///
/// A world-space reading gives `vn = -10.375`, `j = 14.431` and a `z` term of the opposite sign, so
/// both assertions fail under it (hence pinning `z`), and it costs `07_Track` its clean lap in
/// `race_ground_truth`.
#[test]
fn the_point_velocity_of_a_yawing_craft_is_textbook_and_that_is_the_originals() {
    let mut state = ship_at(1.0, 10.0);
    state.body.angular_velocity = Vec3::new(0.0, 1.0, 0.0);

    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );

    assert_eq!(response.contacts, 1, "{response:?}");
    let velocity = state.body.linear_velocity;
    assert!(
        (velocity.x - -3.3878).abs() < 5e-3,
        "reading +0x150 as world-space would give -4.4310: {velocity:?}"
    );
    assert!(
        velocity.z > 0.0 && (velocity.z - 0.02625).abs() < 5e-4,
        "the tangential term's sign is the other half of the read: {velocity:?}"
    );
    // The yaw feeds back into itself through the lever arm, so the craft leaves
    // the contact turning slightly faster than it arrived.
    assert!(
        (state.body.angular_velocity.y - 1.023151).abs() < 2e-3,
        "{:?}",
        state.body.angular_velocity
    );
}

/// A sample point more than [`MAX_CONTACT_DEPTH`] behind a surface produces **no contact**:
/// `Collision_AddContact`'s `d <= -2.0` reject. Same geometry with the wall moved so the flank
/// point is `2.4` units behind it instead of `0.4`; the swept pass that catches this in a real
/// frame is given nothing (`previous_position` equals the current one) so the gate is what shows.
#[test]
fn a_sample_point_too_far_behind_a_surface_makes_no_contact() {
    let mut state = ship_at(1.0, 10.0);
    let before = state.body;
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        // The right flank point is at x = 1.75; a plane at x = -0.4 leaves
        // it 2.15 behind, still past MAX_CONTACT_DEPTH.
        &narrow_wall(-0.4, Surface::Wall),
    );
    assert_eq!(response.contacts, 0, "{response:?}");
    assert_eq!(state.body, before);
}

/// The ten sample points are the hull box's corners plus the two flank points, in the original's
/// own order. Pinned because the *order* is the contact resolution order and the **flank pair is
/// easy to get wrong**: the `0.25` multiplies the forward axis, not up, so the pair is
/// left/right slightly ahead of centre, not fore/aft slightly above it.
#[test]
fn the_ten_sample_points_are_the_hull_corners_plus_two_flank_points() {
    let body = Body {
        position: Vec3::ZERO,
        ..Body::default()
    };
    let points = hull_sample_points(&body, &handling());

    // width 2, height 1, length 4, scaled by TARGET_GLOBAL_SCALE (0.75)
    // before halving -> half extents (0.75, 0.375, 1.5), and forward is -Z.
    let expected = [
        Vec3::new(0.75, -0.375, -1.5),
        Vec3::new(0.75, -0.375, 1.5),
        Vec3::new(-0.75, -0.375, -1.5),
        Vec3::new(-0.75, -0.375, 1.5),
        Vec3::new(0.75, 0.375, -1.5),
        Vec3::new(0.75, 0.375, 1.5),
        Vec3::new(-0.75, 0.375, -1.5),
        Vec3::new(-0.75, 0.375, 1.5),
        // length/8 ahead of centre, a half-width out to either side.
        Vec3::new(0.75, 0.0, -0.375),
        Vec3::new(-0.75, 0.0, -0.375),
    ];
    for (got, want) in points.iter().zip(expected.iter()) {
        assert!((*got - *want).length() < 1e-6, "{got:?} is not {want:?}");
    }
    assert_eq!(points.len(), HULL_PROBES);
}

/// The tunnelling case the hull probes cannot see: at 300 u/s a ship half a unit short of a
/// zero-thickness wall shell travels five units in one frame and would end 3.5 past it, beyond any
/// probe's reach. [`resolve`] alone would miss it (no probe from the far side reaches a
/// single-sided shell), so this goes through the whole step, where `Body_StepWorld`'s pass 1
/// ([`pre_integration_clip`], `0x0884f70c`) clips the body back along its velocity first.
#[test]
fn a_ship_fast_enough_to_cross_the_wall_within_one_frame_is_clipped_before_it() {
    let mut state = ship_at(3.5, 300.0);
    crate::integrate::step(
        &mut state,
        &crate::ShipControls::default(),
        &handling(),
        &Environment::default(),
        &wall(5.0, Surface::Wall),
        1.0 / 60.0,
    );
    assert!(
        state.body.position.x < 5.0,
        "the ship tunnelled through the wall: {:?}",
        state.body
    );
    assert!(
        state.body.linear_velocity.x < 300.0,
        "the wall took nothing off the ship: {:?}",
        state.body
    );
}

/// A floor standing in front of a wall used to hide the wall. [`Raycaster::raycast`] returns the
/// nearest hit of *any* surface, so the old probe took the floor, found the surface filter false
/// and gave up although the wall two tenths further on was penetrating (the nearest-hit reading
/// would give zero contacts, silently). `Collision_BoxAgainstMesh` tests every triangle on its
/// own, so an ignored surface produces no contact rather than suppressing one.
#[test]
fn a_floor_in_front_of_a_wall_no_longer_hides_it() {
    let mut world = CollisionWorld::new();
    push_quad(&mut world, 1.4, Surface::Floor, 0);
    push_quad(&mut world, 1.6, Surface::Wall, 1);

    let mut state = ship_at(1.0, 40.0);
    let response = resolve(&mut state, &handling(), &Environment::default(), &world);

    // Five against the floor and five against the wall: the floor now makes
    // its own contacts too, and still does not hide the wall.
    assert_eq!(response.contacts, 10, "{response:?}");
    assert_eq!(response.floor_contacts, 5, "{response:?}");
    assert!(
        response
            .resolved
            .is_some_and(|c| c.surface == Surface::Wall),
        "the wall behind the floor is what should have responded: {response:?}"
    );
    assert!(state.body.linear_velocity.x < 40.0, "{:?}", state.body);
}

/// One sample point behind two surfaces makes two contacts and is scrubbed twice.
/// `Collision_BoxAgainstMesh` de-duplicates nothing (see
/// `docs/ghidra/functions/psp-pulse-usa/collision.md#how-many-contacts-a-craft-vs-track-frame-makes`,
/// which works the arithmetic the other way round to conclude the recorded scrape was *one*
/// contact a frame); a nearest-hit query could not express this. Two walls a tenth apart, not two
/// coplanar ones, so the fixture does not depend on a tie-break.
#[test]
fn a_sample_point_behind_two_walls_is_scrubbed_twice() {
    let mut world = CollisionWorld::new();
    push_quad(&mut world, 1.5, Surface::Wall, 0);
    push_quad(&mut world, 1.6, Surface::Wall, 1);

    let mut state = ship_at(1.0, 0.0);
    state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);

    let response = resolve(&mut state, &handling(), &Environment::default(), &world);

    // Five right-hand sample points, each behind both planes.
    assert_eq!(response.contacts, 10, "{response:?}");
    assert_eq!(response.resolved_count, 10, "{response:?}");
    assert_eq!(response.dropped_contacts, 0, "{response:?}");
    // Ten scrubs rather than five: the tangential loss compounds per
    // contact, exactly as the five-contact case compounds per sample point.
    let five = 40.0 * (1.0f32 - 0.035).powi(5);
    assert!(
        state.body.linear_velocity.z < five,
        "{:?} should have lost more than the five-contact case's {five}",
        state.body.linear_velocity
    );
}

/// A wall wound away from the ship is a back face and produces nothing.
/// `Collision_BoxAgainstMesh` (`0x08815cd4`) skips a sample point unless `dot(boxCentre - s, n) >
/// 0` with no flip anywhere (this module once flipped, making every wall two-sided). The pair
/// makes the test worth having: the same geometry indexed the other way *must* respond, or the
/// first half would pass because nothing was in reach.
#[test]
fn a_wall_wound_away_from_the_ship_is_a_back_face() {
    let front_facing = |wound_toward: bool| {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [1.6, -100.0, -100.0],
                [1.6, 100.0, -100.0],
                [1.6, 100.0, 100.0],
                [1.6, -100.0, 100.0],
            ],
            if wound_toward {
                vec![[0, 2, 1], [0, 3, 2]]
            } else {
                vec![[0, 1, 2], [0, 2, 3]]
            },
            Vec::new(),
            Surface::Wall,
            0,
        ));

        let mut state = ship_at(1.0, 0.0);
        state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);
        let response = resolve(&mut state, &handling(), &Environment::default(), &world);
        (response.contacts, state.body.linear_velocity.z)
    };

    let (facing_contacts, facing_speed) = front_facing(true);
    assert_eq!(facing_contacts, 5);
    assert!(facing_speed < 40.0);

    let (back_contacts, back_speed) = front_facing(false);
    assert_eq!(
        back_contacts, 0,
        "a back face must not produce a contact at all"
    );
    assert_eq!(back_speed, 40.0, "and must not touch the body");
}

/// A floor inside the hull pushes it out, and drives nothing else. `Collision_BoxAgainstMesh`
/// reads no surface type, so the original's hull contacts `Floor` and `MagFloor` as `Wall`, one
/// half of how it recovers a craft sunk into the floor. A floor does **not** damage:
/// `FUN_088418e0` charges a ring record only when its friction is positive (`0x08842648`) and a
/// floor's is the `-1.0` sentinel. The swept guard stays wall-only ([`responds`]).
#[test]
fn a_floor_inside_the_hull_pushes_it_out_and_drives_no_reaction() {
    for surface in [Surface::Floor, Surface::MagFloor] {
        let mut state = ship_at(1.0, 10.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, surface),
        );
        assert_eq!(response.contacts, 5, "{surface:?}: {response:?}");
        assert_eq!(response.floor_contacts, 5, "{surface:?}");
        assert!(state.body.position.x < 1.0, "{surface:?}: {:?}", state.body);
        assert!(state.body.linear_velocity.x < 10.0, "{surface:?}");
        assert_eq!(response.impulse_sum, 0.0, "{surface:?}: no damage");
        assert!(!response.impact, "{surface:?}: no impact edge");
        assert_eq!(response.resolved, None, "{surface:?}");
        assert!(!state.wall_contact_prev, "{surface:?}");
        assert!(!responds(surface));
    }
}

/// **No track contact arms the collision stun**, however hard or sustained
/// ([`STUN_PER_CONTACT`] for the evidence: two captures totalling 3,446 ticks, 380 in wall
/// contact, read `stun_timer == 0.0` on every one).
///
/// Replaces three tests that asserted the opposite (`an_impact_arms_the_collision_stun`,
/// `repeated_impacts_accumulate_stun`, `a_sustained_scrape_arms_the_stun_once`). Their subject
/// (`craft+0x290 += 0.5` per posted impulse, not per frame) belongs to
/// `Ship_ApplyCollisionImpulse`; this guards the same property from the other side: **the engine
/// must never be cut by driving along a wall**.
#[test]
fn no_amount_of_track_contact_arms_the_collision_stun() {
    // A square-on impact at speed, which is the hardest hit this geometry
    // allows and which the deleted tests used as their arming case.
    let mut state = ship_at(1.0, 10.0);
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &wall(1.6, Surface::Wall),
    );
    assert!(response.resolved.is_some());
    assert_ne!(response.velocity_delta, Vec3::ZERO);
    assert_eq!(state.stun_timer, 0.0);

    // Sixty frames of it, re-seeded each time so the hull always moves into the surface. The
    // pathology is cumulative: `0.5 s` armed against a `dt` decay is 30:1 at 60 Hz, so one arming
    // per second of contact buys half a minute of dead engine.
    for _ in 0..60 {
        state.body.position = Vec3::new(1.0, 0.0, 0.0);
        state.body.linear_velocity = Vec3::new(10.0, 0.0, 0.0);
        state.body.angular_velocity = Vec3::ZERO;
        resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
        );
    }

    assert_eq!(
        state.stun_timer, 0.0,
        "driving along a wall must never cut the engine"
    );
}

/// A ship sliding along a wall is pushed out every frame without ever moving
/// into it. Stunning on that would cut the engine for the whole length of the
/// wall, which is the difference between scraping and stopping dead.
#[test]
fn a_scrape_along_a_wall_does_not_arm_the_stun() {
    let mut state = ship_at(1.0, -10.0);
    resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        // One contact: five two-sided contacts on one plane reverse the outgoing velocity
        // part-way through the frame and the later ones read as impacts (the original's
        // compounding, pinned elsewhere). Here the question is only whether *leaving* counts.
        &narrow_wall(1.6, Surface::Wall),
    );
    assert_eq!(state.stun_timer, 0.0);
}

/// The contact is **two-sided**: a hull still overlapping the wall but already moving out is
/// pulled back. A deliberate reversal of what this module once did, and what the original does:
/// `Body_ResolveContact` (`0x0884e968`) has no `dot(v, n) < 0` test, only that a contact exists
/// (`depth > 0.0` here), and `-(1 + 0.4) * vn` with `vn` positive is negative, so `10` out becomes
/// `4` back in.
///
/// It makes a wall sticky, which is recorded: in `talons-junction-standing-start.csv`
/// `dot(v, right)` is knocked to `-11.5` at tick 66 and is still `-2.2` at tick 295, never
/// returning to zero across 230 ticks. A one-sided push cannot do that.
#[test]
fn a_ship_leaving_a_wall_it_still_overlaps_is_pulled_back() {
    let mut state = ship_at(1.0, -10.0);
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &narrow_wall(1.6, Surface::Wall),
    );
    assert!(response.resolved.is_some());
    // `3.909`, not `4.00`, for the angular-denominator reason spelled out on
    // `a_ship_driven_into_a_wall_is_pushed_out_and_bounces_back`. What
    // matters here is the *sign*: `10` out becomes `3.909` back in.
    assert!(
        (state.body.linear_velocity.x - 3.909).abs() < 1e-2,
        "{:?}",
        state.body.linear_velocity
    );
    // The position correction still applies, and it is what clears the
    // overlap so the next frame sees no contact at all.
    assert!(
        (state.body.position.x - 0.85).abs() < 1e-4,
        "{:?}",
        state.body
    );
    // But it is not an impact, so no stun.
    assert_eq!(state.stun_timer, 0.0);
}

/// Zero hull dimensions must be a visible no-op rather than a quiet one: it
/// is the one regression that would turn this whole module off without any
/// symptom other than walls not working.
#[test]
fn a_degenerate_hull_reports_itself_instead_of_failing_silently() {
    let mut state = ship_at(1.0, 10.0);
    let before = state.body;
    let response = resolve(
        &mut state,
        &Handling::ZERO,
        &Environment::default(),
        &wall(1.6, Surface::Wall),
    );
    assert!(response.hull_degenerate);
    assert_eq!(response.resolved, None);
    assert_eq!(state.body, before);
}

/// Nothing to hit is not a contact, and must not move the ship.
#[test]
fn open_space_is_left_alone() {
    let mut state = ship_at(0.0, 10.0);
    let before = state.body;
    let response = resolve(
        &mut state,
        &handling(),
        &Environment::default(),
        &CollisionWorld::new(),
    );
    assert_eq!(response, WallResponse::default());
    assert_eq!(state.body, before);
}

/// The box support function has to agree with the axis half-extents, since
/// the probes rely on it degenerating to exactly those.
#[test]
fn the_hull_extent_matches_the_half_extents_along_the_body_axes() {
    let body = Body::default();
    let h = handling();
    // Scaled by TARGET_GLOBAL_SCALE (0.75) before halving: see
    // `hull_sample_points`.
    assert!((hull_extent(&body, &h.dimensions, body.right()) - 0.75).abs() < 1e-6);
    assert!((hull_extent(&body, &h.dimensions, body.up()) - 0.375).abs() < 1e-6);
    assert!((hull_extent(&body, &h.dimensions, body.forward()) - 1.5).abs() < 1e-6);
}

/// The `-1.0` sentinel is not a coefficient. Unreachable in a race today (only walls respond);
/// pinned so whoever makes floors contactable inherits the right rule, not a negative friction
/// that *adds* tangential velocity.
#[test]
fn the_frictionless_sentinel_combines_to_zero_and_never_to_a_negative() {
    assert_eq!(Surface::Wall.friction(), Some(0.05));
    assert_eq!(Surface::Floor.friction(), None);
    assert_eq!(Surface::MagFloor.friction(), None);
    assert_eq!(Surface::Reset.friction(), None);

    assert_eq!(combine_friction(Some(0.05), Some(0.05)), 0.05);
    assert_eq!(combine_friction(Some(0.05), None), 0.0);
    assert_eq!(combine_friction(None, Some(0.05)), 0.0);
    assert_eq!(combine_friction(None, None), 0.0);
    // The one that a race actually uses: the wall's 0.05 against the ship
    // collider's own 0.02.
    assert_eq!(SHIP_FRICTION, Some(0.02));
    assert_eq!(
        combine_friction(SHIP_FRICTION, Surface::Wall.friction()),
        0.035
    );
}

/// The same law at an **angle**, which the pure scrape above cannot see. That test holds `v_n` at
/// zero; this is a craft at 100 units/s meeting the wall at **28 degrees** (the heading error of
/// the Outpost 7 investigation, `HANDOVER.md`), separating the two halves of the response:
///
/// | tick | `\|v\|` | total loss | `v_n` | tangential loss |
/// | ---: | ---: | ---: | ---: | ---: |
/// | 0 | 100.00 -> 87.16 | 12.84 % | 46.95 -> -18.35 | **3.5000 %** |
/// | 1 | 87.16 -> 82.53 | 5.31 % | -18.35 -> 7.18 | **3.5000 %** |
/// | 2 | 82.53 -> 79.39 | 3.81 % | 7.18 -> -2.81 | **3.5000 %** |
/// | 6+ | | 3.5000 % | ~0 | **3.5000 %** |
///
/// The tangential factor is `0.965` on **every** tick regardless of angle, and the *total* loss
/// converges **down** onto `3.5 %` as the normal component is spent, approaching the friction
/// floor from above and never crossing it. That is the shape `contact-response.md` records
/// (`5.21 %`, `4.03 %`, ... `3.560 %`, minimum `3.534 %` at 104 units/s), one-sided rather than a
/// fit: a friction of `0.036` is falsified by the tail and `0.030` by the floor.
///
/// The position is reset each tick so one contact fires, and the spin with it (an off-centre
/// scrape imparts yaw that would make the next tick measure a rotating hull).
#[test]
fn a_28_degree_graze_bleeds_the_recovered_friction_and_nothing_else() {
    let radians = 28.0f32.to_radians();
    let mut state = ship_at(1.0, 0.0);
    state.body.linear_velocity = Vec3::new(radians.sin() * 100.0, 0.0, radians.cos() * 100.0);

    let mut settled = 0.0f32;
    for tick in 0..20 {
        state.body.position = Vec3::new(1.0, 0.0, 0.0);
        state.body.angular_velocity = Vec3::ZERO;
        state.body.orientation = Quat::IDENTITY;

        let before = state.body.linear_velocity;
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
        );
        let after = state.body.linear_velocity;

        assert_eq!(response.contacts, 1, "tick {tick}: {response:?}");
        assert!((response.friction - 0.035).abs() < 1e-6, "tick {tick}");

        // The tangential axis is `+z` here, and it loses exactly `0.035` of
        // itself every tick the contact exists - independent of how much of
        // the motion is in the normal.
        let expected = before.z * (1.0 - 0.035);
        assert!(
            (after.z - expected).abs() < 1e-3,
            "tick {tick}: {after:?} is not {expected}"
        );

        settled = 1.0 - after.length() / before.length();
        // Never below the floor, on any tick. This is the whole one-sided
        // test, and it is what a friction of `0.030` would fail.
        assert!(settled > 0.035 - 1e-5, "tick {tick}: {settled}");
    }

    // And it has converged onto the floor rather than sitting above it, which
    // is what a friction of `0.036` would fail.
    assert!((settled - 0.035).abs() < 1e-5, "{settled}");
}

/// **What stops a craft that leans on a wall: the bounce, not the friction.**
///
/// The graze above spends its normal velocity in three ticks and settles on the friction floor. A
/// craft whose *heading* is 28 degrees off the wall does not: grip pulls the velocity back onto
/// the hull's forward every tick, restoring `v_n` as fast as the contact kills it. **The
/// re-pointing here is a stand-in for that, not the grip law** (`crate::forces` has the real
/// term): it resets the velocity's direction to 28 degrees each tick, magnitude preserved, to
/// isolate what [`resolve`] costs *given* a restored `v_n`.
///
/// One contact a tick and no thrust:
///
/// | angle | per-tick loss | 100 units/s after 19 ticks |
/// | ---: | ---: | ---: |
/// | 10 deg | 4.72 % | 39.87 |
/// | 20 deg | 8.34 % | 19.12 |
/// | **28 deg** | **12.84 %** | **7.34** |
///
/// At 28 degrees the split is `2.72 %` from friction and `9.82 %` from the normal impulse:
/// **the bounce is 73 % of the cost, the friction 27 %**. The `9.82` is
/// `-(1 + BODY_RESTITUTION) * v_n / D` with `D` near `1` (measured `v_n` factor `-0.391`, i.e.
/// `-e`). So the wall response does stop a craft dead, the recovered law does it, and the
/// suspected `0.035` is the smaller half.
#[test]
fn a_held_heading_makes_the_bounce_the_cost_not_the_friction() {
    let radians = 28.0f32.to_radians();
    let mut state = ship_at(1.0, 0.0);
    let mut speed = 100.0f32;

    for tick in 0..19 {
        state.body.position = Vec3::new(1.0, 0.0, 0.0);
        state.body.angular_velocity = Vec3::ZERO;
        state.body.orientation = Quat::IDENTITY;
        state.body.linear_velocity = Vec3::new(radians.sin() * speed, 0.0, radians.cos() * speed);

        let before = speed;
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
        );
        speed = state.body.linear_velocity.length();

        assert_eq!(response.contacts, 1, "tick {tick}: {response:?}");
        // Flat, because both halves of the response scale with the speed.
        let loss = 1.0 - speed / before;
        assert!((loss - 0.128_412).abs() < 1e-4, "tick {tick}: {loss}");
    }

    // 100 units/s to a walking pace in nineteen ticks, with one contact each -
    // and only a quarter of that is the friction coefficient.
    assert!((speed - 7.3437).abs() < 1e-3, "{speed}");
}
