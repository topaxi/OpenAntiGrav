//! Wall contact response: the ship stops passing through walls.
//!
//! **The response law here is read out of the original**, from
//! `Body_ResolveContact` (`0x0884e968`) and `Body_ApplyImpulseAtPoint`
//! (`0x0884d64c`); see
//! `docs/ghidra/functions/psp-pulse/contact-response.md`. What the original does
//! to a body per contact, in one pass, is
//!
//! ```text
//! j        = -(1 + body[0x388]) * dot(v_point, n)
//!            / (invMass + dot(n, cross(invI * cross(r, n), r)))
//! impulse  = j * n - contact[0x34] * (v_point - n * dot(v_point, n))
//! v       += impulse * invMass
//! position+= n * contact[0x30]
//! ```
//!
//! and this module is now that in full: the lever arm, the angular share of the
//! denominator and the angular half of the application are all present, and the
//! contacts come from the original's own box sample points rather than from an
//! invented probe set. The tangential term is the one that matters most for
//! speed: it is a **multiplicative per-frame loss on the tangential velocity**,
//! with no Coulomb clamp against the normal impulse and no dependence on `dt`,
//! which is why the force-balance work could never find it by enumerating force
//! terms. See [`WALL_FRICTION`] and `docs/physics/force-balance-ground-truth.md`.
//!
//! The contact *inputs* are read too, in
//! `docs/ghidra/functions/psp-pulse/collision.md#contact-generation`:
//! `Collider_BoxSamplePoints` (`0x08818a00`) builds **ten** points on the hull
//! box, and `Collision_BoxAgainstMesh` (`0x08815cd4`) tests the segment from the
//! box **centre** to each of them against every candidate triangle. That is a
//! ten-ray star from the centre of mass, which is exactly what [`hull_probes`]
//! casts here.
//!
//! Three things it reproduces exactly, because all three are recovered:
//!
//! - The hull is a **box**, and its extents come from `<Misc width height
//!   length/>` in the ship's own `handlingstats.xml`. Nothing here is an invented
//!   dimension.
//! - Friction is `0.05` for [`Surface::Wall`] and a `-1.0` **sentinel** for
//!   everything else, combined against the ship's own `0.02` by
//!   [`combine_friction`], which forces zero when either side is a sentinel. See
//!   [`Surface::friction`] and [`SHIP_FRICTION`].
//! - Restitution is [`BODY_RESTITUTION`], a property of the *body* rather than of
//!   either surface.
//!
//! # Why this runs after the integrator, not as a force
//!
//! A penetration is a fact about where the body *ended up*, so it is resolved
//! once the frame's motion has happened: [`crate::integrate::step`] evaluates
//! forces, integrates, and then calls [`resolve`]. That also means `race.rs` and
//! every other caller of `step` gets wall response without changing a line - the
//! query surface ([`Raycaster`]) was already class-agnostic, and only the
//! response was missing.
//!
//! It is deliberately *not* a force term. A spring stiff enough to stop a ship
//! inside one frame is exactly the stiffness the integrator cannot carry: see
//! [`crate::integrate`]'s stability note, where the force is evaluated once per
//! frame and held across all three sub-steps.
//!
//! # Only non-hoverable surfaces respond
//!
//! [`Surface::Floor`] and [`Surface::MagFloor`] are skipped, because the hover
//! spring already owns them and a lateral probe that fired on a floor would fight
//! it - a hard-banked ship's own right axis points partly downwards, and the
//! probe would then shove it sideways off a surface it is supposed to be resting
//! on. [`Surface::Reset`] is skipped one layer down, by [`Raycaster`] itself.
//!
//! So **in a real race the only surface that ever responds is `Wall`, and the
//! contact friction is therefore always `(0.05 + 0.02) / 2 = 0.035`**. That is
//! the intended behaviour, not a coincidence to rely on: [`combine_friction`] and
//! the sentinel are kept correct and unit-tested so that whoever enables lateral
//! floor contacts inherits the right semantics rather than rediscovering them.
//!
//! # Known limits
//!
//! - **Two-sided normals.** [`facing`] flips a triangle normal that points along
//!   the probe. The original does not flip: `Collision_BoxAgainstMesh` *rejects*
//!   the sample point when `dot(centre - sample, n) <= 0`, so a wall only
//!   collides from its front face. Kept as it was, because the collision debug
//!   view shades two-sided for a reason and turning walls single-sided is a
//!   separate experiment with its own risk.
//! - **The contact set is built probe-major, the original's is
//!   triangle-major.** `Collision_BoxAgainstMesh` loops candidate triangles on
//!   the outside and the ten sample points on the inside; this module does the
//!   reverse, because [`hull_probes`] is the shared query surface (with
//!   [`crate::reset`]) and a triangle-major loop would need a "candidate
//!   triangles in this AABB" API that does not exist. Every predicate is
//!   independent of the loop order, so the resulting **set** is identical - the
//!   **order** is not, and each contact reads the body state the previous one
//!   left.
//! - **Two invented caps**, [`MAX_HITS_PER_PROBE`] and [`HULL_CONTACTS`]. The
//!   original caps nothing per sample point; its 128 is the size of an array. A
//!   frame that hits either cap reports it in
//!   [`WallResponse::dropped_contacts`] rather than quietly resolving fewer.
//! - **The swept contact applies an impulse; the original's swept pass does
//!   not.** `Body_StepWorld` sweeps in pass 1 and only calls `Body_SetPosition`;
//!   its velocity change comes from whatever contacts pass 5 then finds. Ours is
//!   a tunnelling guard that has to do both because it runs after the move.
//! - **A successful sweep replaces the probe set for that frame.** [`resolve`]
//!   takes the swept contact and never calls [`hull_contacts`]. The original's
//!   pass 1 and pass 5 both run; ours are exclusive, because our sweep runs
//!   after the move and has already applied an impulse for the same crossing
//!   that pass 5's contacts would.
//!
//! Two limits left this list rather than being written off. **"At most one
//! contact per sample point"** and **"a nearer non-wall hit hides a wall behind
//! it"** were the same missing capability - a nearest-hit query - and
//! [`Raycaster::raycast_all`] closes both. Neither moved the whole-lap scenario
//! by a single digit, which is worth knowing: they were real divergences from
//! the original that this track's geometry never exercises.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster, Surface, combine_friction};
use crate::forces::Environment;
use crate::params::{Dimensions, Handling};
use crate::ship::{Body, ShipState};

/// The ship collider's own friction, for [`combine_friction`].
///
/// `0.02`, written to `collider+0x64` by the ship-entity constructor
/// (`0x08840c74`) through the setter at `0x0884da7c`; the literal is
/// `0x3ca3d70a` at `0x088414e4`. So a ship against a wall combines to
/// `(0.05 + 0.02) / 2 = 0.035`, and **that number is the whole of the sustained
/// speed loss** the force-balance work measured. Confidence 88.
///
/// One branch of that constructor passes `0.0` instead, when the entity's class
/// word at `+0xb8` reads `6` and a global byte at `0x08ab07e3` is clear. Which
/// entity class that is has not been read, so it is not modelled: a racing craft
/// is assumed to take the `0.02` branch, which is what the trace shows.
pub const SHIP_FRICTION: Option<f32> = Some(0.02);

