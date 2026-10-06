//! Wall contact response: the ship stops passing through walls.
//!
//! **The response law is read out of the original**: `Body_ResolveContact` (`0x0884e968`)
//! and `Body_ApplyImpulseAtPoint` (`0x0884d64c`); see
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`. Per contact:
//!
//! ```text
//! j        = -(1 + body[0x388]) * dot(v_point, n)
//!            / (invMass + dot(n, cross(invI * cross(r, n), r)))
//! impulse  = j * n - contact[0x34] * (v_point - n * dot(v_point, n))
//! v       += impulse * invMass
//! position+= n * contact[0x30]
//! ```
//!
//! The tangential term matters most for speed: a **multiplicative per-frame loss on the
//! tangential velocity**, with no Coulomb clamp against the normal impulse and no `dt`,
//! which is why the force-balance work could never find it by enumerating force terms
//! ([`WALL_FRICTION`], `docs/physics/force-balance-ground-truth.md`).
//!
//! The contact inputs are read too (`docs/ghidra/functions/psp-pulse-usa/collision.md#contact-generation`):
//! `Collider_BoxSamplePoints` (`0x08818a00`) builds **ten** points on the hull box, and
//! `Collision_BoxAgainstMesh` (`0x08815cd4`) tests the segment from the box **centre** to
//! each against every candidate triangle: the ten-ray star [`hull_probes`] casts.
//!
//! Reproduced exactly, because recovered:
//!
//! - The hull is a **box** from `<Misc width height length/>`, scaled by
//!   [`crate::hover::TARGET_GLOBAL_SCALE`] ([`hull_sample_points`]).
//! - Friction is `0.05` for [`Surface::Wall`] and a `-1.0` **sentinel** for everything
//!   else, combined with the ship's `0.02` by [`combine_friction`], which forces zero when
//!   either side is a sentinel ([`Surface::friction`], [`SHIP_FRICTION`]).
//! - Restitution is [`BODY_RESTITUTION`], a property of the *body*.
//!
//! # Why this runs after the integrator, not as a force
//!
//! A penetration is a fact about where the body *ended up*, so [`crate::integrate::step`]
//! evaluates forces, integrates, then calls [`resolve`]. A spring stiff enough to stop a
//! ship inside one frame is exactly what the integrator cannot carry (see
//! [`crate::integrate`]'s stability note).
//!
//! # Every mesh surface makes hull contacts
//!
//! Corrected 2026-09-29: this crate once responded to `Wall` only, on its own reasoning
//! that the hover spring owns floors. The original does not: `Collision_DispatchPair`
//! (`0x08816eac`) dispatches on shape kind alone, `Collision_BoxAgainstMesh` tests every
//! candidate triangle without reading `collider+0x6c`, and `Collision_AddContact`
//! (`0x08816864`) reads only friction and owner. The ring consumer `FUN_088418e0` proves
//! non-wall contacts arrive: at `0x088426f4`-`0x08842728` it calls `FUN_08844100(craft, 3)`
//! when `collider+0x6c == 2`, a `Reset` contact. So [`hull_contacts`] takes every surface
//! the raycaster returns except `Reset` (handled by [`crate::reset`]). Confidence **88**.
//!
//! Two more mechanisms landed with it, because the three only match the original
//! together (tick for tick on the sunk and flank-down trials in
//! `docs/gameplay/leaving-the-track.md`):
//!
//! - **`Collision_AddContact`'s projection gate** ([`hull_contacts`]): a contact is taken
//!   only when the sample point's perpendicular projection lands inside the triangle the
//!   centre segment crossed (`Collision_SegmentTriangle`, `0x08818bdc`, called from
//!   `0x08816864` on `sample -> sample + n * (depth + 0.01)`), confidence **90**. Without
//!   it, placed 3.6 units into `16_Track`'s floor ours rose 3.5 in one tick against the
//!   original's 1.8.
//! - **`Body_StepWorld`'s pass 1** ([`pre_integration_clip`]): the pre-integration clip
//!   along the velocity to the box face (`0x0884f70c`, `Collision_RaycastWorld(.., 1, 2)`,
//!   backing off `0.9` of the overshoot), confidence **85**. On `03_Track` the gate rejects
//!   every corner and pass 1 lifts the original out (`0.757` measured, `0.752` predicted).
//!
//! A floor contact **moves the body and drives nothing else** ([`reacts`]): the original
//! charges no damage for it (friction combines to `0.0`; `FUN_088418e0` damages only a
//! positive-friction record), and the impact edge, sparks and zone test stay wall-only by
//! choice. A hovering craft never makes one, so ordinarily only `Wall` responds, at the
//! combined friction `(0.05 + 0.02) / 2 = 0.035`.
//!
//! # Known limits
//!
//! - **The contact set is built probe-major; the original's is triangle-major.** A
//!   triangle-major loop would need a "candidate triangles in this AABB" API that does not
//!   exist ([`hull_probes`] is shared with [`crate::reset`]). Every predicate is
//!   order-independent, so the **set** is identical but the **order** is not, and each
//!   contact reads the body state the previous one left.
//! - **Two invented caps**, [`MAX_HITS_PER_PROBE`] and [`HULL_CONTACTS`]; the original's
//!   128 is an array size. Hitting either is reported in
//!   [`WallResponse::dropped_contacts`].
//!
//! Closed limits, with their measured cost on the whole-lap scenario: "one contact per
//! sample point" and "a nearer non-wall hit hides a wall behind it" are closed by
//! [`Raycaster::raycast_all`] and **moved the lap by no digit** (real divergences this
//! track never exercises). "Two-sided normals": [`hull_contacts`] now *rejects* a sample
//! whose triangle faces away, as `Collision_BoxAgainstMesh` does; the lap moved barely
//! (617.5 to 618.6 units, from a spawn `oag_raceplay` no longer uses), as expected since
//! 99.6 % of `16_Track`'s wall triangles already face the circuit.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster, Surface, combine_friction};
use crate::forces::Environment;
use crate::params::{Dimensions, Handling};
use crate::ship::{Body, ShipState};

