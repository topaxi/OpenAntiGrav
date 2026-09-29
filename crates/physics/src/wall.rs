//! Wall contact response: the ship stops passing through walls.
//!
//! **The response law here is read out of the original**, from
//! `Body_ResolveContact` (`0x0884e968`) and `Body_ApplyImpulseAtPoint`
//! (`0x0884d64c`); see
//! `docs/ghidra/functions/psp-pulse-usa/contact-response.md`. What the original does
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
//! `docs/ghidra/functions/psp-pulse-usa/collision.md#contact-generation`:
//! `Collider_BoxSamplePoints` (`0x08818a00`) builds **ten** points on the hull
//! box, and `Collision_BoxAgainstMesh` (`0x08815cd4`) tests the segment from the
//! box **centre** to each of them against every candidate triangle. That is a
//! ten-ray star from the centre of mass, which is exactly what [`hull_probes`]
//! casts here.
//!
//! Three things it reproduces exactly, because all three are recovered:
//!
//! - The hull is a **box**, and its extents come from `<Misc width height
//!   length/>` in the ship's own `handlingstats.xml`, scaled by
//!   [`crate::hover::TARGET_GLOBAL_SCALE`] before use - see
//!   [`hull_sample_points`]. Nothing here is an invented dimension.
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
//! # Every mesh surface makes hull contacts
//!
//! **Corrected 2026-09-29.** This section used to say only `Wall` responds,
//! because "the hover spring already owns" floors and a lateral probe that fired
//! on one would shove a banked ship off the surface it rests on. That was this
//! crate's own reasoning and the original does not share it:
//! `Collision_DispatchPair` (`0x08816eac`) dispatches on shape kind alone,
//! `Collision_BoxAgainstMesh` (`0x08815cd4`) tests every candidate triangle of
//! the mesh with no read of `collider+0x6c`, and `Collision_AddContact`
//! (`0x08816864`) reads the colliders' friction and owner and nothing else. The
//! ring consumer `FUN_088418e0` then proves non-wall contacts arrive: at
//! `0x088426f4`-`0x08842728` it looks up each record's mesh collider and calls
//! `FUN_08844100(craft, 3)` when `collider+0x6c == 2`, a `Reset` contact. So
//! [`hull_contacts`] now takes every surface the raycaster returns, which is
//! everything but `Reset` (handled by [`crate::reset`]). Confidence **88**.
//!
//! It is what recovers a craft whose hull has sunk into a floor: the lower
//! corners are behind the floor's plane by less than [`MAX_CONTACT_DEPTH`],
//! each makes a contact, and each translates the body out by its own depth.
//! Placed 3.6 units below rest height on `03_Track`'s level floor, the original
//! is back at its rest height and ours now is too; before this, ours fell
//! through. See `docs/gameplay/leaving-the-track.md`.
//!
//! A floor contact **moves the body and drives nothing else** - see [`reacts`]:
//! the original charges no damage for it (its friction combines to `0.0`, and
//! `FUN_088418e0` damages only a positive-friction record), and the impact edge,
//! sparks and zone test here stay wall-only by choice. A hovering craft never
//! makes one: its box sits above the surface it hovers over, so in ordinary
//! running the only surface that responds is still `Wall`, at the combined
//! friction `(0.05 + 0.02) / 2 = 0.035`.
//!
//! The swept guard in [`resolve`] keeps a wall-only filter ([`responds`]).
//!
//! # Known limits
//!
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
//! Three limits left this list rather than being written off, and what each cost
//! on the whole-lap scenario is recorded because a fidelity fix that changes
//! nothing is worth as much to know about as one that does:
//!
//! - **"At most one contact per sample point"** and **"a nearer non-wall hit
//!   hides a wall behind it"** were the same missing capability - a nearest-hit
//!   query - and [`Raycaster::raycast_all`] closes both. **Neither moved the lap
//!   by a single digit.** They were real divergences from the original that this
//!   track's geometry never exercises.
//! - **"Two-sided normals"**: [`hull_contacts`] now *rejects* a sample point
//!   whose triangle faces away, exactly as `Collision_BoxAgainstMesh` does,
//!   instead of flipping the normal. That one moved the lap, barely - 617.5
//!   units travelled to 618.6, both from the spline-sample-0 spawn that
//!   `oag_game::race` no longer uses, so it is the *difference* that carries the
//!   point and neither absolute reproduces - which is the size the data predicts: 99.6 % of
//!   `16_Track`'s wall triangles already face the circuit, so only the remaining
//!   0.4 % can behave differently.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster, Surface, combine_friction};
use crate::forces::Environment;
use crate::params::{Dimensions, Handling};
use crate::ship::{Body, ShipState};