/// Restitution of a contact, from `body+0x388`.
///
/// **A property of the body, not of either surface.** `Body_ResolveContact`
/// (`0x0884e968`) reads `body+0x388` at `0x0884eb28` and adds `1.0` to it before
/// scaling the normal-direction relative velocity - the textbook `-(1 + e) * vn`.
/// `Body_Init` (`0x0884de5c`) defaults the field to `0.5`, and the ship-entity
/// constructor overwrites it with `0.4` (`0x3ecccccd`, loaded at `0x08841424`,
/// stored at `0x088414b8` with no branch target in between). Confidence 88.
///
/// The `0.05` this constant used to be came from `collider+0x64`, which
/// [`WALL_FRICTION`] now correctly names: the resolver never reads that field as
/// a restitution.
///
/// **The PS2 build says `0.1`, and this crate deliberately does not follow it.**
/// `Body_ResolveContactPair` (`0x0015e600`) hardcodes a `-1.1` numerator rather
/// than reading a per-body field. PSP is the target, so `0.4` it is; see the
/// divergence table in
/// `docs/ghidra/functions/psp-pulse/contact-response.md`.
pub const BODY_RESTITUTION: f32 = 0.4;

/// How much of a contact's angular impulse actually reaches the body.
///
/// `body+0x394`, and it is **`0.1`**. `Body_ApplyImpulseAtPoint` (`0x0884d64c`)
/// applies the linear half at full strength (`v += p * invMass` at
/// `0x0884d6d0`) and then scales the angular half by this field before it
/// reaches the angular momentum accumulator (`0x0884d768` loads it,
/// `0x0884d77c` scales, `0x0884d7c8` stores `body+0x160` back). One writer in
/// the whole image, `0x08841520` in the ship-entity constructor
/// (`0x08840c74`), storing the literal `0x3dcccccd`; both arms of the
/// friction branch above it converge on that store, so a racing craft always
/// gets it. Confidence **90** - one literal, one writer, one reader, no branch.
///
/// # It is deliberately inconsistent with the denominator, and that is the point
///
/// [`resolve`]'s denominator contains the *full* angular compliance term, so the
/// impulse `j` is computed as though all of the spin were going to be applied,
/// and then only a tenth of it is. A textbook solver would use the same factor
/// in both places. The original does not, and the effect is a contact that is
/// **soft in translation and very stiff in rotation**: `j` is divided by a
/// denominator up to three times `invMass`, while the craft barely spins. That
/// combination is what stops a hull corner catching a crest from flipping the
/// ship, and it cannot be reproduced by applying either half alone.
pub const ANGULAR_IMPULSE_SCALE: f32 = 0.1;

/// How far behind a surface a sample point may be and still make a contact.
///
/// `Collision_AddContact` (`0x08816864`) computes the plane distance
/// `d = dot(n, sample - v0)` at `0x088168e4` and takes the contact only while
/// `-2.0 < d < 0`: `0x08816920` rejects `d >= 0` and `0x08816938` rejects
/// `d <= -2.0`, the `-2.0` being the `lui 0xc000` two instructions earlier.
/// Confidence **90**.
///
/// **A deeper overlap therefore produces no contact at all**, which reads like a
/// bug and is not one: a hull that far through a wall has crossed it within the
/// frame, and that case belongs to the swept pass, which runs first and does not
/// consult this gate. Reproduced rather than smoothed over, because a clamp here
/// would make deep penetrations recoverable in a way the original's are not.
pub const MAX_CONTACT_DEPTH: f32 = 2.0;

/// Below this hull extent the box is treated as degenerate and nothing responds.
///
/// Guards the case that would otherwise fail silently: a parameter set whose
/// `<Misc>` dimensions are zero produces zero-length probes, no hit is ever
/// possible, and the ship passes through walls exactly as it did before. See
/// [`WallResponse::hull_degenerate`].
pub const MIN_HULL_EXTENT: f32 = 1e-4;

/// Seconds of [`ShipState::stun_timer`] a **posted** collision impulse adds.
///
/// `craft+0x290 += 0.5` in `Ship_ApplyCollisionImpulse` (`0x0883f274`).
/// **Added, not assigned**, so a ship that keeps being hit accumulates stun
/// rather than holding a flat half second. Confidence 85; see
/// `docs/ghidra/functions/psp-pulse/engine.md`.
///
/// # Nothing in this crate arms it, and that is a correction
///
/// [`resolve`] used to, on the first frame of every inward contact. It was wrong,
/// and the evidence is three-legged:
///
/// - `Ship_ApplyCollisionImpulse` is gated on a **pending impulse vector** at
///   `entity->0x4c + 0x110` being non-zero, and it zeroes that vector on the way
///   out. Nothing on the track-contact path writes it: `Body_ResolveContact`
///   applies its impulse through `Body_ApplyImpulseAtPoint` and records the
///   contact into the observation ring, and `FUN_088418e0` - the only consumer of
///   that ring - computes camera and audio amplitudes from literals `0.05`,
///   `0.7`, `0.0125` and `1.0` and writes no impulse anywhere.
/// - `data/traces/talons-junction-time-trial-lap.csv` is 3,146 ticks of a
///   complete Time Trial lap, `4.8 %` of it in wall contact, and `stun_timer` is
///   **`0.0` on every tick**.
/// - `data/traces/talons-junction-standing-start.csv` is 300 more ticks, 230 of
///   them one continuous wall scrape, and `stun_timer` is **`0.0` on every tick**
///   there too.
///
/// So the timer belongs to being hit by something - a rival, a weapon - and not
/// to touching the track. The gate it feeds is real and stays
/// ([`ShipState::stun_timer`] still cuts thrust and lateral grip); the arming
/// site is simply not in this crate yet, the same way nothing arms
/// [`ShipState::leap_timer`].
///
/// **What that mistake cost, so nobody re-derives it**: with the old
/// single-deepest-probe contact model it fired rarely enough to look harmless.
/// The moment the hull grew its ten real sample points it fired on `2,045` of a
/// `3,146`-tick open-loop run - `65 %` of the race with no engine at all - and
/// the craft crawled `742` units instead of a lap. Two earlier bugs in the same
/// place are recorded history: arming *per frame* rather than per impact
/// (`0.5 s` armed against a `dt` decay is 30:1 at 60 Hz, reported from play as
/// "at some point the racer completely stopped accelerating"), and this one.
///
/// The original's impulse path also projects the posted impulse onto the ship's
/// forward axis before applying it, and scales the ship's own forward axis by the
/// impulse magnitude rather than using the impulse direction. Whoever wires up a
/// ship-to-ship contact wants `Ship_ApplyCollisionImpulse` reproduced whole, not
/// this constant on its own.
pub const STUN_PER_CONTACT: f32 = 0.5;

