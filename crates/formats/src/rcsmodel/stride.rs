//! Recovering a `.rcsmodel`'s vertex stride, which is in no field of the file.
//!
//! **Four independent rules, and they agree wherever more than one answers.**
//! Split out of [`super`] as its own `impl Mesh` block because they are one
//! subject and the rest of the module is another: the container walks a
//! directory, and this decides how wide a vertex is before a byte of it can be
//! read.
//!
//! In the order a caller reaches for them:
//!
//! 1. [`Mesh::solve_stride`] - the authored bounding box a `.vex` `Mesh` node
//!    carries, and the tightest oracle there is.
//! 2. [`Mesh::solve_stride_by_layout`] - the file's own buffer packing, which
//!    needs no oracle at all.
//! 3. [`Mesh::solve_stride_by_normals`] - whether the `+6` word decodes to unit
//!    vectors, which is per-vertex evidence at every chunk size.
//! 4. [`Mesh::solve_stride_by_extent`] - the geometry's compactness, the last
//!    resort.
//!
//! [`Mesh::solve_stride_without_a_box`] is the chain 2, 3, 4 for a chunk no node
//! references - which on a circuit is most of them.

use super::{
    Bounds, Mesh, NORMAL_MARGIN_BAR, NORMAL_MIN_VERTICES, NORMAL_UNIT_BAR, POSITION_LEN, STRIDES,
    SubMesh,
};
use crate::ByteOrder;

impl Mesh {
    /// Recovers the vertex stride from the bounding box the `.vex` node
    /// authors, or `None` when no single stride explains it.
    ///
    /// # Why this is a search and not a field read
    ///
    /// **The stride is not in the file, as far as anyone has found.** The eight
    /// bytes at a descriptor's `+0x00` look like a vertex-format word and the
    /// byte at `+0x01` does vary with it - but it takes values `0x07` through
    /// `0x0d` against strides of 14, 18 and 22, and one value of it (`0x08`)
    /// appears with all three. No byte or `u16` anywhere in the 0x80-byte
    /// descriptor equals the stride on more than one of 38 solved submeshes.
    /// So this reads it out of the data by consequence instead.
    ///
    /// # Why the box is a sound oracle
    ///
    /// The `.vex` node carries an authored min/max pair whose `min <= max`
    /// holds on 1,638 of 1,638 nodes, and it is the mesh's *tight* box: with
    /// the true stride the dequantised points touch all six faces to within a
    /// quantisation step. A wrong stride reads position bits out of the
    /// attributes that follow, which lands outside the box almost immediately.
    ///
    /// **The test is tightness, not containment**, and the difference matters:
    /// requiring only "inside" admits a stride that is a divisor of the true
    /// one, which walks a subset of the vertices and stays inside by
    /// construction. Requiring the union to fill the box removed every
    /// ambiguity across all 89 meshes measured - 0 ambiguous, in three models
    /// including the largest circuit on the disc.
    ///
    /// `bounds` is the node's `(min, max)`. `tolerance` is how far a face may
    /// be missed by; two quantisation steps is what the callers use.
    #[must_use]
    pub fn solve_stride(&self, data: &[u8], bounds: Bounds, tolerance: f32) -> Option<usize> {
        let (min, max) = bounds;
        let populated = self.submeshes.iter().filter(|s| s.vertex_count > 0).count();
        let mut found = None;
        for &stride in STRIDES {
            let Some(((lo, hi), fitted)) = self.extent(data, stride, bounds) else {
                continue;
            };
            // **The majority rule.** `extent` skips a submesh that does not fit,
            // so without this a stride could qualify by fitting one submesh out
            // of nineteen and skipping the rest. Half is enough: the case this
            // exists for is one stray submesh, not a coin toss.
            if fitted * 2 < populated {
                continue;
            }
            let tight = (0..3).all(|i| {
                (lo[i] - min[i]).abs() <= tolerance && (hi[i] - max[i]).abs() <= tolerance
            });
            if !tight {
                continue;
            }
            if found.is_some() {
                // Two strides explain the same box. Never seen on the disc, and
                // reported as unknown rather than resolved by preference.
                return None;
            }
            found = Some(stride);
        }
        found
    }

