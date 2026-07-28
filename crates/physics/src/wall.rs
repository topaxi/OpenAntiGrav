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
//! and this module is that with `r = 0`, which collapses the denominator to
//! `invMass` and the normal term to `-(1 + e) * dot(v, n) * n`. The tangential
//! term is carried in full, and it is the one that matters: it is a
//! **multiplicative per-frame loss on the tangential velocity**, with no Coulomb
//! clamp against the normal impulse and no dependence on `dt`, which is why the
//! force-balance work could never find it by enumerating force terms. See
//! [`WALL_FRICTION`] and `docs/physics/force-balance-ground-truth.md`.
//!
//! The contact *inputs* were already evidence-shaped -
//! `docs/ghidra/functions/psp-pulse/collision.md` records a per-triangle
//! separating-axis reject against the box axes, then ten box sample points tested
//! against the triangle, with contacts landing in 0x40-byte slots, 128 of them -
//! and remain a coarser approximation here; see the known limits below.
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
//! - **One contact per frame.** The original keeps 128 slots; this takes the
//!   deepest. A ship wedged into a corner resolves one wall per frame rather than
//!   both at once. Because friction is multiplicative per contact resolved, a
//!   ship that the original would scrub twice in one frame is scrubbed once here.
//! - **No angular response.** Contacts are applied at the centre of mass, so a
//!   glancing hit slows the ship without yawing it. The original's contacts carry
//!   a point and would produce torque. Two consequences for the law above: the
//!   denominator collapses to `invMass`, and the tangential velocity is the
//!   body's rather than the contact point's.
//! - **A nearer non-wall hit hides a wall behind it.** [`Raycaster::raycast`]
//!   returns the nearest hit of any surface, so a floor triangle in front of a
//!   wall along the same probe suppresses that probe.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster, Surface, combine_friction};
use crate::forces::Environment;
use crate::hover;
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

/// Below this hull extent the box is treated as degenerate and nothing responds.
///
/// Guards the case that would otherwise fail silently: a parameter set whose
/// `<Misc>` dimensions are zero produces zero-length probes, no hit is ever
/// possible, and the ship passes through walls exactly as it did before. See
/// [`WallResponse::hull_degenerate`].
pub const MIN_HULL_EXTENT: f32 = 1e-4;

/// Seconds of [`ShipState::stun_timer`] a resolved contact adds.
///
/// `craft+0x290 += 0.5` in `Ship_ApplyCollisionImpulse` (`0x0883f274`), which is
/// the original's site for applying a contact impulse. Confidence 85; see
/// `docs/ghidra/functions/psp-pulse/engine.md`.
///
/// **Added, not assigned**, exactly as the original does, so a ship that keeps
/// hitting things accumulates stun rather than holding a flat half second.
///
/// # This crate arms it from the wall constraint, which is not where the original does
///
/// `Ship_ApplyCollisionImpulse` runs on the entity, outside `Ship_UpdateCraft`, and
/// handles impulses from every source - walls, other ships, weapons. This crate has
/// only the wall constraint, so that is the one source wired up. The gate it feeds
/// is faithful; the set of things that can trigger it is not yet complete, and a
/// ship-to-ship contact should arm the same timer when that exists.
///
/// The original's impulse path also projects the impulse onto the ship's forward
/// axis before applying it, which [`resolve`] does not: this crate removes the
/// inward normal velocity instead. That difference is in the impulse, not in the
/// stun, and is left as it was.
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
    /// The contact that was actually resolved: the deepest one, or the swept one.
    pub resolved: Option<WallContact>,
    /// The position correction applied to the body.
    pub escape: Vec3,
    /// The velocity change applied to the body.
    pub velocity_delta: Vec3,
    /// The friction the resolved contact used.
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
}