mod clip;
pub use clip::{CLIP_BACKOFF, CLIP_PROBE_REACH, Clip, pre_integration_clip};
mod impulse;
pub use impulse::{STUN_PER_CONTACT, apply_pending_impulse, post_blast_impulse};

/// The ship collider's own friction, for [`combine_friction`].
///
/// `0.02`, written to `collider+0x64` by the ship-entity constructor (`0x08840c74`)
/// through the setter at `0x0884da7c` (literal `0x3ca3d70a` at `0x088414e4`). A ship
/// against a wall combines to `(0.05 + 0.02) / 2 = 0.035`, **the whole of the sustained
/// speed loss** the force-balance work measured. Confidence 88.
///
/// One constructor branch passes `0.0` instead, when the entity's class word at `+0xb8`
/// is `6` and the global byte at `0x08ab07e3` is clear. That class is unread, so a racing
/// craft is assumed to take the `0.02` branch, as the trace shows.
pub const SHIP_FRICTION: Option<f32> = Some(0.02);

/// Restitution of a contact, from `body+0x388`.
///
/// **A property of the body, not either surface.** `Body_ResolveContact` (`0x0884e968`)
/// reads `body+0x388` at `0x0884eb28` and adds `1.0`: the textbook `-(1 + e) * vn`.
/// `Body_Init` (`0x0884de5c`) defaults it to `0.5`; the ship-entity constructor
/// overwrites it with `0.4` (`0x3ecccccd`, loaded `0x08841424`, stored `0x088414b8`, no
/// branch between). Confidence 88. (The `0.05` this once was came from `collider+0x64`,
/// now [`WALL_FRICTION`].)
///
/// **The PS2 build says `0.1`, deliberately not followed**: `Body_ResolveContactPair`
/// (`0x0015e600`) hardcodes a `-1.1` numerator. PSP is the target; see the divergence
/// table in `contact-response.md`.
pub const BODY_RESTITUTION: f32 = 0.4;

/// How much of a contact's angular impulse actually reaches the body: **`0.1`**
/// (`body+0x394`).
///
/// `Body_ApplyImpulseAtPoint` (`0x0884d64c`) applies the linear half at full strength
/// (`0x0884d6d0`) and scales the angular half by this field (loaded `0x0884d768`, scaled
/// `0x0884d77c`, stored back to `body+0x160` at `0x0884d7c8`). One writer in the image,
/// `0x08841520` in the ship-entity constructor (`0x08840c74`), literal `0x3dcccccd`; both
/// arms of the friction branch above it converge on that store. Confidence **90**.
///
/// **Deliberately inconsistent with the denominator.** [`resolve`]'s denominator holds the
/// *full* angular compliance, so `j` is computed as though all the spin were applied, and
/// then a tenth is. The contact is **soft in translation and very stiff in rotation**,
/// which stops a hull corner catching a crest from flipping the ship; applying either half
/// alone cannot reproduce it.
pub const ANGULAR_IMPULSE_SCALE: f32 = 0.1;

/// How far behind a surface a sample point may be and still make a contact.
///
/// `Collision_AddContact` (`0x08816864`) computes `d = dot(n, sample - v0)` at
/// `0x088168e4` and takes the contact only while `-2.0 < d < 0` (`0x08816920` rejects
/// `d >= 0`, `0x08816938` rejects `d <= -2.0`). Confidence **90**.
///
/// **A deeper overlap produces no contact at all**, which looks like a bug and is not: a
/// hull that far through has crossed the wall within the frame, which belongs to the
/// swept pass that runs first. Reproduced, because a clamp would make deep penetrations
/// recoverable in a way the original's are not.
pub const MAX_CONTACT_DEPTH: f32 = 2.0;