    /// Recovers the vertex stride with no bounding box to check against, by
    /// taking the one whose decoded positions are most compact.
    ///
    /// # Why a circuit needs this and a craft does not
    ///
    /// **Wipeout HD's road is not in the `.vex`.** All 126 `Mesh` nodes of
    /// `talons_junction/track.vex` are props - blimps, girders, sky traffic -
    /// and the circuit itself is among the **904 of 983** chunks no node
    /// references at all, drawn from the visibility set instead. Those chunks
    /// have no authored box, so [`Self::solve_stride`] has nothing to ask.
    ///
    /// # Why the most compact reading is the right one
    ///
    /// A position is a quantised `i16`; the attribute bytes after it are
    /// normalised across the whole `i16` range. So a wrong stride reads
    /// attributes as positions and spreads them over the full +/-32768 - two
    /// orders of magnitude wider than a real mesh, which occupies a tile.
    /// Taking the minimum is therefore not a heuristic dressed as a rule, it is
    /// reading the one interpretation that is not noise.
    ///
    /// **Checked against the oracle it replaces**: on the 78 meshes of Assegai
    /// and Talon's Junction where an authored box settles the stride, this
    /// picks the same value on **77**. The one disagreement is a mesh where the
    /// box admitted 36 and this picks 18 - half of it, so the box was matching
    /// every second vertex and this is the better answer rather than a worse
    /// one.
    ///
    /// # The winner has to be decisive
    ///
    /// Smallest-wins alone is not enough: on a circuit the three widths often
    /// produce spans within a few per cent of each other, and picking one of
    /// those by a hair decodes to spikes radiating out of the level. So the
    /// winner must be **at most half** the runner-up - a relative test with no
    /// threshold to tune, and the reading either stands out from the noise or
    /// there is no answer.
    ///
    /// Validated on the same oracle: all **70 of 70** of `talons_junction`'s
    /// box-labelled chunks clear it, the worst at 0.35 and the median at 0.02.
    /// It keeps 718 of the circuit's 983 chunks; the rest draw nothing.
    ///
    /// `None` for a chunk with no readable vertex buffer, or none whose reading
    /// stands out.
    #[must_use]
    pub fn solve_stride_by_extent(&self, data: &[u8]) -> Option<usize> {
        let mut spans: Vec<(usize, i32)> = STRIDES
            .iter()
            .filter_map(|&stride| Some((stride, self.quantised_span(data, stride, None)?)))
            .collect();
        spans.sort_by_key(|&(_, span)| span);
        let [(stride, best), (_, runner_up), ..] = spans[..] else {
            return None;
        };
        (i64::from(best) * 2 <= i64::from(runner_up)).then_some(stride)
    }

    /// Recovers the vertex stride from where the file **puts** its buffers,
    /// rather than from what they decode to.
    ///
    /// A mesh's vertex buffers are packed back to back, so the distance from one
    /// submesh's buffer to the next one's, over the first one's vertex count, is
    /// the stride - arithmetic on two numbers the file states outright, with no
    /// oracle and nothing decoded. Every consecutive pair votes and the majority
    /// wins.
    ///
    /// # Why this is the rule to prefer
    ///
    /// It is **structural where the other two are not**. [`Self::solve_stride`]
    /// needs a box the `.vex` authors, and [`Self::solve_stride_by_extent`] is a
    /// statistic about what the bytes look like once read. This is neither: it
    /// is the layout the exporter wrote.
    ///
    /// Measured against both:
    ///
    /// - **18 of 18 agreement with the authored box**, 0 disagreements, over
    ///   every mesh of Assegai and Talon's Junction that has a box and more than
    ///   one submesh.
    /// - **95 of 95 agreement with the compactness rule** on `talons_junction`'s
    ///   chunks where both decide. Two independent rules - one about layout, one
    ///   about content - agreeing exactly is a far better argument for the
    ///   compactness rule than the compactness rule can make for itself.
    /// - It decides **16 chunks the compactness rule cannot**, and the
    ///   compactness rule decides 623 this one cannot: a chunk with a single
    ///   submesh has no step to measure, and most of a circuit's chunks are
    ///   single-submesh. They are complements, not alternatives.
    ///
    /// # The rounding, and what is unexplained
    ///
    /// A step is usually exactly `vertex_count * stride`, but not always: the
    /// residual `step - vertex_count * stride` is 0 on 38 of 46 measured pairs
    /// and otherwise -16, -32 or -96 - always negative, always a multiple of 16.
    /// **Why the next buffer starts before the previous one's declared length
    /// ends is unrecovered**, and this rounds rather than modelling it, because a
    /// correction nobody can justify is worse than a rounding everybody can see.
    /// The error is at most 96 bytes over at least 33 vertices, well inside the
    /// 2-byte gaps between the three widths.
    ///
    /// `None` for a mesh with fewer than two submeshes, or one whose pairs do
    /// not agree on a width [`STRIDES`] carries.
    #[must_use]
    pub fn solve_stride_by_layout(&self) -> Option<usize> {
        let mut by_offset: Vec<&SubMesh> = self.submeshes.iter().collect();
        by_offset.sort_unstable_by_key(|sub| sub.vertex_offset);

        let mut votes = vec![0usize; STRIDES.len()];
        for pair in by_offset.windows(2) {
            let [first, next] = pair else { continue };
            if first.vertex_count == 0 || next.vertex_offset <= first.vertex_offset {
                continue;
            }
            let step = next.vertex_offset - first.vertex_offset;
            // Round to nearest without leaving integer arithmetic: the +/- 96
            // residual above means the quotient is not exact.
            let stride = (2 * step + first.vertex_count) / (2 * first.vertex_count);
            if let Some(slot) = STRIDES.iter().position(|&s| s == stride) {
                votes[slot] += 1;
            }
        }
        let (slot, &best) = votes.iter().enumerate().max_by_key(|&(_, n)| n)?;
        if best == 0 {
            return None;
        }
        // A tie is two readings with equal support, and there is no principle
        // here that breaks one - so it is unknown, the same answer the other two
        // rules give when nothing stands out.
        let tied = votes.iter().filter(|&&n| n == best).count() > 1;
        (!tied).then(|| STRIDES[slot])
    }

