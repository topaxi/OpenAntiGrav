//! Wall contact response: the ship stops passing through walls.
//!
//! Everything else in this crate reproduces a force law read out of the
//! original. **This does not, and says so up front.** What is recovered is the
//! *shape* of contact generation - `docs/ghidra/functions/psp-pulse/collision.md`
//! records a per-triangle separating-axis reject against the box axes, then ten
//! box sample points tested against the triangle, with contacts landing in
//! 0x40-byte slots, 128 of them - but not the response law that consumes those
//! contacts. So the contact *inputs* here are evidence-shaped and the *response*
//! is an ordinary projection: push out, then remove the inward velocity with
//! restitution. It is labelled an implementation choice everywhere it shows,
//! and M3's trace comparison is what will replace it.
//!
//! Two things it does reproduce exactly, because both are recovered:
//!
//! - The hull is a **box**, and its extents come from `<Misc width height
//!   length/>` in the ship's own `handlingstats.xml`. Nothing here is an invented
//!   dimension.
//! - Restitution is `0.05` for [`Surface::Wall`] and a `-1.0` **sentinel** for
//!   everything else, combined by [`combine_restitution`], which forces zero when
//!   either side is a sentinel. See [`Surface::restitution`].
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
//! contact restitution is therefore always `0.05`**. That is the intended
//! behaviour, not a coincidence to rely on: [`combine_restitution`] and the
//! sentinel are kept correct and unit-tested so that whoever enables lateral
//! floor contacts inherits the right semantics rather than rediscovering them.
//!
//! # Known limits
//!
//! - **One contact per frame.** The original keeps 128 slots; this takes the
//!   deepest. A ship wedged into a corner resolves one wall per frame rather than
//!   both at once.
//! - **No angular response.** Contacts are applied at the centre of mass, so a
//!   glancing hit slows the ship without yawing it. The original's contacts carry
//!   a point and would produce torque.
//! - **A nearer non-wall hit hides a wall behind it.** [`Raycaster::raycast`]
//!   returns the nearest hit of any surface, so a floor triangle in front of a
//!   wall along the same probe suppresses that probe.

use oag_core::math::Vec3;

use crate::collide::{Ray, Raycaster, Surface, combine_restitution};
use crate::forces::Environment;
use crate::hover;
use crate::params::{Dimensions, Handling};
use crate::ship::{Body, ShipState};