/// Below this hull extent the box is treated as degenerate and nothing responds.
///
/// Guards against zero `<Misc>` dimensions giving zero-length probes and a ship that
/// silently passes through walls. See [`WallResponse::hull_degenerate`].
pub const MIN_HULL_EXTENT: f32 = 1e-4;

/// One resolved contact between the hull and a surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallContact {
    /// Where the probe met the triangle, in world space.
    pub point: Vec3,
    /// The contact normal, **flipped to point back towards the ship**.
    ///
    /// A collision triangle's stored winding does not reliably face the ship, so the raw
    /// normal is negated when it points along the probe. Without that, half of a track's
    /// walls would push a ship further in.
    pub normal: Vec3,
    /// How far the hull is past the surface, along [`Self::normal`].
    pub depth: f32,
    /// The surface tag of what was hit.
    pub surface: Surface,
    /// Index of the collider that was hit.
    pub collider: u32,
}

/// What the wall constraint did this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WallResponse {
    /// How many probes found a respondable surface.
    pub contacts: u32,
    /// How many of those were resolved. Equal to [`Self::contacts`] today; kept separate
    /// because the original's depth gate can drop one.
    pub resolved_count: u32,
    /// The deepest contact resolved this frame, or the swept one.
    /// The deepest contact resolved this frame, or the swept one. **Reporting only**: every
    /// contact in [`Self::contacts`] was resolved; this is the representative one for
    /// debug views and single-wall tests.
    pub resolved: Option<WallContact>,
    /// The total position correction applied to the body, summed over contacts.
    pub escape: Vec3,
    /// The total velocity change applied to the body, summed over contacts.
    pub velocity_delta: Vec3,
    /// The total angular-velocity change applied, summed over contacts.
    pub angular_velocity_delta: Vec3,
    /// The friction [`Self::resolved`] used.
    pub friction: f32,
    /// How many of [`Self::resolved_count`] were against a frictionless surface (`Floor`,
    /// `MagFloor`), and so moved the body without driving any reaction ([`reacts`]).
    /// Reporting only.
    pub floor_contacts: u32,
    /// Set when the hull box is too small to probe with, so nothing can respond: a loud
    /// "this did nothing and here is why" instead of a feature that silently stops working.
    pub hull_degenerate: bool,
    /// How many contacts were discarded for want of room.
    ///
    /// Non-zero means a probe found more than [`MAX_HITS_PER_PROBE`] hits or the frame
    /// exceeded [`HULL_CONTACTS`]. Both caps are **inventions** (the original has
    /// neither), so such a frame may differ from the original, and says so.
    pub dropped_contacts: u32,
    /// Whether any contact this frame was inbound (`normal_speed < 0.0`).
    ///
    /// **Reporting only**, not read by physics: [`ShipState::wall_contact_prev`]'s value
    /// for this frame, exposed so a cosmetic consumer (sparks) can edge-trigger without
    /// touching simulation state. It stays `true` through a sustained scrape, so a
    /// consumer must edge-detect (see [`STUN_PER_CONTACT`] for what firing on the level
    /// costs).
    pub impact: bool,
    /// The sum of `|p|` over every contact resolved this frame: the impulse
    /// `resolve_contact` applied, normal and tangential together, as the original stores
    /// per contact record. Summed because `FUN_088418e0` walks the whole ring and reacts
    /// once per record, so two contacts damage twice.
    ///
    /// The input to [`crate::damage::contact_damage`], and the one field here that
    /// physics writes for gameplay rather than for a debug view.
    pub impulse_sum: f32,
    /// The most inbound `normal_speed` across this frame's contacts, as a positive
    /// magnitude; `0.0` if [`Self::impact`] is `false`.
    ///
    /// **Reporting only.** `FUN_088418e0` (`contact-response.md`) scales the same
    /// per-contact impulse magnitude by `0.05` and `0.0125` for its own non-visual
    /// reactions, the anchor a spark effect's intensity curve borrows.
    pub impact_speed: f32,
}

/// The box support function: how far the hull reaches along `direction`.
///
/// `|n.right| * w/2 + |n.up| * h/2 + |n.forward| * l/2`. Half-extents are the `<Misc>`
/// dimensions **scaled by [`crate::hover::TARGET_GLOBAL_SCALE`] first** ([`hull_sample_points`]).
/// The hover probes are not placed from these (`hover::probe_offsets` is a code literal,
/// as is the inertia tensor), so this is the one place the shipped dimensions still act.
#[must_use]
pub fn hull_extent(body: &Body, dimensions: &Dimensions, direction: Vec3) -> f32 {
    let half_width = dimensions.width * crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    let half_height = dimensions.height * crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    let half_length = dimensions.length * crate::hover::TARGET_GLOBAL_SCALE * 0.5;

    direction.dot(body.right()).abs() * half_width
        + direction.dot(body.up()).abs() * half_height
        + direction.dot(body.forward()).abs() * half_length
}

