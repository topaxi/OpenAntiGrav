//! Craft against craft: the two-body contact response.
//!
//! `Body_ResolveContactPair` (`0x0884ef30`) reimplemented, at confidence **95**
//! for the response (caught live on six contacts 2026-09-10, `j` reproduced
//! from the captured inputs, PS2 twin agreeing - see [`respond`]) and **92**
//! for the shape (`Collision_BoxAgainstBox`, `0x0881702c`, live-verified
//! 2026-09-07 to run and produce contacts for two craft) - see [`overlap`].
//! The recovery is on `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
//!
//! # A craft bounces off another craft differently from how it bounces off a wall
//!
//! The one-body path in [`crate::wall`] takes its restitution from the body -
//! `0.4` on a ship - and folds friction into the same impulse. The pair resolver
//! does neither: restitution is a **code literal** giving `e = 0.1`, and there is
//! no tangential term at all. Both are what the original does, in the same
//! build, and the split is deliberate rather than an oversight here.
//!
//! It also has a separating test the one-body path does not: a pair already
//! moving apart faster than [`SEPARATING_LIMIT`] is left alone.

use oag_core::math::Vec3;

use crate::params::Dimensions;
use crate::ship::{Body, ShipState};

/// `1 + e` from the resolver's hardcoded `-1.1` numerator, so `e = 0.1`.
///
/// **Not the same as a craft against the track**, which uses the per-body `0.4`
/// the ship constructor writes. Confidence 80: a literal in the decompiled
/// arithmetic, and the PS2's own pair resolver carries the same one.
pub const PAIR_RESTITUTION_PLUS_ONE: f32 = 1.1;

/// Relative normal speed above which a pair is treated as already separating.
///
/// The resolver's `((vA . n) - (vB . n)) - 0.5 <= 0` gate, a code literal.
pub const SEPARATING_LIMIT: f32 = 0.5;

/// Fraction of the overlap each body is pushed along the normal.
///
/// A quarter each way, the same for both bodies whatever their masses - which is
/// what the original does, and is why it is a constant here rather than a mass
/// split.
pub const POSITIONAL_SPLIT: f32 = 0.25;

/// A resolved contact between two craft.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PairContact {
    /// The normal, pointing from B toward A.
    pub normal: Vec3,
    /// How far the two hulls interpenetrate.
    pub depth: f32,
    /// The magnitude of the impulse that was applied along the normal.
    pub impulse: f32,
}

