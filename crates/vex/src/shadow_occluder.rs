//! `Dynamic Shadow Occluder` `0x3c3`: the convex hull a craft or a pickup
//! casts its shadow from.
//!
//! The payload closes at `0x50 + 32n + 16m` on 129 of 129 nodes on the Pulse
//! PSP disc and 6 of 6 on Wipeout 2048, over at least twelve distinct
//! `(n, m)` pairs - see
//! [`shadows.md`](../../../docs/rendering/shadows.md) for the closure argument
//! and [`shadow-occluder.md`](../../../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md)
//! for `Shadow_RenderOccluderVolume` (`0x089038c8`), the runtime reader this
//! feeds.
//!
//! ```text
//! +0x00  u16   n, face records
//! +0x02  u16   m, vertex slots
//! +0x04  two stale PSP main-RAM pointers, some zeroed
//! +0x0c  bounding box, min then max, 3+3 f32
//! +0x24  u32   5, on 129/129
//! +0x28  f32   1.0, on 129/129
//! +0x30  the same box padded to two vec4s, with exceptions - see the docs
//! +0x50  n face records, 32 bytes each
//!        +0x00  unit plane normal, 3 f32
//!        +0x0c  u32  how many of the four slots below are used, 3 or 4
//!        +0x10  4 x u16  the face across each edge, `NO_NEIGHBOUR` for none
//!        +0x18  4 x u16  vertex indices, the fourth repeating the first on a
//!                        triangle
//! +0x50+32n  m vertex records, 16 bytes each: (w, x, y, z), w first, w == 1.0
//! ```
//!
//! # The two index arrays are what a silhouette needs, and they close exactly
//!
//! The 16 bytes at `+0x10` were the last unread field. Across all 129 nodes and
//! 4,381 faces on `pulse-psp-usa.chd`:
//!
//! - **Every vertex index is in range** (4,381 of 4,381).
//! - **Adjacency is reciprocal on 14,328 of 14,328 edges**: the face named
//!   across an edge owns that same edge. A wrong stride or field offset does not
//!   give a consistent edge graph on fourteen thousand edges.
//! - **Every `NO_NEIGHBOUR` edge is owned by no other face** (4,381 of 4,381).
//!   On 4,377 the only such edge is a triangle's degenerate fourth (`v0`->`v0`);
//!   the remaining four faces are two flat two-face hulls (`Data.wad#242`,
//!   `#244`) whose quads have genuinely open boundary edges.
//! - **Every triangle is coplanar with its declared plane** (3,184 of 3,184),
//!   its declared normal matching the normal of its vertices to within
//!   **0.028 degrees**.
//!
//! **Quads are not planar, and that is authored**: 300 of 1,197 spread past
//! `1e-4` of the hull's scale from their declared plane (worst `5.3e-2`),
//! bilinear quads out of an exporter. A sliver quad's first three vertices can
//! describe a normal 180 degrees from the declared one, so the normal check is
//! stated over triangles. The runtime tests against the declared normal, so
//! nothing here needs a flat quad.
//!
//! Confidence **92**: exact reciprocal-adjacency closure over 14,328 edges plus
//! an independent 0.028-degree agreement between two stored quantities, on one
//! disc. Not 95-100: no runtime trace of the reader exists, and the second
//! title's six hulls re-prove the *payload* closure rather than this indexing.
//!
//! # Two faces this parser carries rather than rejects
//!
//! Of 4,381 faces, one winds the other way about its declared normal
//! (`Data.wad#840` `shadowShape`, face 13 of 110) and one is degenerate
//! (`Data.wad#744` `shadowShape`, face 9 of 18). **Both are carried as
//! authored**: refusing them would refuse two whole hulls over two faces, and
//! the caller decides what a volume does with them ([`Face::winding`]).

use oag_formats::ByteOrder;

/// Length of the header before the first face record.
pub const HEADER_LEN: usize = 0x50;

/// Bytes per face record.
pub const FACE_LEN: usize = 32;

/// Bytes per vertex record.
pub const VERTEX_LEN: usize = 16;

/// The value an edge carries where no face lies across it.
///
/// A triangle's fourth edge always (`v0`->`v0`); a boundary edge on an open hull,
/// of which the Pulse disc has twelve.
pub const NO_NEIGHBOUR: u16 = 0xffff;

