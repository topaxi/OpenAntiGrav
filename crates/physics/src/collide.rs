//! The collision query surface the ship dynamics are written against.
//!
//! The types here are pinned deliberately: the geometry that answers these
//! queries is decoded in [`oag_formats::collision`](../../../formats/src/collision.rs)
//! and the code that asks them lives in [`crate::ship`], and neither should have
//! to wait for the other. A [`Raycaster`] is whatever can answer "what does this
//! segment hit", which in tests is a single hand-written triangle and in a race
//! is the track's collision soup.
//!
//! Evidence for the shape of all of this is in
//! `docs/ghidra/functions/psp-pulse/collision.md`.

use oag_core::math::Vec3;

/// What kind of surface a collider is.
///
/// All five collision node classes in the original share one vtable; the class
/// ID only selects this enum and a friction constant, which is why a magstrip
/// is not special geometry but ordinary floor with a different tag. Confidence
/// 92; see `docs/ghidra/functions/psp-pulse/collision.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Surface {
    /// `Wall Collision`, class `0x3ba`.
    Wall = 0,
    /// `Floor Collision`, class `0x3b9`. Accepted by the hover probes.
    Floor = 1,
    /// `Reset Collision`, class `0x3cd`. Excluded from ordinary raycasts;
    /// touching one respawns the ship.
    Reset = 2,
    /// `Mag Floor Collision`, class `0x3e6`. Byte-identical to [`Self::Floor`]
    /// at load apart from this tag, and the hover paths accept both
    /// interchangeably.
    MagFloor = 3,
}

impl Surface {
    /// Whether the hover probes accept contact with this surface.
    ///
    /// Types 1 and 3 only. This is the whole of the type test in the original's
    /// hover path, so it is stated once here rather than spelled out at each
    /// probe.
    #[must_use]
    pub fn is_hoverable(self) -> bool {
        matches!(self, Self::Floor | Self::MagFloor)
    }

    /// Whether a segment query should ignore this surface unless asked for it.
    #[must_use]
    pub fn is_skipped_by_default(self) -> bool {
        matches!(self, Self::Reset)
    }

    /// Surface friction, where `None` means **frictionless**.
    ///
    /// The class ID selects a surface type and this one float at `collider+0x64`
    /// and nothing else. The file's value for floors is `-1.0`, and that is a
    /// **sentinel, not a coefficient**: `Collision_AddContact` (`0x08816864`)
    /// forces the combined value to zero when either side is negative, so a
    /// negative never reaches the response. `None` makes that distinction
    /// impossible to lose. See `docs/formats/collision.md`.
    ///
    /// # This field was read as restitution until the contact resolver was
    ///
    /// `Body_ResolveContact` (`0x0884e968`) takes its restitution from
    /// `body+0x388` instead, and uses `contact+0x34` - which is what this value
    /// becomes - only to scale the *tangential* relative velocity. So the disc's
    /// `0.05` is a friction coefficient, and it is the whole of the sustained
    /// speed loss a scraping craft suffers. See
    /// `docs/ghidra/functions/psp-pulse/contact-response.md`.
    ///
    /// This deliberately duplicates `oag_formats::collision::SurfaceKind::
    /// friction`, because this crate must not depend on `oag-formats` - the
    /// same reason [`Surface`] itself is duplicated.
    /// `oag_gameplay::collision` owns the test that the two agree.
    #[must_use]
    pub fn friction(self) -> Option<f32> {
        match self {
            Self::Wall => Some(WALL_FRICTION),
            Self::Floor | Self::Reset | Self::MagFloor => None,
        }
    }
}

/// Friction of a [`Surface::Wall`], from `collider+0x64`.
///
/// The only non-sentinel value in the file.
pub const WALL_FRICTION: f32 = 0.05;

/// Friction of a contact between two colliders.
///
/// `Collision_AddContact` (`0x08816864`) averages the two values but forces zero
/// if **either** is negative, so one frictionless surface makes the whole contact
/// frictionless. A floor against a wall is therefore `0.0`, not the average of
/// `-1.0` and `0.05`.
#[must_use]
pub fn combine_friction(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => (a + b) * 0.5,
        _ => 0.0,
    }
}