/// Whether a surface takes part in the **swept** tunnelling guard: walls only.
///
/// The ten-ray star once used this filter too, which was this crate's invention: the
/// original's contact path reads no surface type, so hull contacts against `Floor` and
/// `MagFloor` happen as against `Wall` ([`hull_contacts`], module docs). The swept guard
/// keeps the filter because it is not the original's pass 1 (which moves the body back
/// along its velocity and applies no impulse) but this crate's own guard, which does
/// apply one, and nothing measured says what it should do against a floor.
#[must_use]
pub fn responds(surface: Surface) -> bool {
    !surface.is_hoverable()
}

/// Whether a contact drives the craft's gameplay reactions: damage, the impact edge, the
/// sparks and the zone's "clean" test.
///
/// `FUN_088418e0` charges hull damage only for a record whose stored friction
/// (`record + 0x1a0`, averaged by `Collision_AddContact`) is **positive**: `lwc1 f12,0x30(s0)`
/// at `0x08842648`, `c.le.s` against `0.0`, `bc1t` past the `Ship_Damage` call at
/// `0x088426ac`. A floor or magstrip is the `-1.0` sentinel and never damages.
/// Confidence **90** on the damage gate.
///
/// The impact edge, sparks and zone test share the predicate, **chosen, not measured**:
/// the original's spark and camera-shake dispatch at `0x0884263c` is gated on `0x0883e37c`
/// (unread) and a camera proximity test, so whether a floor contact sparks is open. Kept
/// wall-only so floor contacts change the body and nothing a player hears or is scored on.
#[must_use]
pub fn reacts(friction: f32) -> bool {
    friction > 0.0
}

/// Resolves the hull against the collision world, mutating the body.
pub fn resolve<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
) -> WallResponse {
    let dimensions = &handling.dimensions;
    let largest = dimensions
        .width
        .max(dimensions.height)
        .max(dimensions.length);
    // A bound bool so a `NaN` dimension lands in the degenerate branch.
    let usable = largest > MIN_HULL_EXTENT;
    if !usable {
        state.wall_contact_prev = false;
        return WallResponse {
            hull_degenerate: true,
            ..WallResponse::default()
        };
    }

    let mut response = WallResponse::default();

    // No swept query: a ship crossing a zero-thickness wall shell within one frame is
    // caught *before* integration by `Body_StepWorld`'s pass 1 ([`pre_integration_clip`],
    // `0x0884f70c`), where the original catches it. A fixed-size array, not a `Vec`:
    // world state is plain data (ADR-0003) and a bounded set keeps iteration order
    // trivially deterministic.
    let mut contacts = [None; HULL_CONTACTS];
    let (found, dropped) = hull_contacts(state, handling, env, raycaster, &mut contacts);
    response.dropped_contacts = dropped;
    response.contacts = found as u32;

    if found == 0 {
        // No wall this frame, so the next impact is a fresh one.
        state.wall_contact_prev = false;
        return response;
    }

    // **Ascending probe index**, the original's order: `Body_StepWorld` (`0x0884f70c`)
    // walks the contact array upward and `Collision_BoxAgainstMesh` fills it in sample
    // order. Each contact reads the body state the previous one left, as in the original,
    // and determinism forbids an order that depends on anything but the index.
    let mut impact = false;
    let mut impact_speed = 0.0f32;
    let mut deepest: Option<(WallContact, f32)> = None;

    for contact in contacts.iter().flatten().take(found) {
        let friction = combine_friction(SHIP_FRICTION, contact.surface.friction());
        let applied = resolve_contact(&mut state.body, contact, friction);

        response.resolved_count += 1;
        response.escape += applied.escape;
        response.velocity_delta += applied.velocity_delta;
        response.angular_velocity_delta += applied.angular_velocity_delta;
        if !reacts(friction) {
            // A floor contact moves the body and nothing else ([`reacts`]).
            response.floor_contacts += 1;
            continue;
        }
        impact |= applied.normal_speed < 0.0;
        impact_speed = impact_speed.max(-applied.normal_speed);
        response.impulse_sum += applied.impulse_magnitude;

        if deepest.is_none_or(|(d, _)| contact.depth > d.depth) {
            deepest = Some((*contact, friction));
        }
    }

    if let Some((contact, friction)) = deepest {
        response.resolved = Some(contact);
        response.friction = friction;
    }

    response.impact = impact;
    response.impact_speed = if impact { impact_speed } else { 0.0 };

    // A track contact does not arm the collision stun; see [`STUN_PER_CONTACT`].
    state.wall_contact_prev = impact;

    response
}