/// Whether two craft are touching, and where.
///
/// # Corrected 2026-09-07: live-verified, and now read in comparison detail
///
/// `Collision_DispatchPair` (`0x08816eac`) sends a box proxy against a box
/// proxy to `Collision_BoxAgainstBox` (`0x0881702c`), not `0x08815ccc` as an
/// earlier note said - that was a mislabelled shape-kind reading, corrected in
/// `docs/ghidra/functions/psp-pulse-usa/collision.md`. `0x08815ccc` really is a
/// two-instruction stub (`jr ra; nop`), but it is the *mesh*-against-mesh
/// dispatch; craft never reach it. **Both gates that stood between
/// `Collision_BoxAgainstBox` existing and it ever running for two craft are now
/// confirmed live**, not just plausible from static reading: a PPSSPP session
/// against a full eight-craft grid read `world+0x5464 == 1` and every craft
/// collider's own `+0x68` byte `== 1` during a race, then forced two craft to
/// overlap and caught `Collision_BoxAgainstBox` itself firing on their exact
/// proxy pair (owners matching the two placed craft) ten times running, with
/// the world's own contact counter (`world+0x2450`) rising by one across the
/// first of those hits. See
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
///
/// So this is a real, running narrowphase, not a stand-in with nothing to
/// approximate - and `Collision_BoxAgainstBox` has now been decompiled and
/// compared against this function in the detail `contact-response.md` asked
/// for. It differs in three ways this port now follows:
///
/// - **The nine edge-edge axes never choose the normal.** The original tests
///   all fifteen axes as separating candidates (reject if any one clears), but
///   only the **six face axes** - three per box - ever update the running
///   "best" depth and normal. An edge-edge cross product can throw the pair
///   out as not touching; it can never become the contact normal. This
///   function used to let all fifteen compete on equal footing. The nine edge
///   axes are also tested **unnormalized** (`vcrsp.t` with no `vsqrt`/`vrcp`
///   before the `vdot.t`), so their "depth" is never in the same units as a
///   face axis's - positive evidence the original *could not* have mixed them
///   into one best-depth comparison, not merely an absence of writes.
/// - **Each box's own "up" axis needs to beat *half* the reigning best depth
///   to become the normal; right and forward only need to beat it outright.**
///   Confirmed at instruction level, not just in the decompiler's summary
///   (see the VFPU-inlining trap on `docs/ghidra/workflow.md`): `A.right`
///   seeds the running best unconditionally, then exactly two of the
///   remaining five face-axis comparisons - `A.up` and `B.up` - are preceded
///   by a real `mul.s` against the same `0.5f` register (`f13`, loaded once
///   at function entry and never reloaded) the function already uses to turn
///   each collider's `<Misc>` dimensions into half-extents; the other three
///   (`A.forward`, `B.right`, `B.forward`) compare against the running best
///   directly, with no such multiply anywhere near them. So the bias is a
///   real scalar op on a real constant, not a mis-attributed prefix or a
///   decompiler artifact. The effect is a standing bias against picking
///   either hull's vertical axis as the push-apart direction unless it is
///   decisively the shallower one - read as tuned against craft popping
///   vertically off a graze that should read as a sideways scrape. This
///   function's `axes()` returns `[right, up, forward]` in exactly this
///   order, so the bias applies to index `1` on each side.
/// - **The contact point is the plain midpoint of the two box centres**,
///   `(a.position + b.position) * 0.5` - not a support point on the chosen
///   axis at all. The original's `collider+0xb0` field the two box centres are
///   read from is the body's own position (`Body_SyncBoxCollider` copies it
///   there every sync), so this reduces to the two bodies' positions
///   unconditionally, independent of which axis won. The lever arm from each
///   body to this midpoint reaches the *denominator* of [`respond`] and
///   nothing else: the impulse itself is applied at each body's own centre,
///   so no contact point of any construction spins a craft here.
///
/// Friction is unread here because `Body_ResolveContactPair` (the response,
/// not this narrowphase) never applies any - see `contact-response.md`'s
/// own account of that function, already ported in [`resolve`].
///
/// It replaced a sphere of half the hull's diagonal, which was **far too big**:
/// on a 4 x 2 x 8 hull that sphere reaches 4.58 units where the flank is 2 away,
/// so craft shoved each other while visibly apart.
///
/// Returns the normal (pointing from `b` toward `a`), the contact point, and the
/// penetration depth.
#[must_use]
pub fn overlap(
    a: &Body,
    a_size: &Dimensions,
    b: &Body,
    b_size: &Dimensions,
) -> Option<(Vec3, Vec3, f32)> {
    let between = a.position - b.position;
    // A sphere reject first, so the fifteen-axis test only runs on candidates.
    // Half the diagonal bounds the box, so this never drops a real overlap.
    if between.length_squared() >= (hull_radius(a_size) + hull_radius(b_size)).powi(2) {
        return None;
    }

    let a_axes = axes(a);
    let b_axes = axes(b);
    let a_half = half_extents(a_size);
    let b_half = half_extents(b_size);

    let mut best_depth = f32::INFINITY;
    let mut best_axis = Vec3::ZERO;

    // The six face axes: `A.right, A.up, A.forward, B.right, B.up, B.forward`,
    // in exactly this order - it is what the bias below keys on. Every one of
    // the six is also a reject test: `Collision_BoxAgainstBox` returns with no
    // contact the moment any single axis clears, so this loop does the same
    // rather than collecting all six depths first.
    for (index, axis) in a_axes.into_iter().chain(b_axes).enumerate() {
        let reach = project(&a_axes, a_half, axis) + project(&b_axes, b_half, axis);
        let distance = between.dot(axis).abs();
        let depth = reach - distance;
        if depth <= 0.0 {
            return None;
        }
        // Each side's own "up" axis - index 1 of the three-axis block, so
        // index 1 or 4 overall - only replaces the running best if it clears
        // *half* of it; the other four face axes use a plain `<`. `A.right`
        // (index 0) always wins here since `best_depth` starts at infinity.
        let wins = if index == 1 || index == 4 {
            depth < best_depth * 0.5
        } else {
            depth < best_depth
        };
        if wins {
            best_depth = depth;
            // Oriented from `b` toward `a`, which is what the caller expects.
            best_axis = if between.dot(axis) < 0.0 { -axis } else { axis };
        }
    }

    // The nine edge-edge cross products: reject-only. Unlike the face axes
    // above, none of these can ever become the contact normal in the
    // original - only the SAT's separation test uses them.
    for a_axis in a_axes {
        for b_axis in b_axes {
            let axis = a_axis.cross(b_axis);
            // A near-zero cross product means the two edges are parallel and
            // the axis carries no information; the face axes already cover
            // that case.
            let length = axis.length();
            if length <= 1.0e-4 {
                continue;
            }
            let axis = axis / length;
            let reach = project(&a_axes, a_half, axis) + project(&b_axes, b_half, axis);
            let distance = between.dot(axis).abs();
            if reach - distance <= 0.0 {
                return None;
            }
        }
    }

    if !best_depth.is_finite() {
        return None;
    }
    // The plain midpoint of the two box centres, which are the two bodies'
    // own positions - not a support point on the chosen axis, and not
    // dependent on which axis won. `Collision_BoxAgainstBox` writes exactly
    // this: `(colliderA.centre + colliderB.centre) * 0.5`, and a collider's
    // centre is copied from its body's position on every sync.
    let point = (a.position + b.position) * 0.5;
    Some((best_axis, point, best_depth))
}