/// One resolved contact between the hull and a surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallContact {
    /// Where the probe met the triangle, in world space.
    pub point: Vec3,
    /// The contact normal, **flipped to point back towards the ship**.
    ///
    /// A collision triangle's stored winding does not reliably face the ship, so
    /// the raw normal is negated when it points along the probe rather than
    /// against it. Without that, half of a track's walls would push a ship
    /// further in.
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
    /// How many of those were resolved. Equal to [`Self::contacts`] today; kept
    /// separate because the original's depth gate can drop one.
    pub resolved_count: u32,
    /// The deepest contact resolved this frame, or the swept one.
    ///
    /// **Reporting only.** Every contact in [`Self::contacts`] was resolved; this
    /// is the representative one for debug views and for the tests that assert on
    /// a single hand-placed wall.
    pub resolved: Option<WallContact>,
    /// The total position correction applied to the body, summed over contacts.
    pub escape: Vec3,
    /// The total velocity change applied to the body, summed over contacts.
    pub velocity_delta: Vec3,
    /// The total angular-velocity change applied, summed over contacts.
    pub angular_velocity_delta: Vec3,
    /// The friction [`Self::resolved`] used.
    pub friction: f32,
    /// Set when the swept query, rather than a hull probe, found the contact.
    ///
    /// Worth surfacing: it means the ship crossed the wall entirely within one
    /// frame and would have tunnelled without it.
    pub swept: bool,
    /// Set when the hull box is too small to probe with, so nothing can respond.
    ///
    /// A loud "this did nothing and here is why", because the alternative is a
    /// feature that silently stops working when a parameter set arrives with zero
    /// dimensions.
    pub hull_degenerate: bool,
    /// How many contacts were discarded for want of room.
    ///
    /// Non-zero means a probe found more geometry than
    /// [`MAX_HITS_PER_PROBE`] allows, or the frame produced more than
    /// [`HULL_CONTACTS`] contacts in total. Both caps are **inventions** - the
    /// original has neither - so a frame that hits one is a frame where this
    /// crate and the original can differ, and it says so instead of quietly
    /// resolving fewer contacts.
    pub dropped_contacts: u32,
}

/// The box support function: how far the hull reaches along `direction`.
///
/// `|n.right| * w/2 + |n.up| * h/2 + |n.forward| * l/2`, the standard extent of
/// an oriented box along an axis. Along a body axis it degenerates to that axis's
/// own half-extent, which is why the probes below can use it uniformly.
///
/// Half-extents: `<Misc>` gives full hull dimensions, and the hull box is what
/// the collider is built from. The hover probes are **not** placed from these -
/// `oag_physics::hover::probe_offsets` is a code literal, as is the inertia
/// tensor - so this is the one place the shipped dimensions still act.
#[must_use]
pub fn hull_extent(body: &Body, dimensions: &Dimensions, direction: Vec3) -> f32 {
    let half_width = dimensions.width * 0.5;
    let half_height = dimensions.height * 0.5;
    let half_length = dimensions.length * 0.5;

    direction.dot(body.right()).abs() * half_width
        + direction.dot(body.up()).abs() * half_height
        + direction.dot(body.forward()).abs() * half_length
}

/// Whether a surface takes part in wall response at all.
///
/// Everything the hover spring owns is excluded; see the module docs.
#[must_use]
pub fn responds(surface: Surface) -> bool {
    !surface.is_hoverable()
}

