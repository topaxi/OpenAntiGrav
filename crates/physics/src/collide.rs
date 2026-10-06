//! The collision query surface the ship dynamics are written against.
//!
//! The types are pinned so the geometry that answers them
//! ([`oag_vex::collision`](../../../formats/src/collision.rs)) and the code that asks
//! ([`crate::ship`]) need not wait for each other. A [`Raycaster`] answers "what does this
//! segment hit": in tests a hand-written triangle, in a race the track's collision soup.
//! Evidence: `docs/ghidra/functions/psp-pulse-usa/collision.md`.

use oag_core::math::Vec3;

/// What kind of surface a collider is.
///
/// All five collision node classes in the original share one vtable; the class ID selects
/// only this enum and a friction constant, so a magstrip is ordinary floor with a different
/// tag. Confidence 92; `collision.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Surface {
    /// `Wall Collision`, class `0x3ba`.
    Wall = 0,
    /// `Floor Collision`, class `0x3b9`. Accepted by the hover probes.
    Floor = 1,
    /// `Reset Collision`, class `0x3cd`. Excluded from ordinary raycasts; touching one
    /// respawns the ship.
    Reset = 2,
    /// `Mag Floor Collision`, class `0x3e6`. Identical to [`Self::Floor`] at load apart from
    /// the tag; the hover paths accept both.
    MagFloor = 3,
}

impl Surface {
    /// Whether the hover probes accept contact with this surface: types 1 and 3 only, the
    /// whole of the original's hover type test.
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
    /// The class ID selects a surface type and this one float at `collider+0x64`. The file's
    /// `-1.0` for floors is a **sentinel, not a coefficient**: `Collision_AddContact`
    /// (`0x08816864`) forces the combined value to zero when either side is negative, so
    /// `None` keeps that distinction (`docs/formats/collision.md`).
    ///
    /// This field was once read as restitution. `Body_ResolveContact` (`0x0884e968`) takes
    /// restitution from `body+0x388` and uses `contact+0x34` (this value) only to scale the
    /// *tangential* relative velocity, so the disc's `0.05` is a friction coefficient and the
    /// whole of a scraping craft's sustained speed loss (`contact-response.md`).
    ///
    /// Duplicates `oag_vex::collision::SurfaceKind::friction` because this crate must not
    /// depend on `oag-formats`; `oag_gameplay::collision` tests that the two agree.
    #[must_use]
    pub fn friction(self) -> Option<f32> {
        match self {
            Self::Wall => Some(WALL_FRICTION),
            Self::Floor | Self::Reset | Self::MagFloor => None,
        }
    }
}

/// Friction of a [`Surface::Wall`], from `collider+0x64`: the only non-sentinel value in the
/// file.

/// Friction of a contact between two colliders.
///
/// `Collision_AddContact` (`0x08816864`) averages the two values but forces zero if
/// **either** is negative, so a floor against a wall is `0.0`, not the average of `-1.0` and
/// `0.05`.
#[must_use]
pub fn combine_friction(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => (a + b) * 0.5,
        _ => 0.0,
    }
}

/// What a segment query found.
///
/// Mirrors the original's 0x2c-byte hit result (point, normal, collider index, averaged
/// per-vertex scalar, hit flag, here `Option`'s job). `distance` is ours: every caller needs
/// it and recomputing from `point` loses precision.
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
    /// Mean of the hit triangle's three per-vertex scalars, or `1.0` when the mesh carries no
    /// chunk 3. **Its meaning is not known** ("grip, friction or roughness" is a guess at
    /// confidence 40), so it is carried untouched rather than named.
    pub vertex_scalar: f32,
    /// Index of the collider within the world, for self-skipping.
    pub collider: u32,
    /// The hit triangle's three vertices in stored winding, or `None` from a raycaster with
    /// no triangles (an infinite test plane). For [`crate::wall`]'s contact gate:
    /// `Collision_AddContact` takes a contact only when the sample point's perpendicular
    /// projection lands inside the **same** triangle the centre segment crossed.
    pub triangle: Option<[Vec3; 3]>,
}

/// Answers segment queries against collision geometry.
///
/// A trait so the dynamics can be tested against one hand-placed triangle with no assets,
/// disc image or GPU.
pub trait Raycaster {
    /// Casts from `origin` along `direction` (expected normalised) for `length` units and
    /// returns the **nearest** hit. Implementations must skip [`Surface::Reset`] unless
    /// `include_reset`, and the collider whose index is `skip` (how the ship avoids itself).
    fn raycast(&self, ray: Ray, skip: Option<u32>, include_reset: bool) -> Option<RaycastHit>;

    /// **Every** hit along the segment, not just the nearest.
    ///
    /// Writes into `out`, returning how many slots it filled (at most `out.len()`). Hits past
    /// that are **dropped rather than substituted**, so the set never depends on how full the
    /// buffer got. Same `skip` and `include_reset` semantics as [`Self::raycast`].
    ///
    /// Order is each implementation's storage order (ascending triangle index within a mesh,
    /// ascending collider index across a world), never a distance sort: the consumer resolves
    /// contacts in received order and each reads the body state the previous one left, so the
    /// order feeds simulation state and must not depend on a float comparison
    /// (`docs/architecture/determinism.md`).
    ///
    /// The hull contact path needs it because `Collision_BoxAgainstMesh` (`0x08815cd4`) tests
    /// every sample point against **every** candidate triangle without de-duplication, so a
    /// point wedged in a corner is scrubbed twice and a floor triangle in front of a wall
    /// does not hide it (`collision.md#contact-generation`).
    ///
    /// **The default implementation is a trap**: it reports only the nearest hit, so a
    /// `Raycaster` that has not overridden this (a test double) yields one contact per probe.
    /// Right for a query-counting double, wrong to debug for an hour.
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

/// An axis-aligned bounding box. The original stores one per collider (`+0x00`/`+0x10`) and
/// one per triangle in the per-mesh sweep-and-prune list; here it is only the reject pass in
/// front of the narrowphase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    /// Lower corner.
    pub min: Vec3,
    /// Upper corner.
    pub max: Vec3,
}