/// The ship's own restitution, for [`combine_restitution`].
///
/// **An assumption, not a reading.** The original combines the restitutions of
/// the two colliders in a contact, and what the ship's own collider declares was
/// not recovered. Taking it as a wall's makes a ship-versus-wall contact
/// `(0.05 + 0.05) / 2 = 0.05`, which is the value
/// `docs/formats/collision.md` names for a wall contact; any other choice would
/// have to explain why that number is quoted as the wall's own. Confidence 70.
pub const SHIP_RESTITUTION: Option<f32> = Some(crate::collide::WALL_RESTITUTION);

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
    /// The contact that was actually resolved: the deepest one, or the swept one.
    pub resolved: Option<WallContact>,
    /// The position correction applied to the body.
    pub escape: Vec3,
    /// The velocity change applied to the body.
    pub velocity_delta: Vec3,
    /// The restitution the resolved contact used.
    pub restitution: f32,
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
        return response;
    };

    let restitution = combine_restitution(SHIP_RESTITUTION, contact.surface.restitution());

    // Push out along the normal, exactly as far as the hull is past the surface.
    // No skin: landing flush leaves the next frame's depth at zero, which is a
    // no-op, where a skin would leave the ship visibly floating off the wall.
    let escape = contact.normal * contact.depth;
    state.body.position += escape;

    // Remove the inward velocity and give back `restitution` of it. Only when the
    // ship is actually moving into the surface: a ship sliding *along* a wall, or
    // already leaving it, must not be shoved.
    let normal_speed = state.body.linear_velocity.dot(contact.normal);
    let velocity_delta = if normal_speed < 0.0 {
        let delta = contact.normal * (-(1.0 + restitution) * normal_speed);
        state.body.linear_velocity += delta;
        delta
    } else {
        Vec3::ZERO
    };

    WallResponse {
        resolved: Some(contact),
        escape,
        velocity_delta,
        restitution,
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

/// The deepest of the hull probes, and how many found something.
///
/// Eight probes: sideways from the nose, the centre and the tail, and forwards
/// and backwards from the centre. That is a coarser sampling than the original's
/// ten box points against every candidate triangle, and it is chosen to cover the
/// hull's length rather than to match a count nobody can check.
fn deepest_hull_contact<R: Raycaster + ?Sized>(
    state: &ShipState,
    handling: &Handling,
    env: &Environment,
    raycaster: &R,
) -> (Option<WallContact>, u32) {
    let body = &state.body;
    let dimensions = &handling.dimensions;
    let right = body.right();
    let forward = body.forward();

    // Reuse the hover probes' own fore and aft placement rather than deriving a
    // second one, so the two cannot drift apart.
    let [fore, aft] = hover::probe_offsets(handling);
    let along_hull = [Vec3::ZERO, fore, aft];

    let mut deepest: Option<WallContact> = None;
    let mut contacts = 0;

    let mut consider = |origin: Vec3, direction: Vec3| {
        let reach = hull_extent(body, dimensions, direction);
        let probeable = reach > MIN_HULL_EXTENT;
        if !probeable {
            return;
        }
        let Some(hit) =
            raycaster.raycast(Ray::new(origin, direction, reach), env.self_collider, false)
        else {
            return;
        };
        if !responds(hit.surface) {
            return;
        }
        let depth = reach - hit.distance;
        let penetrating = depth > 0.0;
        if !penetrating {
            return;
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
    };

    for offset in along_hull {
        let origin = body.position + body.orientation * offset;
        consider(origin, right);
        consider(origin, -right);
    }
    consider(body.position, forward);
    consider(body.position, -forward);

    (deepest, contacts)
}

/// A normal flipped to oppose `direction`, and normalised.
///
/// Collision winding is not guaranteed to face the ship - the same fact the
/// collision debug view shades two-sided for - so the raw triangle normal is only
/// an axis, not an orientation.
fn facing(normal: Vec3, direction: Vec3) -> Vec3 {
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
        // Restitution 0.05: 10 in becomes 0.5 out.
        assert!(
            (state.body.linear_velocity.x + 0.5).abs() < 1e-3,
            "{:?}",
            state.body.linear_velocity
        );
        assert!((response.restitution - 0.05).abs() < 1e-6);
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

    /// A ship already leaving a surface must not be shoved along the normal, or a
    /// scrape would fling it: only the position correction applies.
    #[test]
    fn a_ship_moving_away_from_the_wall_keeps_its_velocity() {
        let mut state = ship_at(1.0, -10.0);
        let response = resolve(
            &mut state,
            &handling(),
            &Environment::default(),
            &wall(1.6, Surface::Wall),
            Vec3::new(1.0, 0.0, 0.0),
        );
        assert!(response.resolved.is_some());
        assert_eq!(response.velocity_delta, Vec3::ZERO);
        assert_eq!(state.body.linear_velocity, Vec3::new(-10.0, 0.0, 0.0));
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
    /// floors contactable inherits the right rule instead of a negative
    /// restitution that adds energy.
    #[test]
    fn the_never_bounce_sentinel_combines_to_zero_and_never_to_a_negative() {
        assert_eq!(Surface::Wall.restitution(), Some(0.05));
        assert_eq!(Surface::Floor.restitution(), None);
        assert_eq!(Surface::MagFloor.restitution(), None);
        assert_eq!(Surface::Reset.restitution(), None);

        assert_eq!(combine_restitution(Some(0.05), Some(0.05)), 0.05);
        assert_eq!(combine_restitution(Some(0.05), None), 0.0);
        assert_eq!(combine_restitution(None, Some(0.05)), 0.0);
        assert_eq!(combine_restitution(None, None), 0.0);
        assert_eq!(
            combine_restitution(SHIP_RESTITUTION, Surface::Wall.restitution()),
            0.05
        );
    }
}