/// What a segment query found.
///
/// Mirrors the original's 0x2c-byte hit result: point, normal, collider index,
/// the averaged per-vertex scalar, and a hit flag - the flag being `Option`'s
/// job here. `distance` is ours, not the original's, and is carried because
/// every caller needs it and recomputing it from `point` loses precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    /// Where the segment met the triangle, in world space.
    pub point: Vec3,
    /// The triangle's outward normal, normalised.
    pub normal: Vec3,
    /// Distance from the segment's origin to [`Self::point`].
    pub distance: f32,
    /// The surface tag of the collider that was hit.
    pub surface: Surface,
    /// Mean of the hit triangle's three per-vertex scalars, or `1.0` when the
    /// mesh carries no chunk 3.
    ///
    /// **What this value means is not known.** "Grip, friction or roughness" is
    /// a guess at confidence 40, so it is carried untouched rather than being
    /// given a name that would stop anyone looking.
    pub vertex_scalar: f32,
    /// Index of the collider within the world, for self-skipping.
    pub collider: u32,
}

/// Answers segment queries against collision geometry.
///
/// A trait rather than a concrete world so the dynamics can be tested against
/// one hand-placed triangle with no assets, no disc image and no GPU, which is
/// the entire reason the simulation is kept ignorant of everything else.
pub trait Raycaster {
    /// Casts from `origin` along `direction` for `length` units.
    ///
    /// `direction` is expected to be normalised. Returns the **nearest** hit, or
    /// `None`. Implementations must skip [`Surface::Reset`] unless
    /// `include_reset` is set, and must skip the collider whose index is
    /// `skip`, which is how the ship avoids hitting itself.
    fn raycast(&self, ray: Ray, skip: Option<u32>, include_reset: bool) -> Option<RaycastHit>;

    /// **Every** hit along the segment, not just the nearest.
    ///
    /// Writes into `out` and returns how many slots it filled, at most
    /// `out.len()`. Hits past that are **dropped rather than substituted**, so
    /// the set never depends on how full the buffer happened to get. Same
    /// `skip` and `include_reset` semantics as [`Self::raycast`].
    ///
    /// Order is each implementation's own storage order - ascending triangle
    /// index within a mesh, ascending collider index across a world - never a
    /// distance sort. That is deliberate: the consumer resolves contacts in the
    /// order it receives them and each one reads the body state the previous one
    /// left, so the order feeds simulation state and must not depend on a float
    /// comparison. See `docs/architecture/determinism.md`.
    ///
    /// # Why the hull contact path needs it
    ///
    /// `Collision_BoxAgainstMesh` (`0x08815cd4`) tests every box sample point
    /// against **every** candidate triangle with no de-duplication, so one point
    /// wedged into a corner produces two contacts and is scrubbed twice. A
    /// nearest-hit query cannot express that, and it also lets a floor triangle
    /// standing in front of a wall suppress the wall entirely - neither of which
    /// the original does. See
    /// `docs/ghidra/functions/psp-pulse/collision.md#contact-generation`.
    ///
    /// # The default implementation is a trap worth knowing about
    ///
    /// It reports only the nearest hit, so a `Raycaster` that has not overridden
    /// this - a test double, say - keeps exactly the old behaviour and produces
    /// exactly one contact per probe. That is the right thing for a double that
    /// exists to count queries, and the wrong thing to debug for an hour.
    fn raycast_all(
        &self,
        ray: Ray,
        skip: Option<u32>,
        include_reset: bool,
        out: &mut [Option<RaycastHit>],
    ) -> usize {
        match (self.raycast(ray, skip, include_reset), out.first_mut()) {
            (Some(hit), Some(slot)) => {
                *slot = Some(hit);
                1
            }
            _ => 0,
        }
    }
}

/// A bounded segment query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Where the segment starts, in world space.
    pub origin: Vec3,
    /// Normalised direction.
    pub direction: Vec3,
    /// How far along [`Self::direction`] the segment extends.
    pub length: f32,
}

impl Ray {
    /// A segment from `origin` along `direction` for `length` units.
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3, length: f32) -> Self {
        Self {
            origin,
            direction,
            length,
        }
    }

    /// The far end of the segment.
    #[must_use]
    pub fn end(&self) -> Vec3 {
        self.origin + self.direction * self.length
    }
}

/// An axis-aligned bounding box.
///
/// The original stores one per collider at `+0x00`/`+0x10` and one per triangle
/// inside the per-mesh sweep-and-prune list. Here it is only used for the reject
/// pass in front of the narrowphase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Lower corner.
    pub min: Vec3,
    /// Upper corner.
    pub max: Vec3,
}