/// The box support function: how far the hull reaches along `direction`.
///
/// `|n.right| * w/2 + |n.up| * h/2 + |n.forward| * l/2`, the standard extent of
/// an oriented box along an axis. Along a body axis it degenerates to that axis's
/// own half-extent, which is why the probes below can use it uniformly.
///
/// Half-extents, following [`hover::probe_offsets`], which places the two hover
/// probes at `length * 0.5` fore and aft: `<Misc>` gives full hull dimensions.
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
    let resolved = swept_contact(state, handling, env, raycaster, previous_position)
        .inspect(|_| response.swept = true)
        .or_else(|| {
            let (deepest, contacts) = deepest_hull_contact(state, handling, env, raycaster);
            response.contacts = contacts;
            deepest
        });

    let Some(contact) = resolved else {
        // No wall this frame, so the next impact is a fresh one.
        state.wall_contact_prev = false;
        return response;
    };

    let friction = combine_friction(SHIP_FRICTION, contact.surface.friction());

    // Push out along the normal, exactly as far as the hull is past the surface.
    // No skin: landing flush leaves the next frame's depth at zero, which is a
    // no-op, where a skin would leave the ship visibly floating off the wall.
    //
    // `Body_ResolveContact` does the same, as `position += n * contact[0x30]`
    // with `contact[0x30]` the sample point's depth behind the triangle plane.
    let escape = contact.normal * contact.depth;
    state.body.position += escape;

    let velocity = state.body.linear_velocity;
    let normal_speed = velocity.dot(contact.normal);

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
    //
    // With the contact at the centre of mass the original's denominator is just
    // `invMass`, which cancels against the `impulse * invMass` on application, so
    // mass does not appear here at all.
    let normal_delta = contact.normal * (-(1.0 + BODY_RESTITUTION) * normal_speed);

    // The tangential impulse, and the headline. `-friction * v_t` is a raw
    // impulse, **not** scaled by the normal impulse and **not** clamped by a
    // Coulomb cone, so applying it costs the tangential velocity a flat
    // `friction` of itself every frame the contact exists. Against a wall that is
    // `3.5 %` per frame - `2.2 * |v|` of equivalent force at 60 Hz, which is what
    // the force balance was chasing as a missing drag term.
    //
    // Applied **once**, inline, as the PSP does. The PS2 defers its tangential
    // impulse into an 8-entry queue instead; the PSP has the structural twin of
    // that queue but consumes it from the ship entity for the gameplay reaction,
    // not to apply a second impulse. Two applications would cost `6.88 %` per
    // frame, which the capture refuses.
    //
    // Unlike the normal term this one *is* divided by mass, because it is an
    // impulse the denominator never touches.
    let tangential = velocity - contact.normal * normal_speed;
    let inverse_mass = if state.body.mass > 0.0 {
        1.0 / state.body.mass
    } else {
        0.0
    };
    let friction_delta = tangential * (-friction * inverse_mass);

    let velocity_delta = normal_delta + friction_delta;
    state.body.linear_velocity += velocity_delta;

    // Arm the collision stun, the other half of the contact response: for the next
    // half second the engine produces nothing and the ship has no lateral grip.
    //
    // Two conditions, and the second one is load-bearing:
    //
    // - The hull must actually have been moving *into* the surface. A ship already
    //   leaving one is only being position-corrected, and that is not an impact.
    //   Tested on `normal_speed` rather than on `velocity_delta`, which is no
    //   longer the same question: friction makes the delta non-zero for a pure
    //   scrape, and the impulse the original posts to `Ship_ApplyCollisionImpulse`
    //   is a real hit, not a graze.
    // - It must be the **first** such frame. `Ship_ApplyCollisionImpulse`
    //   (`0x0883f274`) is gated on a pending impulse vector at
    //   `entity->0x4c + 0x110` and zeroes that vector on its way out on every path,
    //   so the original's `craft+0x290 += 0.5` fires once per impact posted by the
    //   collision system - not once per frame of contact.
    //
    // Without the edge test this runs away: `0.5 s` armed per frame against a `dt`
    // decay is 30:1 at 60 Hz, so leaning on a wall for a second cuts the engine for
    // half a minute. See `ShipState::wall_contact_prev`.
    let impact = normal_speed < 0.0;
    if impact && !state.wall_contact_prev {
        state.stun_timer += STUN_PER_CONTACT;
    }
    state.wall_contact_prev = impact;

    WallResponse {
        resolved: Some(contact),
        escape,
        velocity_delta,
        friction,
        ..response
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
pub const HULL_PROBES: usize = 8;

/// The hull's own probe set: `(origin, direction, reach)`, in world space.
///
/// Eight probes: sideways from the nose, the centre and the tail, and forwards
/// and backwards from the centre. That is a coarser sampling than the original's
/// ten box points against every candidate triangle, and it is chosen to cover the
/// hull's length rather than to match a count nobody can check.
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
    let dimensions = &handling.dimensions;
    let right = body.right();
    let forward = body.forward();

    // Reuse the hover probes' own fore and aft placement rather than deriving a
    // second one, so the two cannot drift apart.
    let [fore, aft] = hover::probe_offsets(handling);
    let nose = body.position + body.orientation * fore;
    let tail = body.position + body.orientation * aft;
    let centre = body.position;

    let reach = |direction: Vec3| hull_extent(body, dimensions, direction);
    [
        (nose, right, reach(right)),
        (nose, -right, reach(-right)),
        (centre, right, reach(right)),
        (centre, -right, reach(-right)),
        (tail, right, reach(right)),
        (tail, -right, reach(-right)),
        (centre, forward, reach(forward)),
        (centre, -forward, reach(-forward)),
    ]
}