/// Resolves the hull against the collision world, mutating the body.
///
/// `previous_position` is where the body was **before the integrator ran**, and
/// not before force evaluation: the hover path's penetration escape is a
/// deliberate teleport, and sweeping across it would invent a contact.
pub fn resolve<R: Raycaster + ?Sized>(
    state: &mut ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    previous_position: Vec3,
) -> WallResponse {
    let dimensions = &handling.dimensions;
    let largest = dimensions
        .width
        .max(dimensions.height)
        .max(dimensions.length);
    // Written as a bound bool so that a `NaN` dimension lands in the degenerate
    // branch rather than being compared into silence.
    let usable = largest > MIN_HULL_EXTENT;
    if !usable {
        state.wall_contact_prev = false;
        return WallResponse {
            hull_degenerate: true,
            ..WallResponse::default()
        };
    }

    let mut response = WallResponse::default();

    // The swept query first, because it covers the case the hull probes cannot:
    // the ship crossing a zero-thickness wall shell entirely within one frame.
    // Walls are single-sided triangles, so once the body is more than a hull
    // extent past one, no probe from the new position reaches back to it.
    //
    // A fixed-size array rather than a `Vec`: the world is plain data
    // (ADR-0003), and a bounded contact set is also what makes the iteration
    // order below trivially deterministic.
    let mut contacts = [None; HULL_CONTACTS];
    let found =
        if let Some(swept) = swept_contact(state, handling, env, raycaster, previous_position) {
            response.swept = true;
            contacts[0] = Some(swept);
            1
        } else {
            let (found, dropped) = hull_contacts(state, handling, env, raycaster, &mut contacts);
            response.dropped_contacts = dropped;
            found
        };
    response.contacts = found as u32;

    if found == 0 {
        // No wall this frame, so the next impact is a fresh one.
        state.wall_contact_prev = false;
        return response;
    }

    // **Iteration order is ascending probe index**, which is the original's own
    // order: `Body_StepWorld` (`0x0884f70c`) walks the contact array from `0`
    // upward at `0x0884ff??`, and `Collision_BoxAgainstMesh` fills it in sample
    // point order. It matters twice over - each contact reads the body state the
    // previous one left behind, exactly as the original's per-contact loop does,
    // and the determinism rules forbid an order that depends on anything but the
    // index.
    let mut impact = false;
    let mut deepest: Option<(WallContact, f32)> = None;

    for contact in contacts.iter().flatten().take(found) {
        let friction = combine_friction(SHIP_FRICTION, contact.surface.friction());
        let applied = resolve_contact(&mut state.body, contact, friction);

        response.resolved_count += 1;
        response.escape += applied.escape;
        response.velocity_delta += applied.velocity_delta;
        response.angular_velocity_delta += applied.angular_velocity_delta;
        impact |= applied.normal_speed < 0.0;

        if deepest.is_none_or(|(d, _)| contact.depth > d.depth) {
            deepest = Some((*contact, friction));
        }
    }

    if let Some((contact, friction)) = deepest {
        response.resolved = Some(contact);
        response.friction = friction;
    }

    // **A track contact does not arm the collision stun**, and this used to.
    // See [`STUN_PER_CONTACT`] for the evidence and for what it cost.
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
/// **The denominator uses the *body-space* inverse inertia on a *world-space*
/// lever arm.** `0x0884ebc0`-`0x0884ebe8` loads the matrix at `body+0x40..0x70`
/// and transforms `cross(r, n)` with it, and `body+0x40` is unambiguously the
/// body-space tensor: `Body_SetBoxInertia` (`0x0884e1ac`) writes it from the
/// hull box's own dimensions and nothing recomputes it, while the *world* copy
/// the integrator derives every sub-step lives at `body+0x80` and is what
/// `Body_ApplyImpulseAtPoint` reads. So the original skips the rotation here and
/// only here. Reproduced literally, because `I` is `(15.6, 21.6, 15.6)` and the
/// resulting error is a real 38 % on a pitched or rolled craft, not a rounding
/// difference - and because inventing the textbook form would make this
/// function disagree with the disassembly for no evidence at all. Confidence 88
/// on the field identification, 90 on the instructions.
///
/// **The application is textbook, and scaled by [`ANGULAR_IMPULSE_SCALE`].**
/// `Body_ApplyImpulseAtPoint` rotates `cross(r, p)` into the body frame through
/// `body+0xc0` (the basis transpose) before touching the momentum accumulator,
/// which is the rotation the denominator omits, and then multiplies by
/// `body+0x394 = 0.1`. The sign is a subtraction there and an addition here, for
/// the `w_game = -w_physics` reason [`crate::integrate`] documents.
fn resolve_contact(body: &mut Body, contact: &WallContact, friction: f32) -> Applied {
    let normal = contact.normal;

    // Push out along the normal, exactly as far as the hull is past the surface.
    // No skin: landing flush leaves the next frame's depth at zero, which is a
    // no-op, where a skin would leave the ship visibly floating off the wall.
    //
    // `Body_ResolveContact` does the same, as `Body_Translate(body, n *
    // contact[0x30])` with `contact[0x30]` the sample point's depth behind the
    // triangle plane. It happens *after* the impulse there; the order does not
    // matter to either, because the impulse reads velocities and the translate
    // writes a position.
    let escape = normal * contact.depth;

    let inverse_mass = if body.mass > 0.0 {
        1.0 / body.mass
    } else {
        0.0
    };

    // The lever arm and the *contact point's* velocity, not the body's. With
    // `r = 0` this whole function collapses back to what it used to be.
    let lever = contact.point - body.position;
    let point_velocity = body.linear_velocity + body.angular_velocity.cross(lever);
    let normal_speed = point_velocity.dot(normal);

    let angular_compliance = normal.dot(inverse_inertia(lever.cross(normal), body).cross(lever));
    let denominator = inverse_mass + angular_compliance;
    if denominator == 0.0 {
        // `0x0884ecb8` compares the denominator against zero and returns without
        // touching the body at all - no impulse, no translate, no contact record.
        return Applied {
            escape: Vec3::ZERO,
            velocity_delta: Vec3::ZERO,
            angular_velocity_delta: Vec3::ZERO,
            normal_speed,
        };
    }

    // The normal impulse. **Unconditional, exactly as the original is**: there is
    // no `vn < 0` test in `Body_ResolveContact`, only the test that a contact
    // exists at all - which is this module's `depth > 0.0`. So a hull still
    // overlapping the wall but already moving out has that outgoing speed turned
    // back into `-BODY_RESTITUTION` of itself, and the contact behaves as a
    // two-sided constraint rather than a one-sided push.
    //
    // That is the "sticky wall" the standing-start capture records: `dot(v,
    // right)` is knocked to `-11.5` on the impact frame and is still `-2.2` two
    // hundred and thirty ticks later, never returning to zero. A one-sided push
    // cannot produce that.
    //
    // **The PS2 build does gate it**, returning early from
    // `Body_ResolveContactPair` (`0x0015e600`) when `vn > 0`. PSP is what this
    // crate targets, so the gate stays out; see the divergence table in
    // `docs/ghidra/functions/psp-pulse/contact-response.md`.
    let mut impulse = normal * (-(1.0 + BODY_RESTITUTION) * normal_speed / denominator);

    // The tangential impulse, and the headline. `-friction * v_t` is a raw
    // impulse, **not** scaled by the normal impulse and **not** clamped by a
    // Coulomb cone, so applying it costs the contact point's tangential velocity
    // a flat `friction` of itself every frame the contact exists. Against a wall
    // that is `3.5 %` per frame - `2.2 * |v|` of equivalent force at 60 Hz, which
    // is what the force balance was chasing as a missing drag term.
    //
    // Applied **once** per contact, inline, as the PSP does. The PS2 defers its
    // tangential impulse into an 8-entry queue instead; the PSP has the
    // structural twin of that queue but consumes it from the ship entity for the
    // gameplay reaction, not to apply a second impulse.
    //
    // The `friction > 0` gate is the original's `c.le.s`/`bc1f` pair at
    // `0x0884ed4c`, and the all-zero-tangent gate at `0x0884ee00`-`0x0884ee34`
    // is reproduced by the `!= Vec3::ZERO` test: with no tangential velocity
    // there is nothing to scale and the original skips the subtraction outright.
    let tangential = point_velocity - normal * normal_speed;
    if friction > 0.0 && tangential != Vec3::ZERO {
        impulse -= tangential * friction;
    }

    // `Body_ApplyImpulseAtPoint` (`0x0884d64c`). Note that the normal term is
    // divided by mass here and by the denominator above, which no longer cancel
    // the way they did when the denominator was `invMass` alone.
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
    }
}

/// `I^-1 * v` with the **body-space** tensor applied straight to a world vector.
///
/// The original's denominator, literally: see [`resolve_contact`]. A zero
/// inertia component contributes zero rather than an infinity, which is what
/// makes a zeroed parameter set safe.
fn inverse_inertia(v: Vec3, body: &Body) -> Vec3 {
    Vec3::new(
        divide_or_zero(v.x, body.inertia.x),
        divide_or_zero(v.y, body.inertia.y),
        divide_or_zero(v.z, body.inertia.z),
    )
}

/// `R * I^-1 * R^T * v`: the same tensor applied the way a world vector needs.
///
/// What `Body_ApplyImpulseAtPoint` does, and what [`crate::integrate`] does to a
/// torque.
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

/// The contact from sweeping the body's centre along this frame's displacement.
fn swept_contact<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
    previous_position: Vec3,
) -> Option<WallContact> {
    let body = &state.body;
    let displacement = body.position - previous_position;
    let travelled = displacement.length();
    let moved = travelled > MIN_HULL_EXTENT;
    if !moved {
        return None;
    }
    let direction = displacement / travelled;

    let hit = raycaster.raycast(
        Ray::new(previous_position, direction, travelled),
        env.self_collider,
        false,
    )?;
    if !responds(hit.surface) {
        return None;
    }

    let normal = facing(hit.normal, direction);
    // Where the centre has to sit for the hull to rest flush against the surface.
    let rest = hit.point + normal * hull_extent(body, &handling.dimensions, normal);
    let depth = (rest - body.position).dot(normal);
    let penetrating = depth > 0.0;
    if !penetrating {
        return None;
    }

    Some(WallContact {
        point: hit.point,
        normal,
        depth,
        surface: hit.surface,
        collider: hit.collider,
    })
}

