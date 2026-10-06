//! Craft against craft: the two-body contact response.
//!
//! `Body_ResolveContactPair` (`0x0884ef30`) reimplemented. Confidence **95** for the response
//! (caught live on six contacts 2026-09-10, `j` reproduced from the captured inputs, PS2 twin
//! agreeing; [`respond`]) and **92** for the shape (`Collision_BoxAgainstBox`, `0x0881702c`,
//! live-verified 2026-09-07 to run and produce contacts for two craft; [`overlap`]). Recovery:
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
//!
//! # A craft bounces off another craft differently from a wall
//!
//! The one-body path in [`crate::wall`] takes restitution from the body (`0.4`) and folds
//! friction into the impulse. The pair resolver has restitution as a **code literal**
//! (`e = 0.1`) and no tangential term at all, both as in the original's same build. It also
//! has a separating test the one-body path lacks: a pair already moving apart faster than
//! [`SEPARATING_LIMIT`] is left alone.

use oag_core::math::Vec3;

use crate::params::Dimensions;
use crate::ship::{Body, ShipState};

/// `1 + e` from the resolver's hardcoded `-1.1` numerator, so `e = 0.1`. **Not the track's**
/// per-body `0.4`. Confidence 80: a literal in the decompiled arithmetic, also in the PS2's
/// pair resolver.
pub const PAIR_RESTITUTION_PLUS_ONE: f32 = 1.1;

/// Relative normal speed above which a pair is treated as already separating: the resolver's
/// `((vA . n) - (vB . n)) - 0.5 <= 0` gate, a code literal.
pub const SEPARATING_LIMIT: f32 = 0.5;