/// The deepest of the hull probes, and how many found something.
fn deepest_hull_contact<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
) -> (Option<WallContact>, u32) {
    let mut deepest: Option<WallContact> = None;
    let mut contacts = 0;

    for (origin, direction, reach) in hull_probes(&state.body, handling) {
        let probeable = reach > MIN_HULL_EXTENT;
        if !probeable {
            continue;
        }
        let Some(hit) =
            raycaster.raycast(Ray::new(origin, direction, reach), env.self_collider, false)
        else {
            continue;
        };
        if !responds(hit.surface) {
            continue;
        }
        let depth = reach - hit.distance;
        let penetrating = depth > 0.0;
        if !penetrating {
            continue;
        }
        contacts += 1;
        let contact = WallContact {
            point: hit.point,
            normal: facing(hit.normal, direction),
            depth,
            surface: hit.surface,
            collider: hit.collider,
        };
        if deepest.is_none_or(|d| contact.depth > d.depth) {
            deepest = Some(contact);
        }
    }

    (deepest, contacts)
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
    #[test]
    fn a_ship_driven_into_a_wall_is_pushed_out_and_bounces_back() {
        // Half-width is 1.0, so a centre at 1.0 leaves the hull 0.4 past x = 1.6.
        let mut state = ship_at(1.0, 10.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert!(!response.hull_degenerate);
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
        // `body+0x388` is 0.4, so `-(1 + e) * vn` turns 10 in into 4 out. The
        // velocity is purely along the normal, so friction has nothing to act on.
        assert!(
            (state.body.linear_velocity.x + 4.0).abs() < 1e-3,
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
    #[test]
    fn a_pure_scrape_costs_exactly_the_contact_friction_per_frame() {
        // Moving along the wall's plane (+z) rather than into it (+x), so
        // `dot(v, n)` is zero and only the tangential term acts.
        let mut state = ship_at(1.0, 0.0);
        state.body.linear_velocity = Vec3::new(0.0, 0.0, 40.0);

        resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );

        let expected = 40.0 * (1.0 - 0.035);
        assert!(
            (state.body.linear_velocity.z - expected).abs() < 1e-4,
            "{:?} is not {expected}",
            state.body.linear_velocity
        );
        // And it is a scale, not a decrement: a slower craft loses proportionally
        // less, which is the property the trace's flat ratio proves.
        state.body.position = Vec3::new(1.0, 0.0, 0.0);
        state.body.linear_velocity = Vec3::new(0.0, 0.0, 10.0);
        resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(
            (state.body.linear_velocity.z - 10.0 * (1.0 - 0.035)).abs() < 1e-4,
            "{:?}",
            state.body.linear_velocity
        );
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

    /// A real impact arms the collision stun, which is what cuts the engine and the
    /// grip for the next half second.
    #[test]
    fn an_impact_arms_the_collision_stun() {
        let mut state = ship_at(1.0, 10.0);
        assert_eq!(state.stun_timer, 0.0);

        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(response.resolved.is_some());
        assert_ne!(response.velocity_delta, Vec3::ZERO);
        assert_eq!(state.stun_timer, STUN_PER_CONTACT);
    }

    /// It accumulates rather than refreshing - `craft+0x290 += 0.5`, not `= 0.5`.
    ///
    /// Separate impacts, with the ship clear of the wall in between, which is what
    /// makes each one a fresh event rather than a continuation.
    #[test]
    fn repeated_impacts_accumulate_stun() {
        let mut state = ship_at(1.0, 10.0);
        for hit in 1..=3 {
            state.body.position = Vec3::new(1.0, 0.0, 0.0);
            state.body.linear_velocity = Vec3::new(10.0, 0.0, 0.0);
            resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &wall(1.6, Surface::Wall),
                Vec3::new(1.0, 0.0, 0.0),
            );
            assert_eq!(state.stun_timer, STUN_PER_CONTACT * hit as f32);

            // Clear of the wall, so the next one is a new impact.
            state.body.position = Vec3::new(-50.0, 0.0, 0.0);
            state.body.linear_velocity = Vec3::ZERO;
            resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &wall(1.6, Surface::Wall),
                Vec3::new(-50.0, 0.0, 0.0),
            );
        }
    }

    /// Held against a wall, the stun arms **once**, not once per frame.
    ///
    /// The regression this exists for was reported from play as "at some point the
    /// racer completely stopped accelerating". `Ship_ApplyCollisionImpulse`
    /// (`0x0883f274`) consumes a pending impulse vector and zeroes it on the way
    /// out, so its `+= 0.5` is per impact; arming per *frame* instead accumulates
    /// `0.5 s` against a `dt` decay - 30:1 at 60 Hz - and a one-second lean on a
    /// wall leaves the engine dead for half a minute.
    #[test]
    fn a_sustained_scrape_arms_the_stun_once() {
        let mut state = ship_at(1.0, 10.0);

        for _ in 0..60 {
            // Re-seeded each frame so the hull is always moving into the surface,
            // which is the worst case: the old code armed on every one of these.
            state.body.position = Vec3::new(1.0, 0.0, 0.0);
            state.body.linear_velocity = Vec3::new(10.0, 0.0, 0.0);
            resolve(
                &mut state,
                &handling(),
                &Environment::default(),
                &wall(1.6, Surface::Wall),
                Vec3::new(1.0, 0.0, 0.0),
            );
        }

        assert_eq!(
            state.stun_timer, STUN_PER_CONTACT,
            "a sustained contact is one impact, not sixty"
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
            &wall(1.6, Surface::Wall),
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
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(response.resolved.is_some());
        assert!(
            (state.body.linear_velocity.x - 4.0).abs() < 1e-3,
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