/// What resolving one contact did to the body.
struct Applied {
    escape: Vec3,
    velocity_delta: Vec3,
    angular_velocity_delta: Vec3,
    /// `dot(v_contactPoint, n)` before the impulse, for the stun edge test.
    normal_speed: f32,
    /// `|p|`, the length of the impulse actually applied. Zero on the
    /// degenerate-denominator path, which applies nothing.
    impulse_magnitude: f32,
}

/// `Body_ResolveContact` (`0x0884e968`) for one contact, in full.
///
/// ```text
/// r        = contact.point - body.position
/// v_p      = v + omega x r
/// vn       = dot(v_p, n)
/// D        = invMass + dot(n, cross(I_body^-1 * cross(r, n), r))
/// j        = -(1 + e) * vn / D                      (skipped when D == 0)
/// p        = j * n - friction * (v_p - n * vn)
/// v       += p * invMass
/// omega   += ANGULAR_IMPULSE_SCALE * R * I_body^-1 * R^T * cross(r, p)
/// position+= n * depth
/// ```
///
/// # The two asymmetries, both read rather than reasoned
///
/// **The denominator applies `body+0x40` straight to a *world-space* lever arm**
/// (`0x0884ebc0`-`0x0884ebe8`: loads the matrix at `body+0x40..0x70`, transforms
/// `cross(r, n)` with no rotation either side). Reproduced literally: `I` is
/// `(15.6, 21.6, 15.6)`, so the difference is a real 38 % on a pitched or rolled craft.
/// Confidence 88 on the field, 90 on the instructions.
///
/// **That is not an asymmetry (corrected 2026-09-10).** `+0x40` is a diagonal fixed in
/// **world** axes: `Body_Integrate`'s `R (+0x40) R^T` is only the trip in and out of the
/// body frame its fields are stored in, and that reading explains `100.00 %` of the
/// recorded momentum column on three captures (body-local: 91-95 %); see `rigid-body.md`
/// and `docs/physics/cornering-ground-truth.md`. A world-axis diagonal on a world lever
/// arm is the engine's one convention. The asymmetry that remains is **ours**:
/// [`crate::integrate`] rotates and this does not (see "The crate keeps its body-space
/// diagonal" on that page). Adopting the world-axis reading means moving all four sites
/// at once: here, `body_frame_inverse_inertia`, `crate::integrate` and
/// `crate::forces::YAW_INVERSE_INERTIA`, since a tensor applied in two frames in one
/// crate is the same trap as the integrator's sign flip.
///
/// **The application is textbook, scaled by [`ANGULAR_IMPULSE_SCALE`].**
/// `Body_ApplyImpulseAtPoint` rotates `cross(r, p)` into the body frame through `body+0xc0`
/// (the basis transpose; the rotation the denominator omits), then multiplies by
/// `body+0x394 = 0.1`. The sign is a subtraction there and an addition here, for the
/// `w_game = -w_physics` reason in [`crate::integrate`].
fn resolve_contact(body: &mut Body, contact: &WallContact, friction: f32) -> Applied {
    let normal = contact.normal;

    // Push out along the normal exactly as far as the hull is past the surface. No skin:
    // landing flush leaves the next depth at zero, where a skin would leave the ship
    // floating off the wall. `Body_ResolveContact` does the same (`Body_Translate(body, n
    // * contact[0x30])`) after the impulse; order is irrelevant since the impulse reads
    // velocities and the translate writes a position.
    let escape = normal * contact.depth;

    let inverse_mass = if body.mass > 0.0 {
        1.0 / body.mass
    } else {
        0.0
    };

    // The lever arm and the *contact point's* velocity, not the body's.
    let lever = contact.point - body.position;
    let point_velocity = body.linear_velocity + body.angular_velocity.cross(lever);
    let normal_speed = point_velocity.dot(normal);

    let angular_compliance = normal.dot(inverse_inertia(lever.cross(normal), body).cross(lever));
    let denominator = inverse_mass + angular_compliance;
    if denominator == 0.0 {
        // `0x0884ecb8`: a zero denominator returns without touching the body at all.
        return Applied {
            escape: Vec3::ZERO,
            velocity_delta: Vec3::ZERO,
            angular_velocity_delta: Vec3::ZERO,
            normal_speed,
            impulse_magnitude: 0.0,
        };
    }

    // The normal impulse. **Unconditional, as in the original**: there is no `vn < 0`
    // test in `Body_ResolveContact`, only that a contact exists (`depth > 0.0`), so a hull
    // still overlapping but moving out has that speed turned back into
    // `-BODY_RESTITUTION` of itself: a two-sided constraint. That is the "sticky wall" the
    // standing-start capture records (`dot(v, right)` knocked to `-11.5` and still `-2.2`
    // 230 ticks later); a one-sided push cannot produce it.
    //
    // **The PS2 build does gate it** (`Body_ResolveContactPair`, `0x0015e600`, returns early
    // when `vn > 0`). PSP is the target; see the divergence table in `contact-response.md`.
    let mut impulse = normal * (-(1.0 + BODY_RESTITUTION) * normal_speed / denominator);

    // The tangential impulse, the headline: `-friction * v_t` is a raw impulse, **not**
    // scaled by the normal impulse and **not** Coulomb-clamped, so each frame of contact
    // costs the contact point's tangential velocity a flat `friction` of itself. Against a
    // wall that is `3.5 %` per frame, `2.2 * |v|` of equivalent force at 60 Hz, the
    // "missing drag" the force balance chased.
    //
    // Applied **once** per contact, inline, as the PSP does (the PS2 defers it into an
    // 8-entry queue; the PSP's structural twin is consumed by the ship entity for the
    // gameplay reaction). The `friction > 0` gate is the `c.le.s`/`bc1f` pair at
    // `0x0884ed4c`; the `!= Vec3::ZERO` test reproduces the all-zero-tangent gate at
    // `0x0884ee00`-`0x0884ee34`.
    let tangential = point_velocity - normal * normal_speed;
    if friction > 0.0 && tangential != Vec3::ZERO {
        impulse -= tangential * friction;
    }

    // `Body_ApplyImpulseAtPoint` (`0x0884d64c`). The normal term is divided by mass here
    // and by the denominator above, which no longer cancel as with `invMass` alone.
    let velocity_delta = impulse * inverse_mass;
    let angular_velocity_delta =
        body_frame_inverse_inertia(lever.cross(impulse), body) * ANGULAR_IMPULSE_SCALE;

    body.linear_velocity += velocity_delta;
    body.angular_velocity += angular_velocity_delta;
    body.position += escape;

    Applied {
        escape,
        velocity_delta,
        angular_velocity_delta,
        normal_speed,
        impulse_magnitude: impulse.length(),
    }
}