/// How many hull probes [`hull_probes`] returns.
///
/// Ten, because `Collider_BoxSamplePoints` (`0x08818a00`) builds ten.
pub const HULL_PROBES: usize = 10;

/// The original's ten box sample points, in world space.
///
/// `Collider_BoxSamplePoints` (`0x08818a00`) is twenty branch-free instructions
/// over three scaled axis vectors the collider caches, which
/// `Collider_SetBoxTransform` (`0x08818964`) fills as `row_i * dimension_i * 0.5`
/// (`collider+0xe0` from row 0, `+0xc0` from row 1, `+0xd0` from row 2, the
/// `0.5` being the `vfim.s S400,0x3800` at `0x08818990`). Rows 0/1/2 are the
/// body's right, up and forward, and dimensions 0/1/2 are width, height and
/// length: the same ordering `Body_SetBoxInertia` uses to reach the confirmed
/// `(15.6, 21.6, 15.6)` tensor from the box `(12, 8, 12)`.
///
/// Substituting that in, the ten are, **in the original's own memory order**,
/// which is the order they become contacts and therefore the order they resolve
/// in:
///
/// ```text
/// 0..3   centre - up*h/2  +- right*w/2  +- forward*l/2      the four lower corners
/// 4..7   centre + up*h/2  +- right*w/2  +- forward*l/2      the four upper corners
/// 8      centre + forward*l/8 + right*w/2                   the right flank
/// 9      centre + forward*l/8 - right*w/2                   the left flank
/// ```
///
/// The last pair is the interesting one and it is read, not guessed: the
/// `vfim.s S400,0x3400` at `0x08818a3c` is the half-float `0.25`, applied to the
/// *forward* axis vector, so the two extra points sit an eighth of the hull's
/// length ahead of centre and a full half-width out to either side. **They are
/// wall-scrape probes** - which is what a racer with a hull that never touches a
/// floor actually needs, and why the crate's old hand-chosen probe set happened
/// to be shaped roughly like them.
///
/// Confidence **90** on the geometry, **88** on the axis-to-dimension mapping
/// (which rests on the box-inertia agreement rather than on a second read).
#[must_use]
pub fn hull_sample_points(body: &Body, handling: &Handling) -> [Vec3; HULL_PROBES] {
    let dimensions = &handling.dimensions;
    let centre = body.position;
    // `collider+0xc0`, `+0xd0`, `+0xe0` respectively.
    let up = body.up() * (dimensions.height * 0.5);
    let forward = body.forward() * (dimensions.length * 0.5);
    let right = body.right() * (dimensions.width * 0.5);

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
/// One ray per [`hull_sample_points`] entry, cast from the body's centre of mass
/// out to that point. That is what `Collision_BoxAgainstMesh` (`0x08815cd4`)
/// tests: `Collision_SegmentHitsTriangle` (`0x08818d58`) is called with the box
/// centre and the sample point as the segment's ends, so a sample point counts as
/// penetrating exactly when the segment from the centre to it crosses a triangle.
///
/// Shared with [`crate::reset`], which asks the same question of different
/// geometry. One definition rather than two, because two would drift and the
/// difference would show up as a trigger volume that fires for the hull's left
/// side but not its right.
///
/// A probe whose reach is degenerate is still returned, with its reach as
/// computed; callers skip those with [`MIN_HULL_EXTENT`].
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
/// **An invention, and labelled as one.** The original caps nothing per sample
/// point: `Collision_BoxAgainstMesh` tests every point against every candidate
/// triangle and appends until the world's contact array is full, and that array
/// holds 128 - a layout artefact of `world+0x450`, not a design number. Four is
/// enough for a point wedged into a corner where three or four wall triangles
/// meet, which is the case the cap exists for, and it keeps
/// [`HULL_CONTACTS`] small enough to live on the stack.
///
/// Overflow is reported rather than swallowed: see
/// [`WallResponse::dropped_contacts`].
pub const MAX_HITS_PER_PROBE: usize = 4;

/// The hull contact buffer: [`HULL_PROBES`] probes times [`MAX_HITS_PER_PROBE`].
pub const HULL_CONTACTS: usize = HULL_PROBES * MAX_HITS_PER_PROBE;

/// Every hull contact, in probe order and then in the geometry's own order.
///
/// Writes into `out` and returns how many it filled, plus how many were dropped
/// for want of room. Fixed-size and index-ordered, so nothing about the result
/// depends on iteration order or on an allocation; see the determinism rules in
/// `docs/architecture/determinism.md`.
///
/// # One probe can make more than one contact
///
/// `Collision_BoxAgainstMesh` (`0x08815cd4`) tests each sample point against
/// **every** candidate triangle and de-duplicates nothing, so a point behind two
/// triangles of the same wall produces two contacts and is scrubbed twice. It
/// also means a floor triangle standing in front of a wall does not hide the
/// wall: each triangle is its own test. Both follow from asking the raycaster
/// for every hit rather than the nearest one, which is what
/// [`Raycaster::raycast_all`] is for.
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
            // Per hit, not per probe. Bailing on the nearest hit's surface is
            // what used to let a floor in front of a wall suppress the wall.
            if !responds(hit.surface) {
                continue;
            }
            let normal = facing(hit.normal, direction);
            // The original's depth is the *perpendicular* distance of the sample
            // point behind the triangle plane, `-dot(n, sample - v0)`, not the
            // shortfall along the probe. The two differ by the cosine between the
            // probe and the normal, and on a corner probe against an oblique wall
            // that cosine is nowhere near one - taking the shortfall would push the
            // hull out too far and turn a graze into a shove.
            let overshoot = reach - hit.distance;
            let depth = overshoot * -normal.dot(direction);
            let penetrating = depth > 0.0 && depth < MAX_CONTACT_DEPTH;
            if !penetrating {
                continue;
            }
            if found == out.len() {
                dropped += 1;
                continue;
            }
            out[found] = Some(WallContact {
                // The contact point is the **sample point**, not where the segment
                // met the triangle: `Collision_AddContact` stores `s1`, its
                // `samplePoint` argument, into `contact+0x00` at `0x088169e8`, and
                // the intersection point `Collision_SegmentTriangle` hands back goes
                // nowhere. It matters now that there is a lever arm, because the two
                // are a whole penetration depth apart.
                point: origin + direction * reach,
                normal,
                depth,
                surface: hit.surface,
                collider: hit.collider,
            });
            found += 1;
        }

        // A probe that filled its own buffer may have had more behind it.
        if hit_count == MAX_HITS_PER_PROBE {
            dropped += 1;
        }
    }

    (found, dropped)
}