impl Aabb {
    /// A box that contains nothing, and so overlaps nothing.
    ///
    /// `min` above `max` deliberately, so [`Self::overlaps`] rejects it without
    /// needing a separate emptiness flag.
    pub const EMPTY: Self = Self {
        min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// Grows the box to contain `point`.
    pub fn expand(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// The tightest box containing both endpoints of a segment.
    #[must_use]
    pub fn from_segment(a: Vec3, b: Vec3) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// Whether the two boxes share any volume, touching inclusive.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

/// Where a segment crosses a triangle, if it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentTriangleHit {
    /// Fraction along the segment, in `0.0..1.0`.
    pub t: f32,
    /// The triangle's **unnormalised** winding normal, `(b - a) x (c - a)`.
    ///
    /// Unnormalised because all three edge tests are performed against this same
    /// vector, and normalising first would only add rounding without changing a
    /// single sign.
    pub normal: Vec3,
}

/// Tests a segment against one triangle.
///
/// # Why this and not Moller-Trumbore
///
/// The original's `Collision_SegmentTriangle` (`0x08818bdc`, confidence 90) does
/// **a plane sign change on both segment endpoints plus three edge half-space
/// tests**, and that is what is reproduced here. The textbook Moller-Trumbore
/// test answers the same question but computes different intermediates in a
/// different order, so it disagrees on exactly the cases that matter for
/// comparison against a trace: a probe grazing a shared edge between two
/// triangles, or sitting almost exactly in a floor's plane. Rewriting it as
/// barycentric coordinates would be an optimisation of something that is not
/// slow, bought by giving up the one property this code exists to have.
///
/// # Degenerate cases, deliberately
///
/// The plane test requires a **strict** sign change, `d0 * d1 < 0.0`. So a
/// segment lying in the triangle's plane misses, and a segment with an endpoint
/// exactly on the plane misses. Both are deliberate: they are what makes
/// `t = d0 / (d0 - d1)` safe from `0.0 / 0.0`, and a degenerate or zero-length
/// segment cannot produce a `NaN` hit that then propagates into the ship's
/// position. A zero-area triangle has a zero normal, both distances come out
/// zero, and it misses for the same reason.
///
/// # Winding
///
/// The returned normal follows the vertex winding, and **which winding the
/// track data uses is not established** - the collision chunk format is decoded
/// (`docs/ghidra/functions/psp-pulse/collision.md`) but the handedness of its
/// triangles is not, and neither is the coordinate convention generally. The
/// hover path treats this normal as pointing out of the surface, so if the
/// parser turns out to emit the opposite winding, floors will report downward
/// normals and the correction belongs at the parser boundary rather than here.
/// A unit test pins the convention this crate assumes.
#[must_use]
pub fn segment_triangle(
    p0: Vec3,
    p1: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<SegmentTriangleHit> {
    let normal = (b - a).cross(c - a);

    let d0 = (p0 - a).dot(normal);
    let d1 = (p1 - a).dot(normal);
    // A **strict** sign change, and `NaN` rejected explicitly rather than by
    // accident: `NaN >= 0.0` is false, so without the second test a single
    // non-finite coordinate anywhere in the geometry would produce a `NaN` hit
    // point and teleport the ship.
    let product = d0 * d1;
    if product >= 0.0 || product.is_nan() {
        return None;
    }

    let t = d0 / (d0 - d1);
    let point = p0 + (p1 - p0) * t;

    // Three edge half-space tests against the same unnormalised normal. A point
    // inside the triangle is to the left of all three directed edges.
    if (b - a).cross(point - a).dot(normal) < 0.0 {
        return None;
    }
    if (c - b).cross(point - b).dot(normal) < 0.0 {
        return None;
    }
    if (a - c).cross(point - c).dot(normal) < 0.0 {
        return None;
    }

    Some(SegmentTriangleHit { t, normal })
}

/// One collider: indexed triangles, a surface tag and an index.
///
/// This mirrors the original's 0xa0-byte collider object without being a copy of
/// its layout: vertices, `u16` triangle indices widened to `u32`, the optional
/// per-vertex scalar chunk, the surface type and the owner index. Geometry
/// arrives as plain `Vec<[f32; 3]>` / `Vec<[u32; 3]>` / `Vec<f32>` on purpose -
/// the decoder that produces it lives in `oag-formats`, which this crate must
/// not depend on, so the conversion is the integration layer's job.
///
/// # No broadphase yet
///
/// [`Raycaster::raycast`] does a box reject and then a **linear scan over every
/// triangle**. That is correct, and it is not the bottleneck at the point this
/// crate exists; the original's per-mesh sweep and prune is a pure acceleration
/// structure over the same test.
///
/// Worth recording before anyone implements it, because it is a hard limit
/// inherited from the data rather than a design choice we would make: both
/// levels of the original's sweep and prune pack an endpoint into 22 bits as
/// `bits[0:11] = ((int)coord + 0x400) * 2` and `bits[12:21] = objectId`. That
/// **quantises coordinates to 1 unit over roughly +/-1024 and caps ids at 1024
/// per list**, while the triangle count field in the format is a `u16` and so
/// permits more. Reproducing the structure would reproduce the limit.
#[derive(Debug, Clone)]
pub struct TriangleSoup {
    vertices: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    vertex_scalars: Vec<f32>,
    surface: Surface,
    collider: u32,
    bounds: Aabb,
}

impl TriangleSoup {
    /// Builds a collider from plain indexed geometry.
    ///
    /// `vertex_scalars` is the format's chunk 3, which is **per vertex** and may
    /// be absent; pass an empty `Vec` for absent, and every hit then reports the
    /// documented fallback of `1.0`.
    ///
    /// Triangles whose indices fall outside `vertices` are **dropped**, so a
    /// malformed mesh cannot panic the simulation mid-race. That is a deliberate
    /// choice about where to fail: a decoder is the right place to reject bad
    /// geometry loudly, and a query at tick 40000 is the wrong place.
    #[must_use]
    pub fn new(
        vertices: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
        vertex_scalars: Vec<f32>,
        surface: Surface,
        collider: u32,
    ) -> Self {
        let vertices: Vec<Vec3> = vertices.into_iter().map(Vec3::from_array).collect();

        let count = vertices.len() as u32;
        let triangles: Vec<[u32; 3]> = triangles
            .into_iter()
            .filter(|tri| tri[0] < count && tri[1] < count && tri[2] < count)
            .collect();

        let mut bounds = Aabb::EMPTY;
        for vertex in &vertices {
            bounds.expand(*vertex);
        }

        Self {
            vertices,
            triangles,
            vertex_scalars,
            surface,
            collider,
            bounds,
        }
    }

    /// The surface tag every triangle in this collider carries.
    #[must_use]
    pub fn surface(&self) -> Surface {
        self.surface
    }

    /// This collider's index within its world.
    #[must_use]
    pub fn collider(&self) -> u32 {
        self.collider
    }

    /// How many triangles survived construction.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// One triangle's three world-space corners, or `None` if out of range.
    ///
    /// Mirrors `oag_formats::collision::CollisionMesh::triangle`. Exists so a
    /// test can aim a ship at a *real* piece of track geometry rather than at a
    /// hand-written plane - which is the only way to find out whether a shipped
    /// wall's winding, scale and normal survive the response law.
    #[must_use]
    pub fn triangle(&self, triangle: usize) -> Option<[Vec3; 3]> {
        let [a, b, c] = *self.triangles.get(triangle)?;
        Some([
            *self.vertices.get(a as usize)?,
            *self.vertices.get(b as usize)?,
            *self.vertices.get(c as usize)?,
        ])
    }

    /// The box around every vertex, or [`Aabb::EMPTY`] when there are none.
    #[must_use]
    pub fn bounds(&self) -> Aabb {
        self.bounds
    }

    /// Mean of a triangle's three per-vertex scalars.
    ///
    /// `1.0` when the mesh carries no chunk 3, which is what the original
    /// returns. The three-term sum is grouped left to right and divided once,
    /// rather than averaged pairwise, so the operation order is fixed.
    fn triangle_scalar(&self, tri: [u32; 3]) -> f32 {
        if self.vertex_scalars.is_empty() {
            return 1.0;
        }

        let at = |index: u32| -> f32 {
            self.vertex_scalars
                .get(index as usize)
                .copied()
                .unwrap_or(1.0)
        };

        (at(tri[0]) + at(tri[1]) + at(tri[2])) / 3.0
    }
}

impl Raycaster for TriangleSoup {
    fn raycast(&self, ray: Ray, skip: Option<u32>, include_reset: bool) -> Option<RaycastHit> {
        if skip == Some(self.collider) {
            return None;
        }
        if !include_reset && self.surface.is_skipped_by_default() {
            return None;
        }

        let p0 = ray.origin;
        let p1 = ray.end();

        let segment = Aabb::from_segment(p0, p1);
        if !segment.overlaps(&self.bounds) {
            return None;
        }

        let mut nearest: Option<(f32, RaycastHit)> = None;

        for tri in &self.triangles {
            let a = self.vertices[tri[0] as usize];
            let b = self.vertices[tri[1] as usize];
            let c = self.vertices[tri[2] as usize];

            // The same box reject as above, per triangle. The original reaches
            // this point through two slab rejects and a per-mesh sweep and
            // prune; both are acceleration only.
            let mut box_of_tri = Aabb::EMPTY;
            box_of_tri.expand(a);
            box_of_tri.expand(b);
            box_of_tri.expand(c);
            if !segment.overlaps(&box_of_tri) {
                continue;
            }

            let Some(hit) = segment_triangle(p0, p1, a, b, c) else {
                continue;
            };

            let nearer = match nearest {
                Some((t, _)) => hit.t < t,
                None => true,
            };
            if !nearer {
                continue;
            }

            nearest = Some((
                hit.t,
                RaycastHit {
                    point: p0 + (p1 - p0) * hit.t,
                    normal: hit.normal.normalize_or_zero(),
                    distance: hit.t * ray.length,
                    surface: self.surface,
                    vertex_scalar: self.triangle_scalar(*tri),
                    collider: self.collider,
                },
            ));
        }

        nearest.map(|(_, hit)| hit)
    }

    /// Every triangle the segment crosses, in **ascending triangle index**.
    ///
    /// The same order `Collision_BoxAgainstMesh` walks its candidates in, and
    /// the same traversal and rejects `raycast` uses - only the nearest-hit
    /// comparison is gone. Nothing is de-duplicated: two coplanar triangles both
    /// crossed by one segment report twice, which is what the original does.
    fn raycast_all(
        &self,
        ray: Ray,
        skip: Option<u32>,
        include_reset: bool,
        out: &mut [Option<RaycastHit>],
    ) -> usize {
        if skip == Some(self.collider) {
            return 0;
        }
        if !include_reset && self.surface.is_skipped_by_default() {
            return 0;
        }

        let p0 = ray.origin;
        let p1 = ray.end();

        let segment = Aabb::from_segment(p0, p1);
        if !segment.overlaps(&self.bounds) {
            return 0;
        }

        let mut found = 0;
        for tri in &self.triangles {
            if found == out.len() {
                break;
            }

            let a = self.vertices[tri[0] as usize];
            let b = self.vertices[tri[1] as usize];
            let c = self.vertices[tri[2] as usize];

            let mut box_of_tri = Aabb::EMPTY;
            box_of_tri.expand(a);
            box_of_tri.expand(b);
            box_of_tri.expand(c);
            if !segment.overlaps(&box_of_tri) {
                continue;
            }

            let Some(hit) = segment_triangle(p0, p1, a, b, c) else {
                continue;
            };

            out[found] = Some(RaycastHit {
                point: p0 + (p1 - p0) * hit.t,
                normal: hit.normal.normalize_or_zero(),
                distance: hit.t * ray.length,
                surface: self.surface,
                vertex_scalar: self.triangle_scalar(*tri),
                collider: self.collider,
            });
            found += 1;
        }

        found
    }
}

/// Every collider a segment query can reach.
///
/// Ordered storage, not a map: iteration order feeds the nearest-hit comparison
/// and so feeds simulation state, and a `HashMap`'s order is not stable even
/// between two runs of one binary. See `docs/architecture/determinism.md`.
#[derive(Debug, Clone, Default)]
pub struct CollisionWorld {
    colliders: Vec<TriangleSoup>,
}

impl CollisionWorld {
    /// An empty world, which every query misses.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a collider, keeping the index it was built with.
    pub fn push(&mut self, collider: TriangleSoup) {
        self.colliders.push(collider);
    }

    /// The colliders, in query order.
    #[must_use]
    pub fn colliders(&self) -> &[TriangleSoup] {
        &self.colliders
    }
}

impl Raycaster for CollisionWorld {
    /// Nearest hit across every collider.
    ///
    /// Self-skipping and the [`Surface::Reset`] exclusion are each collider's own
    /// business, so they are not repeated here; the original applies both at the
    /// broadphase, which is the same thing one level up.
    fn raycast(&self, ray: Ray, skip: Option<u32>, include_reset: bool) -> Option<RaycastHit> {
        let mut nearest: Option<RaycastHit> = None;

        for collider in &self.colliders {
            let Some(hit) = collider.raycast(ray, skip, include_reset) else {
                continue;
            };

            let nearer = match nearest {
                Some(best) => hit.distance < best.distance,
                None => true,
            };
            if nearer {
                nearest = Some(hit);
            }
        }

        nearest
    }

    /// Every hit across every collider, in **ascending collider index** and,
    /// within a collider, ascending triangle index.
    ///
    /// The same order [`Self::raycast`]'s nearest-hit comparison already walks,
    /// which `crates/gameplay`'s loader keeps stable for a given track.
    fn raycast_all(
        &self,
        ray: Ray,
        skip: Option<u32>,
        include_reset: bool,
        out: &mut [Option<RaycastHit>],
    ) -> usize {
        let mut found = 0;
        for collider in &self.colliders {
            if found == out.len() {
                break;
            }
            found += collider.raycast_all(ray, skip, include_reset, &mut out[found..]);
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single triangle in the `y = height` plane, wound so that its normal is
    /// `+Y` under this crate's assumption. Large enough that a probe near the
    /// origin is well inside it.
    fn floor(height: f32, surface: Surface, collider: u32) -> TriangleSoup {
        TriangleSoup::new(
            vec![
                [-100.0, height, -100.0],
                [-100.0, height, 100.0],
                [100.0, height, 0.0],
            ],
            vec![[0, 1, 2]],
            Vec::new(),
            surface,
            collider,
        )
    }

    #[test]
    fn a_downward_ray_hits_a_floor_below_it() {
        let soup = floor(0.0, Surface::Floor, 0);
        let hit = soup
            .raycast(
                Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0),
                None,
                false,
            )
            .expect("a ray straight down from above a floor hits it");

        assert_eq!(hit.point.y, 0.0);
        assert_eq!(hit.distance, 10.0);
        assert_eq!(hit.surface, Surface::Floor);
        assert_eq!(hit.collider, 0);
    }

    /// The winding convention this crate assumes, stated as a test because the
    /// parser that will feed it is being written separately and the handedness of
    /// the track data is not established.
    #[test]
    fn a_counter_clockwise_floor_seen_from_above_reports_an_upward_normal() {
        let soup = floor(0.0, Surface::Floor, 0);
        let hit = soup
            .raycast(
                Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0),
                None,
                false,
            )
            .expect("hit");

        assert!(hit.normal.y > 0.0, "normal was {:?}", hit.normal);
        assert!((hit.normal.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_ray_that_stops_short_of_the_floor_misses() {
        let soup = floor(0.0, Surface::Floor, 0);
        assert_eq!(
            soup.raycast(
                Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 5.0),
                None,
                false
            ),
            None
        );
    }

    /// The one degenerate case the force law will actually meet, because a probe
    /// can settle arbitrarily close to the surface it is pushing off.
    #[test]
    fn a_segment_lying_in_the_triangles_plane_misses_rather_than_returning_nan() {
        let soup = floor(0.0, Surface::Floor, 0);
        let hit = soup.raycast(
            Ray::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::X, 20.0),
            None,
            false,
        );
        assert_eq!(hit, None);
    }

    #[test]
    fn a_segment_with_an_endpoint_exactly_on_the_plane_misses() {
        // Starting on the plane and heading away, and starting above and ending
        // exactly on it, are both a strict-sign-change miss.
        assert_eq!(
            segment_triangle(
                Vec3::ZERO,
                Vec3::new(0.0, 5.0, 0.0),
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
            ),
            None
        );
        assert_eq!(
            segment_triangle(
                Vec3::new(0.0, 5.0, 0.0),
                Vec3::ZERO,
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(1.0, 0.0, 0.0),
            ),
            None
        );
    }

    #[test]
    fn a_zero_length_segment_misses_everything() {
        let soup = floor(0.0, Surface::Floor, 0);
        assert_eq!(
            soup.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 0.0), None, false),
            None
        );
    }

    #[test]
    fn a_zero_area_triangle_misses_rather_than_dividing_by_zero() {
        let hit = segment_triangle(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
        );
        assert_eq!(hit, None);
    }

    #[test]
    fn a_ray_outside_the_triangles_edges_misses() {
        let soup = floor(0.0, Surface::Floor, 0);
        assert_eq!(
            soup.raycast(
                Ray::new(Vec3::new(1000.0, 10.0, 0.0), Vec3::NEG_Y, 20.0),
                None,
                false
            ),
            None
        );
    }

    #[test]
    fn triangles_with_out_of_range_indices_are_dropped_at_construction() {
        let soup = TriangleSoup::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            vec![[0, 1, 2], [0, 1, 7]],
            Vec::new(),
            Surface::Floor,
            0,
        );
        assert_eq!(soup.triangle_count(), 1);
    }

    #[test]
    fn a_mesh_without_chunk_three_reports_a_vertex_scalar_of_one() {
        let soup = floor(0.0, Surface::Floor, 0);
        let hit = soup
            .raycast(
                Ray::new(Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Y, 10.0),
                None,
                false,
            )
            .expect("hit");
        assert_eq!(hit.vertex_scalar, 1.0);
    }

    #[test]
    fn a_mesh_with_chunk_three_reports_the_mean_of_three_vertices() {
        let soup = TriangleSoup::new(
            vec![[-1.0, 0.0, -1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
            vec![[0, 1, 2]],
            vec![0.0, 3.0, 6.0],
            Surface::Floor,
            0,
        );
        let hit = soup
            .raycast(
                Ray::new(Vec3::new(-0.5, 5.0, 0.0), Vec3::NEG_Y, 10.0),
                None,
                false,
            )
            .expect("hit");
        assert_eq!(hit.vertex_scalar, 3.0);
    }

    #[test]
    fn a_query_skips_the_collider_it_is_told_to_skip() {
        let soup = floor(0.0, Surface::Floor, 7);
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0);
        assert!(soup.raycast(ray, None, false).is_some());
        assert_eq!(soup.raycast(ray, Some(7), false), None);
        assert!(soup.raycast(ray, Some(6), false).is_some());
    }

    #[test]
    fn a_query_skips_reset_colliders_unless_asked_for_them() {
        let soup = floor(0.0, Surface::Reset, 0);
        let ray = Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 20.0);
        assert_eq!(soup.raycast(ray, None, false), None);
        assert!(soup.raycast(ray, None, true).is_some());
    }

    #[test]
    fn a_world_returns_the_nearest_of_several_colliders() {
        let mut world = CollisionWorld::new();
        world.push(floor(0.0, Surface::Floor, 0));
        world.push(floor(4.0, Surface::Floor, 1));
        world.push(floor(-4.0, Surface::Floor, 2));

        let hit = world
            .raycast(
                Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 30.0),
                None,
                false,
            )
            .expect("hit");
        assert_eq!(hit.collider, 1);
        assert_eq!(hit.point.y, 4.0);
    }