/// `I^-1 * v` with the **body-space** tensor applied straight to a world vector: the
/// original's denominator, literally ([`resolve_contact`]). A zero inertia component
/// contributes zero, which makes a zeroed parameter set safe.
/// makes a zeroed parameter set safe.
fn inverse_inertia(v: Vec3, body: &Body) -> Vec3 {
    Vec3::new(
        divide_or_zero(v.x, body.inertia.x),
        divide_or_zero(v.y, body.inertia.y),
        divide_or_zero(v.z, body.inertia.z),
    )
}

/// `R * I^-1 * R^T * v`: the same tensor applied the way a world vector needs, as
/// `Body_ApplyImpulseAtPoint` and [`crate::integrate`] do to a torque.
fn body_frame_inverse_inertia(v: Vec3, body: &Body) -> Vec3 {
    let local = body.orientation.inverse() * v;
    body.orientation * inverse_inertia(local, body)
}

/// Component-wise division that treats a non-positive denominator as zero.
fn divide_or_zero(numerator: f32, denominator: f32) -> f32 {
    if denominator > 0.0 {
        numerator / denominator
    } else {
        0.0
    }
}

/// How many hull probes [`hull_probes`] returns.
///
/// Ten, because `Collider_BoxSamplePoints` (`0x08818a00`) builds ten.
pub const HULL_PROBES: usize = 10;