mod impulse;
pub use impulse::{STUN_PER_CONTACT, apply_pending_impulse, post_blast_impulse};

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
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
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
    /// How many of [`Self::resolved_count`] were against a frictionless
    /// surface (`Floor`, `MagFloor`), and so moved the body without driving
    /// any reaction - see [`reacts`]. Reporting only.
    pub floor_contacts: u32,
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
    /// Whether any contact this frame was inbound (`normal_speed < 0.0`).
    ///
    /// **Reporting only**, same tier as [`Self::resolved`] - not read by physics.
    /// This is [`ShipState::wall_contact_prev`]'s own value for the frame that
    /// just ran, exposed so a cosmetic consumer (a spark effect, say) can build
    /// an edge trigger without touching simulation state. It stays `true` for
    /// every tick of a sustained scrape, which is exactly why a consumer needs
    /// to edge-detect rather than fire on the level - see [`STUN_PER_CONTACT`]'s
    /// doc comment for what firing on the level costs.
    pub impact: bool,
    /// The sum of `|p|` over every contact resolved this frame.
    ///
    /// `p` is the impulse `resolve_contact` applied, normal and tangential terms
    /// together - the same quantity the original stores per contact record and
    /// then reads back three ways. Summed rather than taken from the deepest
    /// contact because `FUN_088418e0` walks the whole ring and reacts once per
    /// record, so a frame with two contacts damages twice.
    ///
    /// This is the input to [`crate::damage::contact_damage`], and it is the one
    /// field of this struct that physics writes for gameplay rather than for a
    /// debug view.
    pub impulse_sum: f32,
    /// The most inbound `normal_speed` across this frame's contacts, negated
    /// to a positive magnitude - `0.0` if [`Self::impact`] is `false`.
    ///
    /// **Reporting only.** Not a recovered quantity by itself; `FUN_088418e0`
    /// (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) scales the same
    /// per-contact impulse magnitude by `0.05` and `0.0125` for its own
    /// (non-visual) reactions, which is the anchor a spark effect's intensity
    /// curve borrows.
    pub impact_speed: f32,
}

/// The box support function: how far the hull reaches along `direction`.
///
/// `|n.right| * w/2 + |n.up| * h/2 + |n.forward| * l/2`, the standard extent of
/// an oriented box along an axis. Along a body axis it degenerates to that axis's
/// own half-extent, which is why the probes below can use it uniformly.
///
/// Half-extents: `<Misc>` gives full hull dimensions, and the hull box is what
/// the collider is built from - **scaled by [`crate::hover::TARGET_GLOBAL_SCALE`]
/// first**, the same `0.75` global the hover target height uses. See
/// [`hull_sample_points`] for the read. The hover probes are **not** placed from
/// these - `oag_physics::hover::probe_offsets` is a code literal, as is the
/// inertia tensor - so this is the one place the shipped dimensions still act.
#[must_use]
pub fn hull_extent(body: &Body, dimensions: &Dimensions, direction: Vec3) -> f32 {
    let half_width = dimensions.width * crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    let half_height = dimensions.height * crate::hover::TARGET_GLOBAL_SCALE * 0.5;
    let half_length = dimensions.length * crate::hover::TARGET_GLOBAL_SCALE * 0.5;

    direction.dot(body.right()).abs() * half_width
        + direction.dot(body.up()).abs() * half_height
        + direction.dot(body.forward()).abs() * half_length
}