/// The greatest number of vertices one face record can name.
pub const MAX_FACE_VERTICES: usize = 4;

/// One face of the hull: its plane, the vertices it is wound from, and the
/// face across each of its edges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// The plane normal, a unit vector on 129/129 nodes. **The record's own, not
    /// derived from the vertices**: on a triangle they agree to 0.028 degrees; on
    /// a non-planar quad only this one is meaningful.
    pub normal: [f32; 3],
    /// How many of [`Self::vertices`] and [`Self::neighbours`] are used: 3 or
    /// 4 on every shipped face.
    pub count: u8,
    /// Vertex indices into [`Occluder::vertices`], wound counter-clockwise about
    /// [`Self::normal`]. A triangle's fourth slot repeats the first (3,184 of
    /// 3,184), so the array is a closed loop and the runtime walks four edges
    /// unconditionally.
    pub vertices: [u16; MAX_FACE_VERTICES],
    /// The face across each edge: edge `s` runs from `vertices[s]` to
    /// `vertices[s + 1]`, wrapping; [`NO_NEIGHBOUR`] where none.
    ///
    /// **The field a silhouette is built from**: an edge is on the silhouette
    /// when exactly one of its two faces faces the projection direction, one
    /// lookup instead of a search.
    pub neighbours: [u16; MAX_FACE_VERTICES],
}

impl Face {
    /// The vertex indices actually used, without the triangle's repeat.
    #[must_use]
    pub fn indices(&self) -> &[u16] {
        &self.vertices[..(self.count as usize).min(MAX_FACE_VERTICES)]
    }

    /// The face across edge `edge`, or `None` where there is none; `edge` is taken
    /// modulo the face's vertex count (the closed loop [`Self::vertices`] holds).
    #[must_use]
    pub fn neighbour(&self, edge: usize) -> Option<u16> {
        let used = (self.count as usize).min(MAX_FACE_VERTICES);
        let slot = self.neighbours[edge % used.max(1)];
        (slot != NO_NEIGHBOUR).then_some(slot)
    }

    /// `dot(cross(v1 - v0, v2 - v0), normal)`: positive where the winding agrees
    /// with the declared normal. Positive on 4,379 of the Pulse disc's 4,381
    /// faces, negative on one, zero on one (see the module docs).
    #[must_use]
    pub fn winding(&self, vertices: &[[f32; 3]]) -> f32 {
        let point = |slot: usize| {
            vertices
                .get(usize::from(self.vertices[slot]))
                .copied()
                .unwrap_or([0.0; 3])
        };
        let (a, b, c) = (point(0), point(1), point(2));
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        cross[0] * self.normal[0] + cross[1] * self.normal[1] + cross[2] * self.normal[2]
    }
}

/// One `Dynamic Shadow Occluder` payload, decoded.
#[derive(Debug, Clone, PartialEq)]
pub struct Occluder {
    /// The authored bounding box at `+0x0c`, min then max. **Authored, not
    /// derived**: `BEData.wad#20` declares a `y` maximum of the denormal
    /// `0x00800000`, which no extent computation produces.
    pub bounds: ([f32; 3], [f32; 3]),
    /// The hull's faces, in file order (the order [`Face::neighbours`] indexes).
    pub faces: Vec<Face>,
    /// The vertex slots, in file order. `w` is dropped (`1.0` on every record on
    /// both discs).
    ///
    /// **Slots, not vertices.** 150 across the Pulse disc sit unused at the
    /// origin; kept so an index means the same as in the file.
    pub vertices: Vec<[f32; 3]>,
}