/// The original's ten box sample points, in world space.
///
/// `Collider_BoxSamplePoints` (`0x08818a00`) is twenty branch-free instructions over three
/// scaled axis vectors the collider caches, which `Collider_SetBoxTransform` (`0x08818964`)
/// fills as `row_i * dimension_i * 0.5` (`collider+0xe0` from row 0, `+0xc0` row 1,
/// `+0xd0` row 2; the `0.5` is `vfim.s S400,0x3800` at `0x08818990`). Rows 0/1/2 are the
/// body's right, up and forward; dimensions 0/1/2 are width, height, length, the ordering
/// `Body_SetBoxInertia` uses to reach `(15.6, 21.6, 15.6)` from the box `(12, 8, 12)`.
///
/// The ten, **in the original's memory order**, which is the order they become contacts
/// and resolve:
///
/// ```text
/// 0..3   centre - up*h/2  +- right*w/2  +- forward*l/2      the four lower corners
/// 4..7   centre + up*h/2  +- right*w/2  +- forward*l/2      the four upper corners
/// 8      centre + forward*l/8 + right*w/2                   the right flank
/// 9      centre + forward*l/8 - right*w/2                   the left flank
/// ```
///
/// The last pair is read, not guessed: `vfim.s S400,0x3400` at `0x08818a3c` is the
/// half-float `0.25` applied to the *forward* axis vector. They are wall-scrape probes.
///
/// **The `<Misc>` dimensions are scaled by `0.75` first**, the global
/// [`crate::hover::TARGET_GLOBAL_SCALE`] (`0x08ab0e1c`). `Ship_InitCraft`'s box-collider
/// setup (`0x08841360`-`0x0884139c`) multiplies `stats+0x78`, `stats+0x80`, `stats+0x7c`
/// (width, height, length; `docs/formats/handling-stats.md`) by `lwc1 f13,0xe1c(s1)`
/// before `Body_SetBoxDimensions` (`0x0884dccc`), in that order. That settles
/// width/length, which `Body_SetBoxInertia`'s square `(12, 8, 12)` box could not.
///
/// Confidence **90** on the geometry, **95** on the axis-to-dimension mapping (two
/// independent reads agree) and **90** on the `0.75` scale.
#[must_use]
pub fn hull_sample_points(body: &Body, handling: &Handling) -> [Vec3; HULL_PROBES] {
    let dimensions = &handling.dimensions;
    let centre = body.position;
    let scale = crate::hover::TARGET_GLOBAL_SCALE;
    // `collider+0xc0`, `+0xd0`, `+0xe0` respectively.
    let up = body.up() * (dimensions.height * scale * 0.5);
    let forward = body.forward() * (dimensions.length * scale * 0.5);
    let right = body.right() * (dimensions.width * scale * 0.5);

    let lower = centre - up;
    let upper = centre + up;
    let flank = centre + forward * 0.25;
    [
        lower + right + forward,
        lower + right - forward,
        lower - right + forward,
        lower - right - forward,
        upper + right + forward,
        upper + right - forward,
        upper - right + forward,
        upper - right - forward,
        flank + right,
        flank - right,
    ]
}

/// The hull's own probe set: `(origin, direction, reach)`, in world space.
///
/// One ray per [`hull_sample_points`] entry, from the centre of mass out to that point:
/// `Collision_BoxAgainstMesh` (`0x08815cd4`) calls `Collision_SegmentHitsTriangle`
/// (`0x08818d58`) with the box centre and the sample point as the segment ends, so a
/// sample point penetrates exactly when that segment crosses a triangle.
///
/// Shared with [`crate::reset`], which asks the same question of different geometry; two
/// definitions would drift. A degenerate reach is still returned; callers skip it with
/// [`MIN_HULL_EXTENT`].
#[must_use]
pub fn hull_probes(body: &Body, handling: &Handling) -> [(Vec3, Vec3, f32); HULL_PROBES] {
    let centre = body.position;
    hull_sample_points(body, handling).map(|point| {
        let offset = point - centre;
        let reach = offset.length();
        let direction = if reach > MIN_HULL_EXTENT {
            offset / reach
        } else {
            Vec3::ZERO
        };
        (centre, direction, reach)
    })
}

/// How many hits a single hull probe may contribute.
///
/// **An invention.** The original caps nothing per sample point: it appends until the
/// world's contact array is full, and that array's 128 is a layout artefact of
/// `world+0x450`. Four covers a point wedged in a corner of three or four wall triangles
/// and keeps [`HULL_CONTACTS`] stack-sized. Overflow is reported in
/// [`WallResponse::dropped_contacts`].
pub const MAX_HITS_PER_PROBE: usize = 4;

/// The hull contact buffer: [`HULL_PROBES`] probes times [`MAX_HITS_PER_PROBE`].
pub const HULL_CONTACTS: usize = HULL_PROBES * MAX_HITS_PER_PROBE;