impl Aabb {
    /// A box that contains nothing and overlaps nothing: `min` above `max`, so
    /// [`Self::overlaps`] rejects it without an emptiness flag.
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
    /// The triangle's **unnormalised** winding normal, `(b - a) x (c - a)`: all three edge
    /// tests use this vector, and normalising first only adds rounding.
}

/// Tests a segment against one triangle.
///
/// # Why this and not Moller-Trumbore
///
/// The original's `Collision_SegmentTriangle` (`0x08818bdc`, confidence 90) does **a plane
/// sign change on both endpoints plus three edge half-space tests**, reproduced here.
/// Moller-Trumbore answers the same question with different intermediates in a different
/// order, so it disagrees exactly where comparison against a trace matters: a probe grazing a
/// shared edge, or lying almost in a floor's plane. Barycentric coordinates would optimise
/// something not slow by giving up the one property this code exists to have.
///
/// # Degenerate cases, deliberately
///
/// The plane test needs a **strict** sign change, `d0 * d1 < 0.0`, so a segment in the
/// triangle's plane or with an endpoint exactly on it misses. That keeps
/// `t = d0 / (d0 - d1)` safe from `0.0 / 0.0` and a zero-length segment from producing a
/// `NaN` hit that propagates into the ship's position. A zero-area triangle has a zero normal
/// and misses for the same reason.
///
/// # Winding
///
/// The normal follows the vertex winding, and **which winding the track data uses is not
/// established** (the chunk format is decoded, `collision.md`, but not its handedness). The
/// hover path treats this normal as pointing out of the surface, so if the parser emits the
/// opposite winding the correction belongs at the parser boundary. A unit test pins the
/// convention assumed.
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
    // A **strict** sign change, with `NaN` rejected explicitly: `NaN >= 0.0` is false, so a
    // single non-finite coordinate would otherwise produce a `NaN` hit and teleport the ship.
    let product = d0 * d1;
    if product >= 0.0 || product.is_nan() {
        return None;
    }

    let t = d0 / (d0 - d1);
    let point = p0 + (p1 - p0) * t;

    // Three edge half-space tests against the same unnormalised normal: inside is left of all
    // three directed edges.
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
/// Mirrors the original's 0xa0-byte collider object without copying its layout. Geometry
/// arrives as plain `Vec`s because the decoder lives in `oag-formats`, which this crate must
/// not depend on; conversion is the integration layer's job.
///
/// # No broadphase yet
///
/// [`Raycaster::raycast`] does a box reject then a **linear scan over every triangle**:
/// correct, and not the bottleneck. A hard limit inherited from the data, worth knowing before
/// anyone implements it: both levels of the original's sweep and prune pack an endpoint into
/// 22 bits (`bits[0:11] = ((int)coord + 0x400) * 2`, `bits[12:21] = objectId`), which
/// **quantises coordinates to 1 unit over roughly +/-1024 and caps ids at 1024 per list**,
/// though the format's `u16` triangle count permits more.
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
    /// `vertex_scalars` is the format's chunk 3, **per vertex** and possibly absent (pass an
    /// empty `Vec`; every hit then reports `1.0`). Triangles with indices outside `vertices`
    /// are **dropped**, so a malformed mesh cannot panic a race: the decoder is the place to
    /// reject bad geometry loudly, not a query at tick 40000.
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

    /// One triangle's three world-space corners, or `None` if out of range. Mirrors
    /// `oag_vex::collision::CollisionMesh::triangle`; lets a test aim a ship at *real* track
    /// geometry to see whether a shipped wall's winding, scale and normal survive the
    /// response law.
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

    /// Mean of a triangle's three per-vertex scalars, `1.0` when the mesh carries no chunk 3
    /// (as the original). Summed left to right and divided once, so the operation order is
    /// fixed.
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

            // The same box reject per triangle. The original reaches this through two slab
            // rejects and a per-mesh sweep and prune; both are acceleration only.
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
                    triangle: Some([a, b, c]),
                },
            ));
        }

        nearest.map(|(_, hit)| hit)
    }

    /// Every triangle the segment crosses, in **ascending triangle index**: the order
    /// `Collision_BoxAgainstMesh` walks its candidates, with `raycast`'s traversal and rejects
    /// minus the nearest-hit comparison. Nothing is de-duplicated: two coplanar triangles both
    /// crossed report twice, as in the original.
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
                triangle: Some([a, b, c]),
            });
            found += 1;
        }

        found
    }
}

/// Every collider a segment query can reach.
///
/// Ordered storage, not a map: iteration order feeds the nearest-hit comparison and so
/// simulation state, and a `HashMap`'s order is unstable even between runs
/// (`docs/architecture/determinism.md`).
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
    /// Nearest hit across every collider. Self-skipping and the [`Surface::Reset`] exclusion
    /// are each collider's business; the original applies both at the broadphase, the same
    /// thing one level up.
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

    /// Every hit across every collider, in **ascending collider index** and, within one,
    /// ascending triangle index: the order [`Self::raycast`] already walks, which
    /// `crates/gameplay`'s loader keeps stable for a track.
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
mod tests;