/// Whether a surface takes part in the **swept** tunnelling guard.
///
/// Walls only. This filter used to gate the hull's ten-ray star as well, and
/// that was this crate's own invention: `Collision_BoxAgainstMesh`
/// (`0x08815cd4`), `Collision_DispatchPair` (`0x08816eac`) and
/// `Collision_AddContact` (`0x08816864`) read no surface type anywhere, so the
/// original's hull makes contacts against `Floor` and `MagFloor` exactly as it
/// does against `Wall`. [`hull_contacts`] now does too; see the module docs,
/// "Every mesh surface makes hull contacts".
///
/// The swept guard keeps the wall-only filter because it is not the original's
/// pass 1 (`Body_StepWorld`'s pre-integration clip, which moves the body back
/// along its velocity and applies no impulse) but a tunnelling guard of this
/// crate's own that does apply one, and nothing measured says what it should
/// do against a floor.
#[must_use]
pub fn responds(surface: Surface) -> bool {
    !surface.is_hoverable()
}

/// Whether a contact drives the craft's gameplay reactions: damage, the
/// impact edge, the sparks and the zone's "clean" test.
///
/// `FUN_088418e0` walks the body's contact ring and charges hull damage only
/// for a record whose stored friction (`record + 0x1a0`, the `contact+0x34`
/// `Collision_AddContact` averaged) is **positive**: `lwc1 f12,0x30(s0)` at
/// `0x08842648`, then `c.le.s f12,f20` against `0.0` and `bc1t` past the
/// `Ship_Damage` call at `0x088426ac`. A floor or magstrip is the `-1.0`
/// sentinel, so its contacts combine to `0.0` and never damage. Confidence
/// **90** on the damage gate.
///
/// The impact edge, the sparks and the zone test are keyed to the same
/// predicate, which is **chosen, not measured**: the original's spark and
/// camera-shake dispatch at `0x0884263c` is not gated on friction but on
/// `0x0883e37c` (unread) and a camera proximity test, so whether a floor
/// contact sparks there is open. Kept wall-only here so that turning floor
/// contacts on changes the body and nothing a player hears or is scored on.
#[must_use]
pub fn reacts(friction: f32) -> bool {
    friction > 0.0
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
            // A floor contact moves the body and nothing else; see [`reacts`].
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
/// **The denominator applies `body+0x40` straight to a *world-space* lever
/// arm.** `0x0884ebc0`-`0x0884ebe8` loads the matrix at `body+0x40..0x70` and
/// transforms `cross(r, n)` with it, with no rotation either side. Reproduced
/// literally, because `I` is `(15.6, 21.6, 15.6)` and the difference is a real
/// 38 % on a pitched or rolled craft, not a rounding difference. Confidence 88
/// on the field identification, 90 on the instructions.
///
/// **This was read as an asymmetry and it is not one - corrected 2026-09-10.**
/// The reason recorded here was that `+0x40` is "unambiguously the body-space
/// tensor", so the original was skipping a rotation "here and only here".
/// `+0x40` is a diagonal fixed in **world** axes: `Body_Integrate`'s own
/// `R (+0x40) R^T` is only the trip in and out of the body frame its two
/// fields are stored in, and that reading explains `100.00 %` of the recorded
/// momentum column on three captures where the body-local one manages 91-95 %.
/// See
/// `docs/ghidra/functions/psp-pulse-usa/rigid-body.md` and
/// `docs/physics/cornering-ground-truth.md`. So a world-axis diagonal applied
/// to a world lever arm is not the odd case - it is the engine's one
/// convention, and this function was already right for a reason nobody had.
/// The asymmetry that remains is **ours**: [`crate::integrate`] rotates and
/// this does not, which is the deliberate divergence recorded under "The crate
/// keeps its body-space diagonal" on that page. Anyone adopting the world-axis
/// reading has to move all four sites at once - here, `body_frame_inverse_inertia`
/// below, `crate::integrate` and `crate::forces::YAW_INVERSE_INERTIA` - because
/// a tensor applied in two frames in one crate is the same "one convention, not
/// three coincidences" trap the integrator's sign flip already documents.
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
            impulse_magnitude: 0.0,
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
    // `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
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
        impulse_magnitude: impulse.length(),
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
/// **The `<Misc>` dimensions feeding the collider are not the authored values -
/// they are scaled by `0.75` first**, the same global
/// [`crate::hover::TARGET_GLOBAL_SCALE`] applies to the hover target height
/// (`0x08ab0e1c`). Read at instruction level in `Ship_InitCraft`'s box-collider
/// setup (`0x08841360`-`0x0884139c`): `stats+0x78`, `stats+0x80` and `stats+0x7c`
/// (width, height, length - `docs/formats/handling-stats.md`'s `<Misc>` offsets)
/// are each multiplied by `lwc1 f13,0xe1c(s1)` - the same address - before being
/// passed to `Body_SetBoxDimensions` (`0x0884dccc`) in that order, which settles
/// the width/length ordering too: `Body_SetBoxInertia`'s `(12, 8, 12)` box is
/// square in `x` and `z` and could never distinguish them, but here the operand
/// order is unambiguous - `stats+0x78` (width) feeds `Body_SetBoxDimensions`'s
/// first argument and `stats+0x7c` (length) its third, exactly the mapping this
/// function already used.
///
/// Confidence **90** on the geometry, **95** on the axis-to-dimension mapping
/// (up from 88 - two independent reads now agree, and this one distinguishes
/// width from length where the inertia tensor could not) and **90** on the
/// `0.75` scale, new with this reading.
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
            // Every surface the raycaster returns makes a contact: it has
            // already dropped `Reset`, and `Collision_BoxAgainstMesh` reads no
            // surface type at all. See the module docs.
            // **Single-sided, by rejection rather than by flipping.**
            // `Collision_BoxAgainstMesh` (`0x08815cd4`) skips a sample point
            // unless `dot(boxCentre - s, n) > 0`, and passes the raw winding
            // normal on to `Collision_AddContact` - there is no flip anywhere in
            // the original. `boxCentre - s` is `-reach * direction`, so the gate
            // is exactly `dot(direction, n) < 0`.
            //
            // Because it is a rejection, every accepted normal already points
            // back at the ship, which is what makes [`facing`] unnecessary here
            // rather than merely unused: the escape `normal * depth` and the
            // depth's own sign both come out right without it.
            //
            // This is only faithful if the shipped walls actually face the
            // circuit, which is a fact about the data and was measured rather
            // than assumed: **2,076 of 2,084 wall triangles on `16_Track`
            // (99.6 %)** are wound toward the nearest point of the track's own
            // spline. See
            // `crates/game/tests/race_ground_truth.rs`'s
            // `the_track_s_walls_are_wound_toward_the_circuit`.
            let normal = hit.normal;
            if normal.dot(direction) >= 0.0 {
                continue;
            }
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
/// **The hull contact path no longer uses this**, and the reason is worth
/// keeping: [`hull_contacts`] reproduces `Collision_BoxAgainstMesh`'s
/// `dot(boxCentre - s, n) > 0` *rejection*, and because a rejection only ever
/// accepts a normal that already opposes the probe, flipping afterwards would be
/// a no-op on everything it accepts and a fabrication on everything it does not.
///
/// Two callers remain, both querying something other than a hull-versus-mesh
/// contact: [`swept_contact`] and [`crate::reset`]. The original's swept pass is
/// `Collision_RaycastWorld`, a different query with no evidence of a side gate,
/// and reset volumes are trigger geometry rather than surfaces - so for both, a
/// triangle's winding is an axis and not an orientation.
pub(crate) fn facing(normal: Vec3, direction: Vec3) -> Vec3 {
    let n = normal.normalize_or_zero();
    if n.dot(direction) > 0.0 { -n } else { n }
}

#[cfg(test)]
mod tests;