/// Every hull contact, in probe order and then the geometry's own order.
///
/// Writes into `out`, returning how many it filled and how many were dropped. Fixed-size
/// and index-ordered, so the result depends on neither iteration order nor allocation
/// (`docs/architecture/determinism.md`).
///
/// One probe can make several contacts: `Collision_BoxAgainstMesh` (`0x08815cd4`) tests
/// each point against **every** candidate triangle and de-duplicates nothing, so a point
/// behind two triangles of a wall is scrubbed twice, and a floor triangle in front of a
/// wall does not hide it. Hence [`Raycaster::raycast_all`] rather than the nearest hit.
fn hull_contacts<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    out: &mut [Option<WallContact>; HULL_CONTACTS],
) -> (usize, u32) {
    let mut found = 0;
    let mut dropped = 0;

    for (origin, direction, reach) in hull_probes(&state.body, handling) {
        let probeable = reach > MIN_HULL_EXTENT;
        if !probeable {
            continue;
        }

        let mut hits = [None; MAX_HITS_PER_PROBE];
        let hit_count = raycaster.raycast_all(
            Ray::new(origin, direction, reach),
            env.self_collider,
            false,
            &mut hits,
        );

        for hit in hits.iter().flatten().take(hit_count) {
            // Every surface the raycaster returns makes a contact (it has dropped `Reset`;
            // the original reads no surface type; see the module docs).
            //
            // **Single-sided, by rejection rather than flipping.**
            // `Collision_BoxAgainstMesh` skips a sample unless `dot(boxCentre - s, n) > 0`
            // and passes the raw winding normal on, with no flip anywhere. `boxCentre - s`
            // is `-reach * direction`, so the gate is `dot(direction, n) < 0`, and every
            // accepted normal already points back at the ship (which is why [`facing`] is
            // unnecessary here).
            //
            // Faithful only if the shipped walls face the circuit, which was measured:
            // **2,076 of 2,084 wall triangles on `16_Track` (99.6 %)** are wound toward the
            // nearest point of the track's spline
            // (`crates/game/tests/race_ground_truth.rs`,
            // `the_track_s_walls_are_wound_toward_the_circuit`).
            let normal = hit.normal;
            if normal.dot(direction) >= 0.0 {
                continue;
            }
            // The original's depth is the *perpendicular* distance of the sample behind the
            // triangle plane, `-dot(n, sample - v0)`, not the shortfall along the probe.
            // They differ by the cosine between probe and normal, nowhere near one for a
            // corner against an oblique wall; the shortfall would turn a graze into a shove.
            let overshoot = reach - hit.distance;
            let depth = overshoot * -normal.dot(direction);
            let penetrating = depth > 0.0 && depth < MAX_CONTACT_DEPTH;
            if !penetrating {
                continue;
            }
            let sample = origin + direction * reach;
            if !projects_inside(sample, normal, depth, hit.triangle) {
                continue;
            }
            if found == out.len() {
                dropped += 1;
                continue;
            }
            out[found] = Some(WallContact {
                // The **sample point**, not where the segment met the triangle:
                // `Collision_AddContact` stores its `samplePoint` into `contact+0x00`
                // (`0x088169e8`) and discards the intersection. It matters for the lever
                // arm, as the two are a whole penetration depth apart.
                point: origin + direction * reach,
                normal,
                depth,
                surface: hit.surface,
                collider: hit.collider,
            });
            found += 1;
        }

        // A probe that filled its buffer may have had more behind it.
        if hit_count == MAX_HITS_PER_PROBE {
            dropped += 1;
        }
    }

    (found, dropped)
}

/// `Collision_AddContact`'s second test: does the sample point's perpendicular projection
/// land inside the triangle the centre-to-sample segment crossed?
///
/// Once the plane distance has passed the `(-2.0, 0)` gate, `Collision_AddContact`
/// (`0x08816864`) builds `p1 = sample - n * (d - 0.01)` (the sample carried along the
/// normal to **0.01 in front of** the plane) and calls `Collision_SegmentTriangle`
/// (`0x08818bdc`) on `(sample, p1)` against the **same** three vertices; the contact
/// stands only on a hit. That is a plane sign change plus three inclusive edge half-space
/// tests, i.e. [`crate::collide::segment_triangle`]; its `5.0` maximum distance
/// (`0x40a00000`) is unreachable behind the `-2.0` gate. Confidence **90**, instruction
/// level.
///
/// A corner past a triangle's edge makes **no** contact against it, nor against the
/// neighbour unless its own centre segment crosses that. On a strip floor that is two
/// contacts against four: placed 3.6 units into `16_Track`'s floor at spline index 200,
/// the original's first tick moves the craft 1.81 along the normal, where four contacts
/// would move it about 3.5.
///
/// A raycaster with no triangles (`triangle == None`, a test plane) skips the gate.
/// gate: an infinite plane contains every projection.
fn projects_inside(sample: Vec3, normal: Vec3, depth: f32, triangle: Option<[Vec3; 3]>) -> bool {
    let Some([a, b, c]) = triangle else {
        return true;
    };
    let past = sample + normal * (depth + 0.01);
    crate::collide::segment_triangle(sample, past, a, b, c).is_some()
}

/// A normal flipped to oppose `direction`, and normalised.
///
/// **The hull contact path no longer uses this**: [`hull_contacts`] reproduces
/// `Collision_BoxAgainstMesh`'s `dot(boxCentre - s, n) > 0` *rejection*, so flipping would
/// be a no-op on what it accepts and a fabrication on what it rejects. Two callers
/// remain, [`swept_contact`] and [`crate::reset`], which query something other than a
/// hull-versus-mesh contact (the original's swept pass is `Collision_RaycastWorld` with no
/// evidence of a side gate; reset volumes are trigger geometry), so a winding is an axis,
/// not an orientation.
pub(crate) fn facing(normal: Vec3, direction: Vec3) -> Vec3 {
    let n = normal.normalize_or_zero();
    if n.dot(direction) > 0.0 { -n } else { n }
}

#[cfg(test)]
mod tests;