    /// Recovers the vertex stride from whether the **normals decode**.
    ///
    /// A vertex's `+6` word is a packed unit vector - see [`unpack_normal`] -
    /// and that is a property nothing else in the record has: read the same
    /// four bytes at the wrong stride and they are a position, a texture
    /// coordinate or the tail of a previous vertex, and they come out unit
    /// about a third of the time by chance. So the stride is the one whose
    /// normals are unit, and the wrong ones lose by a wide margin.
    ///
    /// # Why this is the rule that filled in the road
    ///
    /// It is the only rule here that is **per-vertex evidence at every chunk
    /// size**. The authored box needs a `.vex` node, the buffer layout needs
    /// two submeshes, and the compactness rule needs the true reading to be
    /// decisively smaller than the wrong one - a margin that narrows on exactly
    /// the large chunks a circuit's road is made of. Talon's Junction had
    /// **252 of its 983 chunks** left undrawn by the other three, which is what
    /// the holes in the floor were.
    ///
    /// Measured on that circuit: this decides **968 of 983** where the other
    /// rules together decide 731, rescues **239** of the 252, and **disagrees
    /// with them on none** of the 729 chunks where both answer.
    ///
    /// # The two bars, and why they are where they are
    ///
    /// The winner must be unit on at least [`NORMAL_UNIT_BAR`] of the chunk's
    /// vertices and beat the runner-up by [`NORMAL_MARGIN_BAR`]. Both sit below
    /// the cluster the real answers form and above the ~0.3 a wrong stride
    /// scores by chance; dropping them to 0.7 and 0.2 decides only 3 more
    /// chunks, so this is a knee rather than a tuned threshold. A chunk that
    /// clears neither is reported as undecided and drawn as nothing.
    #[must_use]
    pub fn solve_stride_by_normals(&self, data: &[u8]) -> Option<usize> {
        let mut scores: Vec<(usize, f32)> = STRIDES
            .iter()
            .filter_map(|&stride| Some((stride, self.unit_normal_fraction(data, stride)?)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let &[(stride, best), ..] = &scores[..] else {
            return None;
        };
        let runner_up = scores.get(1).map_or(0.0, |&(_, f)| f);
        (best >= NORMAL_UNIT_BAR && best - runner_up >= NORMAL_MARGIN_BAR).then_some(stride)
    }

    /// How much of a chunk decodes to a unit normal at `stride`, or `None` if
    /// nothing could be read.
    ///
    /// An all-zero word is an unused vertex rather than a misdecode - see
    /// [`Self::normals`] - so it counts towards neither side.
    pub(super) fn unit_normal_fraction(&self, data: &[u8], stride: usize) -> Option<f32> {
        let (mut unit, mut total) = (0usize, 0usize);
        #[allow(clippy::items_after_statements)]
        for submesh in &self.submeshes {
            let Ok(normals) = self.normals(data, submesh, stride) else {
                continue;
            };
            for n in &normals {
                let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if length == 0.0 {
                    continue;
                }
                total += 1;
                if (0.93..=1.07).contains(&length) {
                    unit += 1;
                }
            }
        }
        (total >= NORMAL_MIN_VERTICES).then(|| unit as f32 / total as f32)
    }

    /// The stride of a chunk **no `.vex` node references**, which is most of a
    /// circuit.
    ///
    /// Asks [`Self::solve_stride_by_layout`] first because it is structural, and
    /// falls back to [`Self::solve_stride_by_extent`] because the layout rule
    /// says nothing at all about a single-submesh chunk. On `talons_junction`
    /// the two together decide **734 of 983** chunks where the compactness rule
    /// alone decided 718, and they never disagree - 95 of 95 where both answer.
    #[must_use]
    pub fn solve_stride_without_a_box(&self, data: &[u8]) -> Option<usize> {
        self.solve_stride_by_layout()
            .or_else(|| self.solve_stride_by_normals(data))
            .or_else(|| self.solve_stride_by_extent(data))
    }

    /// The widest axis span of the raw `i16` positions at `stride`, or `None`
    /// if no submesh could be read or the span passed `ceiling`.
    ///
    /// Measured before the bias and scale are applied, so it compares strides
    /// on one chunk without a multiply per vertex. `ceiling` stops a read that
    /// has already exceeded a span the caller has no use for.
    pub(super) fn quantised_span(
        &self,
        data: &[u8],
        stride: usize,
        ceiling: Option<i32>,
    ) -> Option<i32> {
        let mut lo = [i32::MAX; 3];
        let mut hi = [i32::MIN; 3];
        let mut any = false;
        for submesh in &self.submeshes {
            if submesh.vertex_count == 0 {
                continue;
            }
            let end = submesh.vertex_offset + stride * (submesh.vertex_count - 1) + POSITION_LEN;
            if end > data.len() {
                continue;
            }
            any = true;
            for k in 0..submesh.vertex_count {
                let at = submesh.vertex_offset + k * stride;
                for i in 0..3 {
                    let v = i32::from(ByteOrder::Big.i16(data, at + i * 2));
                    lo[i] = lo[i].min(v);
                    hi[i] = hi[i].max(v);
                }
                if let Some(ceiling) = ceiling
                    && (0..3).any(|i| hi[i] - lo[i] >= ceiling)
                {
                    return None;
                }
            }
        }
        any.then(|| (0..3).map(|i| hi[i] - lo[i]).max().unwrap_or(0))
    }

    /// Whether one submesh's positions all land inside `bounds` at `stride`.
    ///
    /// **A caller that draws has to ask this too, not just [`solve_stride`].**
    /// The stride search tolerates a submesh that does not fit - see
    /// [`Mesh::extent`] - so the stride it returns is right for the mesh and
    /// wrong for that one submesh, whose positions then come out of the
    /// attribute bytes and scatter across the world. Assegai's hull has exactly
    /// one such submesh and it stretched the ship's own bounding sphere from 7
    /// units to 130, which framed the craft as a speck.
    ///
    /// [`solve_stride`]: Mesh::solve_stride
    #[must_use]
    pub fn submesh_fits(
        &self,
        data: &[u8],
        submesh: &SubMesh,
        stride: usize,
        bounds: Bounds,
    ) -> bool {
        let (min, max) = bounds;
        let slack = 1e-2;
        self.positions(data, submesh, stride).is_ok_and(|points| {
            points
                .iter()
                .all(|p| (0..3).all(|i| p[i] >= min[i] - slack && p[i] <= max[i] + slack))
        })
    }

    /// The box the submeshes that *fit* inside `bounds` at `stride` occupy, and
    /// how many of them there were.
    ///
    /// # A submesh that does not fit is skipped, not fatal
    ///
    /// **Measured, and it is the difference between drawing a craft and drawing
    /// its airbrakes.** Assegai's hull is one `Mesh` node of 19 submeshes; 18 of
    /// them fit at stride 22 and exactly one - `sub13` - fits at no stride at
    /// all. Requiring every submesh to fit therefore rejected 22 for the whole
    /// node and the hull vanished, while the 18 that do fit reconstruct it
    /// exactly. Why that one submesh reads differently is unrecovered.
    ///
    /// The guard against a stride surviving by skipping almost everything is
    /// [`Mesh::solve_stride`]'s majority rule, which is why the count comes back
    /// with the box rather than being swallowed here.
    pub(super) fn extent(
        &self,
        data: &[u8],
        stride: usize,
        bounds: Bounds,
    ) -> Option<(Bounds, usize)> {
        let (min, max) = bounds;
        // A hair of slack, because the authored box is stored as `f32` and the
        // positions are reconstructed from a quantised integer.
        let slack = 1e-2;
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut fitted = 0;
        for submesh in &self.submeshes {
            if submesh.vertex_count == 0 {
                continue;
            }
            let Ok(points) = self.positions(data, submesh, stride) else {
                continue;
            };
            // Bails on the first stray point, which is what keeps the search
            // over `STRIDES` cheap: a wrong stride usually leaves the box within
            // a few vertices.
            if points
                .iter()
                .any(|p| (0..3).any(|i| p[i] < min[i] - slack || p[i] > max[i] + slack))
            {
                continue;
            }
            fitted += 1;
            for point in points {
                for i in 0..3 {
                    lo[i] = lo[i].min(point[i]);
                    hi[i] = hi[i].max(point[i]);
                }
            }
        }
        (fitted > 0).then_some(((lo, hi), fitted))
    }
}