/// The hull's own axes in world space: right, up, forward.
fn axes(body: &Body) -> [Vec3; 3] {
    [body.right(), body.up(), body.forward()]
}

/// How much of the authored hull box a craft actually collides with.
///
/// **Ours, and a feel knob rather than a measurement.** There is no original to
/// match here - `0x08815ccc` reports nothing - so nothing sets this but play.
///
/// The reason it is below `1.0`: `<Misc width height length>` is a **bounding**
/// box, sized to the hull's widest point. Measured on a real Pulse craft it is
/// `5.5 x 3.5 x 13`, whose half-length of `6.5` lands within a whisker of the
/// `6.45` bounding radius `oag-view` reports for the shipped Feisar mesh - so the
/// box bounds the model rather than tracing it. A Wipeout hull tapers hard toward
/// the nose, so a full-size box has the craft colliding along its whole length at
/// the width of its widest point, and two craft passing bump where the models
/// visibly miss. That was reported from play twice.
///
/// Applied to every axis rather than only to width, because the taper is in
/// plan *and* in profile.
pub const HULL_SCALE: f32 = 0.75;

/// `<Misc width height length>` as half-extents on those axes, scaled by
/// [`HULL_SCALE`].
fn half_extents(size: &Dimensions) -> Vec3 {
    Vec3::new(size.width, size.height, size.length) * (0.5 * HULL_SCALE)
}

/// How far a box reaches along `axis` from its own centre.
fn project(axes: &[Vec3; 3], half: Vec3, axis: Vec3) -> f32 {
    (half.x * axes[0].dot(axis)).abs()
        + (half.y * axes[1].dot(axis)).abs()
        + (half.z * axes[2].dot(axis)).abs()
}