impl Occluder {
    /// Decodes a payload, or `None` if it does not close at `0x50 + 32n + 16m`.
    ///
    /// The length check is the whole validation: it is the closure that
    /// identified the format, and every field inside is then carried as authored
    /// (see the two odd faces in the module docs).
    #[must_use]
    pub fn parse(payload: &[u8], order: ByteOrder) -> Option<Self> {
        if payload.len() < HEADER_LEN {
            return None;
        }
        // The closure check pinned the length exactly, so the panicking
        // `ByteOrder` accessors are safe below.
        let n = usize::from(order.u16(payload, 0));
        let m = usize::from(order.u16(payload, 2));
        if payload.len() != HEADER_LEN + n * FACE_LEN + m * VERTEX_LEN {
            return None;
        }

        let mut bounds = ([0.0f32; 3], [0.0f32; 3]);
        for axis in 0..3 {
            bounds.0[axis] = order.f32(payload, 0x0c + axis * 4);
            bounds.1[axis] = order.f32(payload, 0x18 + axis * 4);
        }

        let mut faces = Vec::with_capacity(n);
        for face in 0..n {
            let at = HEADER_LEN + face * FACE_LEN;
            let mut vertices = [0u16; MAX_FACE_VERTICES];
            let mut neighbours = [0u16; MAX_FACE_VERTICES];
            for slot in 0..MAX_FACE_VERTICES {
                neighbours[slot] = order.u16(payload, at + 0x10 + slot * 2);
                vertices[slot] = order.u16(payload, at + 0x18 + slot * 2);
            }
            faces.push(Face {
                normal: [
                    order.f32(payload, at),
                    order.f32(payload, at + 4),
                    order.f32(payload, at + 8),
                ],
                // Clamped, not refused: 3 or 4 on every shipped face, and a caller
                // reading `indices()` on anything else gets a walkable slice.
                count: order.u32(payload, at + 0x0c).min(MAX_FACE_VERTICES as u32) as u8,
                vertices,
                neighbours,
            });
        }

        let base = HEADER_LEN + n * FACE_LEN;
        let mut vertices = Vec::with_capacity(m);
        for vertex in 0..m {
            let at = base + vertex * VERTEX_LEN;
            vertices.push([
                order.f32(payload, at + 4),
                order.f32(payload, at + 8),
                order.f32(payload, at + 12),
            ]);
        }

        Some(Self {
            bounds,
            faces,
            vertices,
        })
    }

    /// The edges on the silhouette against `direction`: those where exactly one of
    /// the two faces meeting there faces it.
    ///
    /// Each is `(from, to)` as the *front* face winds it, so an extruded stencil
    /// volume is consistently wound. An edge with no face across it belongs
    /// whenever its own face is front-facing (an open hull has nothing to cancel
    /// it). **The test is against the record's declared normal**, as
    /// `Shadow_RenderOccluderVolume` does, so a non-planar quad costs nothing.
    /// Each edge is emitted once, from the front face's side.
    #[must_use]
    pub fn silhouette(&self, direction: [f32; 3]) -> Vec<(u16, u16)> {
        let faces_it = |face: &Face| {
            face.normal[0] * direction[0]
                + face.normal[1] * direction[1]
                + face.normal[2] * direction[2]
                > 0.0
        };
        let mut out = Vec::new();
        for face in &self.faces {
            if !faces_it(face) {
                continue;
            }
            let used = (face.count as usize).clamp(1, MAX_FACE_VERTICES);
            for edge in 0..used {
                let across = match face.neighbour(edge) {
                    // No face across it: nothing can cancel this edge.
                    None => None,
                    Some(other) => self.faces.get(usize::from(other)),
                };
                if across.is_some_and(faces_it) {
                    continue;
                }
                let from = face.vertices[edge];
                let to = face.vertices[(edge + 1) % used];
                if from != to {
                    out.push((from, to));
                }
            }
        }
        out
    }
}

/// One closed ring of the silhouette, as vertex indices in winding order. A
/// convex hull viewed from any direction has exactly one; the two open hulls on
/// the Pulse disc are why [`Loop::closed`] exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loop {
    /// The ring's vertices, first to last; the closing edge back to the first is
    /// implicit, present only when [`Self::closed`].
    pub vertices: Vec<u16>,
    /// Whether the walk returned to where it started. **Reported, not hidden**: an
    /// open chain drawn as a ring is a plausible-looking wrong shape.
    pub closed: bool,
}