/// A normal flipped to oppose `direction`, and normalised.
///
/// Collision winding is not guaranteed to face the ship - the same fact the
/// collision debug view shades two-sided for - so the raw triangle normal is only
/// an axis, not an orientation.
pub(crate) fn facing(normal: Vec3, direction: Vec3) -> Vec3 {
    let n = normal.normalize_or_zero();
    if n.dot(direction) > 0.0 { -n } else { n }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collide::{CollisionWorld, TriangleSoup};
    use crate::params::Dimensions;
    use crate::ship::Body;

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

    /// A large quad in the plane `x = at`, facing along -x.
    fn wall(at: f32, surface: Surface) -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [at, -100.0, -100.0],
                [at, 100.0, -100.0],
                [at, 100.0, 100.0],
                [at, -100.0, 100.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            surface,
            0,
        ));
        world
    }

    /// A quad in the plane `x = at` narrow enough that exactly **one** hull
    /// sample point reaches it.
    ///
    /// With [`handling`]'s box at the origin, the five right-hand sample points
    /// all share `x = centre + 1.0`, so any full-width plane catches all five at
    /// once and the friction law is applied five times over. That is the
    /// original's behaviour and it is pinned separately in
    /// `every_penetrating_sample_point_is_resolved_not_just_the_deepest`, but it
    /// is not what the *law* tests want to measure.
    ///
    /// The window here is chosen from the ray geometry rather than by trial: the
    /// right flank point sits at `(centre + 1, 0, -0.5)`, so its ray crosses
    /// `x = at` at `y = 0`, while the four right corners cross it at
    /// `y = +-0.3, z = +-1.2`. A window of `|y| <= 0.2`, `-1.0 <= z <= 0.5`
    /// therefore admits the flank point and nothing else.
    fn narrow_wall(at: f32, surface: Surface) -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [at, -0.2, -1.0],
                [at, 0.2, -1.0],
                [at, 0.2, 0.5],
                [at, -0.2, 0.5],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
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
            vec![[0, 1, 2], [0, 2, 3]],
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

    /// The headline behaviour. A ship overlapping a wall and moving into it must
    /// come back out *and* reverse a little, and this must fail loudly if the
    /// hull box ever regresses to zero - which is why the dimensions are set
    /// explicitly rather than left at `Handling::ZERO`.
    ///
    /// # The rebound is no longer `-(1 + e) * vn` exactly, and that is the point
    ///
    /// It used to be, because the denominator was `invMass` alone and cancelled
    /// against the `impulse * invMass` on application. With the angular term
    /// present the denominator is `invMass + n . ((I^-1 (r x n)) x r)`, which for
    /// this contact is
    ///
    /// ```text
    /// r      = (1, 0, -0.5)        the right flank point, relative to the centre
    /// r x n  = (0, 0.5, 0)         with n = (-1, 0, 0)
    /// I^-1   = (1/15.6, 1/21.6, 1/15.6)
    /// term   = n . ((0, 0.0231, 0) x r) = 0.01157
    /// j      = 1.4 * 10 / 1.01157 = 13.84
    /// ```
    ///
    /// so `10` in becomes `3.84` out rather than `4.00`. A one-percent softening
    /// on a lever this short; on a hull corner it is a factor of three. Asserted
    /// to `1e-2` on a value derived from the constants rather than measured, so
    /// that a regression to `4.0` - which is what dropping the angular term
    /// again would give - fails here.
    #[test]
    fn a_ship_driven_into_a_wall_is_pushed_out_and_bounces_back() {
        // Half-width is 1.0, so a centre at 1.0 leaves the hull 0.4 past x = 1.6.
        let mut state = ship_at(1.0, 10.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert!(!response.hull_degenerate);
        assert_eq!(response.contacts, 1, "{response:?}");
        let contact = response.resolved.expect("a wall contact");
        assert_eq!(contact.surface, Surface::Wall);
        assert!((contact.depth - 0.4).abs() < 1e-4, "{contact:?}");
        // Normal points back at the ship, i.e. along -x.
        assert!(contact.normal.x < -0.9, "{contact:?}");

        assert!(
            (state.body.position.x - 0.6).abs() < 1e-4,
            "{:?}",
            state.body
        );
        // The velocity is purely along the normal, so friction has nothing to act
        // on and only the normal impulse shows.
        assert!(
            (state.body.linear_velocity.x + 3.84).abs() < 1e-2,
            "{:?}",
            state.body.linear_velocity
        );
        assert!((response.friction - 0.035).abs() < 1e-6);
    }

    /// **The measurement this module exists to reproduce.**
    ///
    /// A craft in sustained wall contact loses a flat `3.5 %` of its speed every
    /// frame - `(0.05 + 0.02) / 2`, the combined contact friction - and the loss
    /// is *multiplicative*, not an impulse: it scales with the speed, which is why
    /// `docs/physics/force-balance-ground-truth.md` mistook it for a drag force
    /// linear in `fs` at coefficient `2.28`.
    ///
    /// This is the exact form of the law, with the normal velocity held at zero so
    /// nothing else contributes. `data/traces/talons-junction-standing-start.csv`
    /// gives the bound rather than the equality: from tick 66 on, the per-frame
    /// loss runs `5.21 %`, `3.92 %`, `3.78 %`, `3.63 %`, `3.577 %`, `3.560 %` -
    /// monotone, and **never below `3.5 %` on any of its 230 contact ticks**. It
    /// cannot go below, because moving speed out of the tangent and into the
    /// normal only adds loss; so the asymptote is a one-sided prediction that a
    /// friction of `0.036` would already have falsified.
    ///
    /// # Why the wall is narrow now
    ///
    /// The `3.5 %` is the loss for **one** contact, and the capture is a scrape:
    /// the craft is leaning on the wall with one flank, so one sample point is
    /// behind the plane. A full-width plane catches all five right-hand sample
    /// points, and five applications would cost `1 - 0.965^5 = 16.3 %`, which the
    /// capture forbids outright. [`narrow_wall`] reproduces the capture's
    /// geometry rather than its arithmetic; the five-at-once case is a real
    /// behaviour of the original and is pinned on its own below.
    #[test]
    fn a_pure_scrape_costs_exactly_the_contact_friction_per_frame() {
        for speed in [40.0f32, 10.0] {
            // A fresh state per speed, because a scrape off a point that is not
            // on the centre line also imparts a small yaw, and carrying that into
            // the second half would mean the second measurement was of a
            // *rotating* craft rather than of the friction law. And it is a
            // scale, not a decrement: a slower craft must lose proportionally
            // less, which is the property the trace's flat ratio proves.
            let mut state = ship_at(1.0, 0.0);
            // Moving along the wall's plane (+z) rather than into it (+x), so
            // `dot(v, n)` is zero and only the tangential term acts.
            state.body.linear_velocity = Vec3::new(0.0, 0.0, speed);

            let response = resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &narrow_wall(1.6, Surface::Wall),
                Vec3::new(1.0, 0.0, 0.0),
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

    /// The multi-contact case, which the single-deepest-probe model could not
    /// produce at all.
    ///
    /// A hull driven flat into a wall puts all five of its right-hand sample
    /// points - four corners and the right flank - behind the same plane, and
    /// `Body_StepWorld` resolves every contact the narrowphase produced. So the
    /// tangential loss compounds, `0.965^5`, and so does the push-out.
    ///
    /// **The compounding push-out is the original's**, not an artefact here:
    /// `Body_ResolveContact` ends with `Body_Translate(body, n * depth)` on every
    /// contact, with no shared-plane test anywhere, so a flat impact is ejected
    /// several depths rather than one. It is recorded rather than corrected
    /// because correcting it would be an invention, and because the geometry that
    /// produces it - a hull square-on to a wall - is a crash, not a racing line.
    #[test]
    fn every_penetrating_sample_point_is_resolved_not_just_the_deepest() {
        let mut state = ship_at(1.0, 0.0);
        state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);

        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(response.contacts, 5, "{response:?}");
        assert_eq!(response.resolved_count, 5, "{response:?}");
        // `0.965^5` to within a hundredth. Not exact, and the residual is real
        // rather than slop: the first contact leaves the body with a little
        // angular velocity, so the four after it measure the tangential velocity
        // *of their own contact point* rather than of the centre of mass.
        let expected = 40.0 * (1.0f32 - 0.035).powi(5);
        assert!(
            (state.body.linear_velocity.z - expected).abs() < 1e-2,
            "{:?} is not {expected}",
            state.body.linear_velocity
        );
        // Five push-outs of 0.4 each, all along -x.
        assert!(
            (response.escape.x + 2.0).abs() < 1e-3,
            "{:?}",
            response.escape
        );
    }

    /// The angular half of the response, which did not exist before: a contact
    /// away from the centre of mass turns the ship.
    ///
    /// The right flank point sits `length/8` **ahead** of the centre, so a wall
    /// on the right pushing along `-x` applies a torque that swings the nose to
    /// the left. Under this crate's textbook convention that is a positive
    /// rotation about `up`.
    ///
    /// The magnitude is pinned too, because it is where
    /// [`ANGULAR_IMPULSE_SCALE`] lives:
    ///
    /// ```text
    /// p        = (-13.84, 0, 0)              the impulse from the test above
    /// r x p    = (0, 6.92, 0)
    /// I^-1     = 1/21.6 on up
    /// omega   += 0.1 * 6.92 / 21.6 = 0.0320
    /// ```
    ///
    /// Without the `0.1` it would be `0.320`, an order of magnitude of spin per
    /// contact frame, which is the difference between a craft that scrapes along
    /// a wall and one that spins out on touching it.
    #[test]
    fn a_contact_off_the_centre_of_mass_yaws_the_ship() {
        let mut state = ship_at(1.0, 10.0);
        assert_eq!(state.body.angular_velocity, Vec3::ZERO);

        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(response.contacts, 1, "{response:?}");
        let omega = state.body.angular_velocity;
        assert!((omega.y - 0.032).abs() < 2e-3, "{omega:?}");
        assert!(omega.x.abs() < 1e-6 && omega.z.abs() < 1e-6, "{omega:?}");
        assert_eq!(response.angular_velocity_delta, omega);
    }

    /// A sample point more than [`MAX_CONTACT_DEPTH`] behind a surface produces
    /// **no contact**, which is `Collision_AddContact`'s `d <= -2.0` reject.
    ///
    /// Driven from the same geometry as the tests above, with the wall moved so
    /// the flank point is `2.4` units behind it instead of `0.4`. The swept pass
    /// is the thing that catches this case in a real frame, and it is
    /// deliberately given nothing to work with here (`previous_position` equals
    /// the current one) so that the gate is what the test sees.
    #[test]
    fn a_sample_point_too_far_behind_a_surface_makes_no_contact() {
        let mut state = ship_at(1.0, 10.0);
        let before = state.body;
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            // The right flank point is at x = 2.0; a plane at x = -0.4 leaves it
            // 2.4 behind.
            &narrow_wall(-0.4, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert_eq!(response.contacts, 0, "{response:?}");
        assert_eq!(state.body, before);
    }

    /// The ten sample points are the hull box's corners plus the two flank
    /// points, in the original's own order.
    ///
    /// Pinned because the *order* is the contact resolution order and the
    /// **flank pair is the part that would be easy to get wrong**: the `0.25`
    /// multiplies the forward axis, not the up axis, so the pair is a
    /// left/right pair slightly ahead of centre rather than a fore/aft pair
    /// slightly above it.
    #[test]
    fn the_ten_sample_points_are_the_hull_corners_plus_two_flank_points() {
        let body = Body {
            position: Vec3::ZERO,
            ..Body::default()
        };
        let points = hull_sample_points(&body, &handling());

        // width 2, height 1, length 4 -> half extents (1, 0.5, 2), and forward
        // is -Z.
        let expected = [
            Vec3::new(1.0, -0.5, -2.0),
            Vec3::new(1.0, -0.5, 2.0),
            Vec3::new(-1.0, -0.5, -2.0),
            Vec3::new(-1.0, -0.5, 2.0),
            Vec3::new(1.0, 0.5, -2.0),
            Vec3::new(1.0, 0.5, 2.0),
            Vec3::new(-1.0, 0.5, -2.0),
            Vec3::new(-1.0, 0.5, 2.0),
            // length/8 ahead of centre, a half-width out to either side.
            Vec3::new(1.0, 0.0, -0.5),
            Vec3::new(-1.0, 0.0, -0.5),
        ];
        for (got, want) in points.iter().zip(expected.iter()) {
            assert!((*got - *want).length() < 1e-6, "{got:?} is not {want:?}");
        }
        assert_eq!(points.len(), HULL_PROBES);
    }

    /// The tunnelling case, which the hull probes cannot see: the ship starts in
    /// front of the wall and ends far behind it, so no probe from the end
    /// position reaches back to a zero-thickness shell.
    #[test]
    fn a_ship_that_crossed_the_wall_within_one_frame_is_caught_by_the_sweep() {
        let mut state = ship_at(20.0, 200.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(5.0, Surface::Wall),
            Vec3::new(-5.0, 0.0, 0.0),
        );

        assert!(response.swept, "{response:?}");
        // Pushed back to one half-width in front of the wall.
        assert!(
            (state.body.position.x - 4.0).abs() < 1e-3,
            "{:?}",
            state.body
        );
        assert!(state.body.linear_velocity.x < 0.0);
    }

    /// A floor standing in front of a wall used to hide the wall entirely.
    ///
    /// [`Raycaster::raycast`] returns the nearest hit of *any* surface, so the
    /// old probe took the floor, found [`responds`] false and gave up - even
    /// though the wall two tenths of a unit further on was penetrating.
    /// `Collision_BoxAgainstMesh` has no such coupling: every triangle is its own
    /// test, and a surface the response ignores simply produces no contact rather
    /// than suppressing one.
    ///
    /// The nearest-hit reading is what the assertion below would have got: zero
    /// contacts, silently, on geometry that should push the ship out.
    #[test]
    fn a_floor_in_front_of_a_wall_no_longer_hides_it() {
        let mut world = CollisionWorld::new();
        push_quad(&mut world, 1.4, Surface::Floor, 0);
        push_quad(&mut world, 1.6, Surface::Wall, 1);

        let mut state = ship_at(1.0, 40.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &world,
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(response.contacts, 5, "{response:?}");
        assert!(
            response
                .resolved
                .is_some_and(|c| c.surface == Surface::Wall),
            "the wall behind the floor is what should have responded: {response:?}"
        );
        assert!(state.body.linear_velocity.x < 40.0, "{:?}", state.body);
    }

    /// One sample point behind two surfaces makes two contacts, and is scrubbed
    /// twice.
    ///
    /// `Collision_BoxAgainstMesh` de-duplicates nothing - see
    /// `docs/ghidra/functions/psp-pulse/collision.md#how-many-contacts-a-craft-vs-track-frame-makes`,
    /// which works the same arithmetic the other way round to conclude that the
    /// recorded scrape must have been *one* contact per frame. A nearest-hit
    /// query could not express this at all.
    ///
    /// Two walls a tenth apart rather than two coplanar ones, so the fixture
    /// cannot be read as depending on a tie-break.
    #[test]
    fn a_sample_point_behind_two_walls_is_scrubbed_twice() {
        let mut world = CollisionWorld::new();
        push_quad(&mut world, 1.5, Surface::Wall, 0);
        push_quad(&mut world, 1.6, Surface::Wall, 1);

        let mut state = ship_at(1.0, 0.0);
        state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);

        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &world,
            Vec3::new(1.0, 0.0, 0.0),
        );

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

    /// The hover spring owns floors. A lateral probe that fired on one would
    /// shove a banked ship off a surface it is meant to be resting on.
    #[test]
    fn hoverable_surfaces_never_produce_a_wall_contact() {
        for surface in [Surface::Floor, Surface::MagFloor] {
            let mut state = ship_at(1.0, 10.0);
            let before = state.body;
            let response = resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &wall(1.6, surface),
                Vec3::new(1.0, 0.0, 0.0),
            );
            assert_eq!(response.resolved, None, "{surface:?}");
            assert_eq!(state.body, before, "{surface:?}");
            assert!(!responds(surface));
        }
    }

    /// **No track contact arms the collision stun**, however hard or however
    /// sustained. See [`STUN_PER_CONTACT`] for the three legs of the evidence;
    /// the short version is that two captures totalling 3,446 ticks, 380 of them
    /// in wall contact, read `stun_timer == 0.0` on every single one.
    ///
    /// This replaces three tests that asserted the opposite
    /// (`an_impact_arms_the_collision_stun`,
    /// `repeated_impacts_accumulate_stun`, `a_sustained_scrape_arms_the_stun_once`).
    /// Their subject - `craft+0x290 += 0.5` being per posted impulse rather than
    /// per frame - is still true and still matters; it just belongs to
    /// `Ship_ApplyCollisionImpulse`, which this crate does not have a caller for
    /// yet. The property they were guarding is the same one this test guards from
    /// the other side: **the engine must never be cut by driving along a wall**.
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
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(response.resolved.is_some());
        assert_ne!(response.velocity_delta, Vec3::ZERO);
        assert_eq!(state.stun_timer, 0.0);

        // And sixty frames of it, re-seeded each time so the hull is always
        // moving into the surface. The pathology this guards against is
        // cumulative: `0.5 s` armed against a `dt` decay is 30:1 at 60 Hz, so
        // even one arming per second of contact buys half a minute of dead
        // engine.
        for _ in 0..60 {
            state.body.position = Vec3::new(1.0, 0.0, 0.0);
            state.body.linear_velocity = Vec3::new(10.0, 0.0, 0.0);
            state.body.angular_velocity = Vec3::ZERO;
            resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &wall(1.6, Surface::Wall),
                Vec3::new(1.0, 0.0, 0.0),
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
            // One contact, because five two-sided contacts on the same plane
            // reverse the outgoing velocity part-way through the frame and the
            // later ones then read as impacts. That compounding is the
            // original's and is pinned elsewhere; here the question is only
            // whether *leaving* a surface counts as a hit.
            &narrow_wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert_eq!(state.stun_timer, 0.0);
    }

    /// The contact is **two-sided**: a hull still overlapping the wall but already
    /// moving out is pulled back, not left alone.
    ///
    /// This is a deliberate reversal of what this module used to do, and it is
    /// what the original does: `Body_ResolveContact` (`0x0884e968`) has no
    /// `dot(v, n) < 0` test anywhere - the only gate is that a contact exists,
    /// which is `depth > 0.0` here. `-(1 + 0.4) * vn` with `vn` positive is
    /// negative, so `10` out becomes `4` back in.
    ///
    /// It matters because it is what makes a wall sticky, and stickiness is
    /// recorded: in `talons-junction-standing-start.csv` `dot(v, right)` is
    /// knocked to `-11.5` at tick 66 and is still `-2.2` at tick 295, never once
    /// returning to zero across 230 ticks. A one-sided push cannot hold a craft
    /// against a wall like that.
    #[test]
    fn a_ship_leaving_a_wall_it_still_overlaps_is_pulled_back() {
        let mut state = ship_at(1.0, -10.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &narrow_wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(response.resolved.is_some());
        // `3.84`, not `4.00`, for the angular-denominator reason spelled out on
        // `a_ship_driven_into_a_wall_is_pushed_out_and_bounces_back`. What
        // matters here is the *sign*: `10` out becomes `3.84` back in.
        assert!(
            (state.body.linear_velocity.x - 3.84).abs() < 1e-2,
            "{:?}",
            state.body.linear_velocity
        );
        // The position correction still applies, and it is what clears the
        // overlap so the next frame sees no contact at all.
        assert!(
            (state.body.position.x - 0.6).abs() < 1e-4,
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
            Vec3::new(1.0, 0.0, 0.0),
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
            Vec3::ZERO,
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
        assert!((hull_extent(&body, &h.dimensions, body.right()) - 1.0).abs() < 1e-6);
        assert!((hull_extent(&body, &h.dimensions, body.up()) - 0.5).abs() < 1e-6);
        assert!((hull_extent(&body, &h.dimensions, body.forward()) - 2.0).abs() < 1e-6);
    }

    /// The `-1.0` sentinel is not a coefficient. This path is unreachable in a
    /// race today, because only walls respond; it is pinned so that whoever makes
    /// floors contactable inherits the right rule instead of a negative friction
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
}