    #[test]
    fn a_world_query_can_skip_the_nearest_collider() {
        let mut world = CollisionWorld::new();
        world.push(floor(0.0, Surface::Floor, 0));
        world.push(floor(4.0, Surface::Floor, 1));

        let hit = world
            .raycast(
                Ray::new(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 30.0),
                Some(1),
                false,
            )
            .expect("hit");
        assert_eq!(hit.collider, 0);
    }

    #[test]
    fn an_empty_world_misses() {
        let world = CollisionWorld::new();
        assert_eq!(
            world.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 10.0), None, false),
            None
        );
    }

    #[test]
    fn a_mesh_with_no_vertices_has_a_box_that_overlaps_nothing() {
        let soup = TriangleSoup::new(Vec::new(), Vec::new(), Vec::new(), Surface::Floor, 0);
        assert!(
            !soup
                .bounds()
                .overlaps(&Aabb::from_segment(Vec3::ZERO, Vec3::ONE))
        );
        assert_eq!(
            soup.raycast(Ray::new(Vec3::ZERO, Vec3::NEG_Y, 10.0), None, false),
            None
        );
    }

    #[test]
    fn mag_floor_is_hoverable_and_wall_is_not() {
        assert!(Surface::Floor.is_hoverable());
        assert!(Surface::MagFloor.is_hoverable());
        assert!(!Surface::Wall.is_hoverable());
        assert!(!Surface::Reset.is_hoverable());
    }
}