impl Occluder {
    /// [`Self::silhouette`]'s edges, chained into rings, each walked `from` to `to`
    /// so it comes out in the front faces' winding, directly fannable into
    /// triangles. **Not a triangulation and not a projection**: the hull's own
    /// outline in its own space.
    #[must_use]
    pub fn silhouette_loops(&self, direction: [f32; 3]) -> Vec<Loop> {
        let edges = self.silhouette(direction);
        // Successors by `from` vertex: one outgoing edge on a convex hull's
        // silhouette, a `Vec` so a misbehaving hull gives short rings instead of
        // losing edges silently.
        let mut next: Vec<(u16, u16, bool)> =
            edges.iter().map(|(from, to)| (*from, *to, false)).collect();
        let mut out = Vec::new();
        for start in 0..next.len() {
            if next[start].2 {
                continue;
            }
            let first = next[start].0;
            let mut vertices = vec![first];
            let mut cursor = start;
            let mut closed = false;
            loop {
                next[cursor].2 = true;
                let to = next[cursor].1;
                if to == first {
                    closed = true;
                    break;
                }
                vertices.push(to);
                match next
                    .iter()
                    .position(|(from, _, used)| *from == to && !*used)
                {
                    Some(step) => cursor = step,
                    None => break,
                }
            }
            out.push(Loop { vertices, closed });
        }
        out
    }
}

impl Occluder {
    /// This hull with a 4x4 (row-major, as `vex` stores them) baked into it.
    ///
    /// **A `.vex` node's payload is in the space of the enclosing `Transform`
    /// nodes**, not the model's own (`vex::world_transforms` resolves it); a hull
    /// drawn without this is the right shape in the wrong frame, the failure that
    /// still *looks* like a shadow.
    ///
    /// Normals go through the matrix's rotation and are renormalised: exact for
    /// rigid and uniformly-scaled placements, wrong under non-uniform scale
    /// (where a plane needs the inverse-transpose). Nothing on either disc places
    /// an occluder that way; a caller that finds one gets a tilted normal, not a
    /// panic.
    #[must_use]
    pub fn placed(&self, matrix: &[f32; 16]) -> Self {
        let rotate = |v: [f32; 3]| {
            let out = [
                matrix[0] * v[0] + matrix[4] * v[1] + matrix[8] * v[2],
                matrix[1] * v[0] + matrix[5] * v[1] + matrix[9] * v[2],
                matrix[2] * v[0] + matrix[6] * v[1] + matrix[10] * v[2],
            ];
            let length = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
            if length > 1e-12 {
                [out[0] / length, out[1] / length, out[2] / length]
            } else {
                v
            }
        };
        let point = |v: [f32; 3]| {
            [
                matrix[0] * v[0] + matrix[4] * v[1] + matrix[8] * v[2] + matrix[12],
                matrix[1] * v[0] + matrix[5] * v[1] + matrix[9] * v[2] + matrix[13],
                matrix[2] * v[0] + matrix[6] * v[1] + matrix[10] * v[2] + matrix[14],
            ]
        };
        Self {
            bounds: (point(self.bounds.0), point(self.bounds.1)),
            faces: self
                .faces
                .iter()
                .map(|face| Face {
                    normal: rotate(face.normal),
                    ..*face
                })
                .collect(),
            // An unused slot stays at the origin: `m` counts slots, and moving
            // padding would put stray vertices inside the hull's box.
            vertices: self
                .vertices
                .iter()
                .map(|v| if *v == [0.0; 3] { *v } else { point(*v) })
                .collect(),
        }
    }

    /// The hull's outline against `direction`: [`Self::silhouette_loops`]' closed
    /// rings, a ring that is another traversed the other way dropped.
    ///
    /// **The dedup is a property of the data.** Two Pulse hulls carry a doubled
    /// shell (every face has an opposite-normal twin), so the silhouette is the
    /// same ring twice; filling both darkens the ground twice under alpha-over.
    /// Across the 119 local-space hulls, 117 give one ring, `Data.wad#840`
    /// `shadowShape` the doubled pair, and `Data.wad#597` `shadow_lodShape` two
    /// genuinely different lobes, both kept.
    ///
    /// An **open** chain is dropped rather than closed by force: inventing its
    /// closing edge is the plausible-looking wrong shape this module is written
    /// against. **That branch is a guard with no known input**: 0 open chains over
    /// 774 walks (129 hulls, six directions), and no fixture produces one.
    #[must_use]
    pub fn outline(&self, direction: [f32; 3]) -> Vec<Vec<u16>> {
        let mut out: Vec<Vec<u16>> = Vec::new();
        let mut seen: Vec<Vec<u16>> = Vec::new();
        for ring in self.silhouette_loops(direction) {
            if !ring.closed {
                continue;
            }
            let mut key = ring.vertices.clone();
            key.sort_unstable();
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            out.push(ring.vertices);
        }
        out
    }
}

#[cfg(test)]
mod tests;