/// Fraction of the overlap each body is pushed along the normal: a quarter each way,
/// whatever their masses, as the original does (hence a constant, not a mass split).
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
/// # Live-verified 2026-09-07
///
/// `Collision_DispatchPair` (`0x08816eac`) sends a box proxy against a box proxy to
/// `Collision_BoxAgainstBox` (`0x0881702c`), not `0x08815ccc` as an earlier note said (a
/// mislabelled shape-kind reading, corrected in `collision.md`; `0x08815ccc` is a
/// two-instruction `jr ra; nop` stub for *mesh*-against-mesh, which craft never reach). **Both
/// gates are confirmed live**: a PPSSPP session on a full eight-craft grid read
/// `world+0x5464 == 1` and every craft collider's `+0x68 == 1` during a race, then two forced
/// overlapping craft made `Collision_BoxAgainstBox` fire on their exact proxy pair ten times
/// running, the world's contact counter (`world+0x2450`) rising across the first. See
/// `contact-response.md`. It differs from the earlier port in three ways, now followed:
///
/// - **The nine edge-edge axes never choose the normal.** All fifteen axes are separating
///   candidates (reject if any clears), but only the **six face axes** update the running
///   best depth and normal. The edge axes are tested **unnormalized** (`vcrsp.t` with no
///   `vsqrt`/`vrcp`), so their "depth" is never in a face axis's units: positive evidence the
///   original could not have mixed them into one comparison.
/// - **Each box's own "up" axis must beat *half* the reigning best depth to become the normal;
///   right and forward only need to beat it outright.** Confirmed at instruction level (see
///   the VFPU-inlining trap on `docs/ghidra/workflow.md`): `A.right` seeds the best, then
///   exactly two of the remaining five comparisons, `A.up` and `B.up`, are preceded by a
///   `mul.s` against the `0.5f` register (`f13`, loaded once at entry) that also makes
///   half-extents from `<Misc>`; the other three compare directly. A standing bias against
///   the vertical axis as push-apart direction unless decisively shallower, read as tuned
///   against craft popping vertically off a graze. `axes()` returns `[right, up, forward]`,
///   so the bias applies to index `1` on each side.
/// - **The contact point is the plain midpoint of the two box centres**,
///   `(a.position + b.position) * 0.5`, not a support point on the chosen axis. The
///   `collider+0xb0` centres are the bodies' own positions (`Body_SyncBoxCollider` copies
///   them every sync). The lever arm to this midpoint reaches only the *denominator* of
///   [`respond`]; the impulse is applied at each body's own centre, so no contact point spins
///   a craft here.
///
/// Friction is unread: `Body_ResolveContactPair` (the response) applies none ([`resolve`]).
///
/// It replaced a sphere of half the hull's diagonal, **far too big**: on a 4 x 2 x 8 hull it
/// reaches 4.58 units where the flank is 2 away, so craft shoved each other while visibly apart.
///
/// Returns the normal (from `b` toward `a`), the contact point and the penetration depth.
#[must_use]
pub fn overlap(
    a: &Body,
    a_size: &Dimensions,
    b: &Body,
    b_size: &Dimensions,
) -> Option<(Vec3, Vec3, f32)> {
    let between = a.position - b.position;
    // A sphere reject first; half the diagonal bounds the box, so it never drops a real
    // overlap.
    if between.length_squared() >= (hull_radius(a_size) + hull_radius(b_size)).powi(2) {
        return None;
    }

    let a_axes = axes(a);
    let b_axes = axes(b);
    let a_half = half_extents(a_size);
    let b_half = half_extents(b_size);

    let mut best_depth = f32::INFINITY;
    let mut best_axis = Vec3::ZERO;

    // The six face axes `A.right, A.up, A.forward, B.right, B.up, B.forward`, in this order
    // (the bias below keys on it). Each is also a reject test: `Collision_BoxAgainstBox`
    // returns the moment any axis clears, so this does too.
    for (index, axis) in a_axes.into_iter().chain(b_axes).enumerate() {
        let reach = project(&a_axes, a_half, axis) + project(&b_axes, b_half, axis);
        let distance = between.dot(axis).abs();
        let depth = reach - distance;
        if depth <= 0.0 {
            return None;
        }
        // Each side's "up" (index 1 or 4) replaces the best only if it clears *half* of it;
        // the other four use a plain `<`. `A.right` (index 0) always wins, `best_depth`
        // starting at infinity.
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

    // The nine edge-edge cross products: reject-only; none can become the contact normal in
    // the original.
    for a_axis in a_axes {
        for b_axis in b_axes {
            let axis = a_axis.cross(b_axis);
            // A near-zero cross product means parallel edges and no information; the face
            // axes cover that case.
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
    // The plain midpoint of the two box centres (the bodies' own positions): not a support
    // point, and independent of the winning axis. `Collision_BoxAgainstBox` writes
    // `(colliderA.centre + colliderB.centre) * 0.5`.
    let point = (a.position + b.position) * 0.5;
    Some((best_axis, point, best_depth))
}

/// The hull's own axes in world space: right, up, forward.
fn axes(body: &Body) -> [Vec3; 3] {
    [body.right(), body.up(), body.forward()]
}

/// How much of the authored hull box a craft actually collides with.
///
/// **Ours, a feel knob rather than a measurement**: there is no original to match
/// (`0x08815ccc` reports nothing), so only play sets it.
///
/// Below `1.0` because `<Misc width height length>` is a **bounding** box sized to the widest
/// point: a real Pulse craft is `5.5 x 3.5 x 13`, whose half-length `6.5` is within a whisker
/// of the `6.45` bounding radius `oag-view` reports for the Feisar mesh. A hull tapers hard
/// toward the nose, so a full-size box collides along its whole length at the widest width and
/// craft bump where the models visibly miss (reported from play twice). Applied to every axis
/// because the taper is in plan *and* profile.
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

/// Resolves one craft-to-craft contact, moving both bodies. Returns `None` when the pair is
/// not touching or the gate refuses it.
///
/// Argument order does not change the outcome (equal and opposite impulse, symmetric split),
/// which matters because a pair loop visits each pair in *some* order.
pub fn resolve(
    a: &mut ShipState,
    a_size: &Dimensions,
    b: &mut ShipState,
    b_size: &Dimensions,
) -> Option<PairContact> {
    let (normal, point, depth) = overlap(&a.body, a_size, &b.body, b_size)?;
    respond(a, b, normal, point, depth)
}

/// The response half of [`resolve`], given a contact: `Body_ResolveContactPair` proper.
///
/// Split out so the arithmetic can be pinned against a contact the original was caught
/// resolving, inputs and output read off the running game (`tests.rs`).
///
/// # A pair hit never spins either craft (measured 2026-09-10)
///
/// The impulse is applied to each body **at its own position**: `Body_ResolveContactPair`
/// passes `body+0x30` as the "point" of both `Body_ApplyImpulseAtPoint` calls (`0x0884f178`
/// loads `a1 = s1 + 0x30`, `0x0884efdc` sets `s3 = s0 + 0x30`) and the applier's lever arm is
/// `*a1 - body+0x30`, exactly zero, so the angular half (`omega -= 0.1 * I^-1 (r x p)`) is
/// too. Caught live in PPSSPP on six contacts including a staged 84 units/s rear-end hit:
/// `a1 == bodyB + 0x30` on every one and both bodies' `+0x150` bit-identical across the call;
/// the PS2 twin (`0x0015e600`) does the same. Confidence **95**.
///
/// This function once applied the full angular half at the midpoint, giving `-267` to `-276`
/// degrees per second of yaw in one tick at a fast, offset first contact (`vn` around `-150`),
/// reported from play as far more spin than the originals. The original produces none.
///
/// **This is the pair path only.** A craft against the *track* goes through
/// `Body_ResolveContact` and [`crate::wall`], which pass the real contact point (`0x0884eea4:
/// move a1,s1`) and spin the craft at the `0.1` share `wall::ANGULAR_IMPULSE_SCALE` carries.
///
/// **The lever arm still reaches the denominator** `D`, with the full angular compliance, so
/// `j` is solved as if all the spin were applied and then none is: the pair-path version of
/// the "soft in translation, stiff in rotation" split ([`angular_term`]).
///
/// # The point velocity is textbook
///
/// The original computes each body's contact-point velocity as `v + cross(r, R^T omega)`
/// (`vtfm4.q C000,E100,C200` at `0x0884f038`, `vcrsp.t` at `0x0884f078` with the lever arm
/// on the left), the same as `v + omega x r` because `body+0x150` holds the rate negated and
/// in body coordinates (`R^T(body+0x150) == -omega`). See [`Body::velocity_at`] and
/// `docs/physics/angular-velocity-column.md`. So this path uses [`Body::velocity_at`], and
/// the two staged captures pin it: recomputing `j` from the raw `+0x150` bytes (which
/// `tests.rs`'s `captured()` maps in as `-R^T(raw)`) reproduces `43.16971` and `0.18440` to
/// the last bit. Feeding **this crate's** `angular_velocity` through the half-turn the raw
/// bytes need was a sign flip on the dominant mode (corrected 2026-09-10); `wall.rs` briefly
/// got the same wound and cost `07_Track` its clean lap ([`Body::velocity_at`]).
pub fn respond(
    a: &mut ShipState,
    b: &mut ShipState,
    normal: Vec3,
    point: Vec3,
    depth: f32,
) -> Option<PairContact> {
    let r_a = point - a.body.position;
    let r_b = point - b.body.position;
    let velocity_a = a.body.velocity_at(point);
    let velocity_b = b.body.velocity_at(point);

    // The gate: `vn` is positive while separating, so a pair coming apart faster than the
    // limit is left alone.
    let vn = (velocity_a - velocity_b).dot(normal);
    if vn - SEPARATING_LIMIT > 0.0 {
        return None;
    }

    // `n . ((I^-1 (r x n)) x r)` for each body (the pair's angular compliance) plus both
    // inverse masses; two masses is what makes this the pair path.
    let angular = angular_term(&a.body, r_a, normal) + angular_term(&b.body, r_b, normal);
    let denominator = angular + inverse_mass(&a.body) + inverse_mass(&b.body);
    if denominator <= f32::EPSILON {
        return None;
    }

    let impulse = -(PAIR_RESTITUTION_PLUS_ONE * vn) / denominator;
    // At each body's own centre: the linear half only.
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

fn inverse_mass(body: &Body) -> f32 {
    if body.mass <= f32::EPSILON {
        0.0
    } else {
        1.0 / body.mass
    }
}

/// `n . ((I^-1 (r x n)) x r)` with the **body-space** diagonal applied to the **world-space**
/// `r x n` directly, no rotation into the body frame.
///
/// The literal reading of `0x0884f3a8`-`0x0884f3d0` (the `+0x40..0x70` block through
/// `vtfm4.q` on a world-space cross product), as for the one-body denominator in
/// `contact-response.md`; on the 2026-09-10 tumbling-craft capture it reproduces the
/// original's `j` (`0.18440` against `0.18398` for the rotated textbook form): a quarter of a
/// percent, but the wrong quarter.
///
/// **Not a quirk (2026-09-10).** `body+0x40` is the inverse inertia the engine applies *in
/// world axes*: `Body_Integrate` maps body-frame momentum to rate through `R (body+0x40)
/// R^T`, i.e. `omega_world = (body+0x40) * L_world` in body coordinates, and a three-capture
/// fit reproduces the recorded `+0x160` column from `+0x150` and the basis at **100.00 %**
/// with the constructor's `(15.6, 21.6, 15.6)`. This is the engine's one convention
/// (`rigid-body.md`).
fn angular_term(body: &Body, r: Vec3, normal: Vec3) -> f32 {
    let torque = r.cross(normal);
    let inertia = body.inertia;
    // Component-wise inverse of the diagonal tensor, guarding a zero axis (`Handling::ZERO`
    // has one).
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