/// Half the hull's diagonal: the sphere the broadphase rejects with.
#[must_use]
pub fn hull_radius(size: &Dimensions) -> f32 {
    half_extents(size).length()
}

/// Resolves one craft-to-craft contact, moving both bodies.
///
/// Returns `None` when the pair is not touching, or when the gate refuses it.
///
/// The order of the two arguments does not change the outcome: the impulse is
/// equal and opposite and the positional split is symmetric, so `resolve(a, b)`
/// and `resolve(b, a)` leave the same world. That is a property worth having,
/// because a pair loop has to visit each pair in *some* order and the simulation
/// must not depend on which.
pub fn resolve(
    a: &mut ShipState,
    a_size: &Dimensions,
    b: &mut ShipState,
    b_size: &Dimensions,
) -> Option<PairContact> {
    let (normal, point, depth) = overlap(&a.body, a_size, &b.body, b_size)?;
    respond(a, b, normal, point, depth)
}

/// The response half of [`resolve`], given a contact: `Body_ResolveContactPair`
/// proper, with the narrowphase already done.
///
/// Split from [`resolve`] so the arithmetic can be pinned against a contact the
/// original was caught resolving, inputs and output both read off the running
/// game rather than constructed - see `tests.rs`.
///
/// # A pair hit never spins either craft - measured, 2026-09-10
///
/// The impulse is applied to each body **at its own position**, not at the
/// contact point. `Body_ResolveContactPair` passes `body+0x30` as the "point"
/// argument of both `Body_ApplyImpulseAtPoint` calls (`0x0884f178` loads
/// `a1 = s1 + 0x30` for the first, `0x0884efdc` sets `s3 = s0 + 0x30` for the
/// second), and the applier's lever arm is `*a1 - body+0x30`, so it is
/// exactly zero and the angular half - `omega -= 0.1 * I^-1 (r x p)` - is
/// exactly zero with it. Read at instruction level, then caught live in
/// PPSSPP on six contacts including a staged 84 units/s rear-end hit: `a1 ==
/// bodyB + 0x30` on every one, and both bodies' `+0x150` bit-identical across
/// the call. The PS2 twin (`0x0015e600`) passes `body + 0x30` the same way.
/// Confidence **95**: runtime trace, corroborated in the second binary.
///
/// This function used to apply the full angular half at the midpoint - the
/// lever arm the *denominator* uses - and at a fast, offset, first contact
/// (`vn` around `-150`) that produced `-267` to `-276` degrees per second of
/// yaw in one tick, which a maintainer reported from play as far more spin
/// than any of the originals. The original produces none.
///
/// **What the lever arm still does**: the denominator `D` carries the full
/// angular compliance about the contact point - so `j` is solved as though
/// all of the resulting spin were going to be applied, and then none of it
/// is. That is the pair-path version of the "soft in translation, stiff in
/// rotation" split `contact-response.md` records for the one-body path, and
/// [`angular_term`] keeps it.
///
/// # The point velocity the gate and `vn` are built from is not textbook
///
/// The original computes each body's velocity at the contact point as
/// `v + cross(r, R^T omega)` (`vtfm4.q` through the body's own basis rows,
/// then `vcrsp.t` with the lever arm on the *left*), where a textbook solver
/// writes `v + omega x r`. Both the operand order and the spurious rotation
/// of an already world-space `omega` are the original's, and they are what
/// [`contact_velocity`] reproduces: recomputing `j` offline from a captured
/// contact's raw inputs, this form matches the impulse the original applied
/// to five significant figures on both captures (`43.1697` and `0.18440`),
/// while `v + omega x r` gives `46.18` and `-0.085`. The second capture was
/// a craft tumbling at 84 rad/s, which is what made the three candidate
/// conventions separable at all.
pub fn respond(
    a: &mut ShipState,
    b: &mut ShipState,
    normal: Vec3,
    point: Vec3,
    depth: f32,
) -> Option<PairContact> {
    let r_a = point - a.body.position;
    let r_b = point - b.body.position;
    let velocity_a = contact_velocity(&a.body, r_a);
    let velocity_b = contact_velocity(&b.body, r_b);

    // The gate. `vn` is positive while the two are separating, so a pair already
    // coming apart faster than the limit is left alone.
    let vn = (velocity_a - velocity_b).dot(normal);
    if vn - SEPARATING_LIMIT > 0.0 {
        return None;
    }

    // `n . ((I^-1 (r x n)) x r)` for each body, which is the pair's angular
    // compliance, plus both inverse masses. The one-body resolver has one mass
    // here; having two is what makes this the pair path.
    let angular = angular_term(&a.body, r_a, normal) + angular_term(&b.body, r_b, normal);
    let denominator = angular + inverse_mass(&a.body) + inverse_mass(&b.body);
    if denominator <= f32::EPSILON {
        return None;
    }

    let impulse = -(PAIR_RESTITUTION_PLUS_ONE * vn) / denominator;
    // At each body's own centre: the linear half only. See above.
    a.body.apply_impulse(normal * impulse);
    b.body.apply_impulse(-normal * impulse);

    // The positional split: a quarter of the overlap each way, mass-independent.
    a.body.position += normal * (POSITIONAL_SPLIT * depth);
    b.body.position -= normal * (POSITIONAL_SPLIT * depth);

    Some(PairContact {
        normal,
        depth,
        impulse,
    })
}

/// `v + cross(r, R^T omega)`: the original's velocity at a lever arm `r`.
///
/// `R` is the basis-rows block at `body+0x00..0x30`, whose rows are the
/// craft's world-space axes `(up x forward, up, forward)`. In this crate's
/// frame `right = orientation * X` and `forward = orientation * -Z`, so the
/// original's row 0 is `-right` and its row 2 is `-(orientation * Z)`, and
/// `R^T omega = omega.x * row0 + omega.y * row1 + omega.z * row2` comes out as
/// `orientation * (-omega.x, omega.y, -omega.z)`. The inertia tensor and the
/// hull box are both symmetric under that half-turn about `up`, so this is
/// the only place the two frames' handedness convention shows.
///
/// Not [`Body::velocity_at`] on purpose: that is the textbook `v + omega x r`,
/// and the pair resolver measurably is not (see [`respond`]).
fn contact_velocity(body: &Body, r: Vec3) -> Vec3 {
    let omega = body.angular_velocity;
    let spun = body.orientation * Vec3::new(-omega.x, omega.y, -omega.z);
    body.linear_velocity + r.cross(spun)
}

fn inverse_mass(body: &Body) -> f32 {
    if body.mass <= f32::EPSILON {
        0.0
    } else {
        1.0 / body.mass
    }
}

/// `n . ((I^-1 (r x n)) x r)`, with the **body-space** diagonal applied to the
/// **world-space** `r x n` directly, no rotation into the body frame.
///
/// That is the literal reading of `0x0884f3a8`-`0x0884f3d0` (the `+0x40..0x70`
/// block through `vtfm4.q` on a world-space cross product), the same quirk
/// `contact-response.md` records for the one-body denominator - and on the
/// 2026-09-10 tumbling-craft capture it is what reproduces the original's `j`
/// (`0.18440` against `0.18398` for the rotated, textbook form). A quarter of
/// a percent on one number, but the wrong quarter.
fn angular_term(body: &Body, r: Vec3, normal: Vec3) -> f32 {
    let torque = r.cross(normal);
    let inertia = body.inertia;
    // Component-wise inverse of the diagonal tensor, guarding a zero axis rather
    // than dividing by it: `Handling::ZERO` really does have one.
    let angular = Vec3::new(
        safe_divide(torque.x, inertia.x),
        safe_divide(torque.y, inertia.y),
        safe_divide(torque.z, inertia.z),
    );
    angular.cross(r).dot(normal)
}

fn safe_divide(numerator: f32, denominator: f32) -> f32 {
    if denominator.abs() <= f32::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests;
