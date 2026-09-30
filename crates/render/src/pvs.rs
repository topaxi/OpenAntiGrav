//! Placing render geometry into the track's authored visibility sections.
//!
//! # The rule: a draw call belongs to its authored section
//!
//! The association is on the disc after all: **a `section` node governs its
//! parent's whole subtree** - a track is authored as sibling groups, one
//! transform per group with the `section` as one child and the group's
//! geometry as the rest. [`oag_vex::pvs::governing_sections`] derives it;
//! each draw call carries its source node
//! ([`crate::mesh::DrawCall::node`]), so its mask is the single bit of its
//! group's section. A draw call whose node no section governs - or that has
//! no node at all - gets [`ALWAYS`] and draws every frame, because the error
//! has to point towards drawing too much.
//!
//! **Placement is structural, not spatial, and that is the finding rather
//! than a convenience.** This module used to intersect each draw call's
//! bounding sphere with every authored box and give it the mask of all
//! sections it touched. That is the natural rule if the association is
//! unknown, and it is *incapable of hiding what the artists hid*: Moa Therma
//! ships a coarse far-LOD copy of its track in a group whose section only
//! four distant vantage sections list in their masks, and the copy occupies
//! the same world-space boxes as the sections the craft races through.
//! Spatial placement put it in the racing sections, it drew coincident with
//! the detailed track, and the z-fight chopped the magstrip's painted lines
//! into sideways-stepping segments - the artifact
//! `crates/render/tests/magstrip_ground_truth.rs` pins down. Authored
//! placement hides it exactly as the original does.
//!
//! **The still-cheaper rule was also wrong, and first.** Placing a draw call
//! in the single section containing its bounding-sphere *centre* lost
//! background scenery: a ridge line spans a dozen sections, its centre lands
//! in one, and the mesh vanished whenever that one left the set - 212,808
//! differing bytes in a screenshot comparison. Under authored placement a
//! big mesh again has one section, but now it is the one the artists placed
//! it in, and the *masks* were authored against that same assignment - which
//! is why one bit is correct here and was wrong when the bit was guessed
//! from geometry. See
//! `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md` and
//! its successor for the full history.
//!
//! Packaged the way `[graphics] frustum_culling` is - config-file only, no
//! menu row - and defaults on.
//!
//! # What this does not do yet
//!
//! Draw calls are tested one at a time. With a single section per draw call,
//! sorting the lists by section at load and walking only the visible ranges
//! became possible in principle; the transparent list is still drawn in
//! authored order, where blending makes reordering change the picture.

use oag_core::math::Vec3;
use oag_core::math::frustum::Frustum;
use oag_vex::pvs::{ALL_VISIBLE, MAX_SECTIONS, TrackPvs};
use oag_vex::track::AiTrack;

use crate::mesh::{DrawCall, Model};

/// The per-draw-call mask for geometry no authored box touches.
///
/// All ones, so it shares a bit with any non-empty visible set and is therefore
/// always drawn. That is the direction the error has to point: an unplaced
/// batch costs a frustum test, a wrongly-excluded one disappears from the shot.
pub const ALWAYS: u64 = u64::MAX;

/// A section id no track declares, meaning "do not trust this position".
///
/// Outside the `0..64` a mask can address on purpose: the value is not a
/// section, and [`oag_vex::pvs::TrackPvs::visible_from`] answers
/// [`ALL_VISIBLE`] for it, which is what turns an unlocatable craft or camera
/// into *draw everything*.
pub const UNPLACED: u8 = u8::MAX;

/// Which sections each of a [`Model`]'s draw calls touches.
///
/// Three vectors rather than one, index-parallel to [`Model::draws`],
/// [`Model::alpha_tested_draws`] and [`Model::transparent_draws`], so a lookup
/// during the draw loop is an index into a `Vec<u64>` beside the iteration
/// already happening.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawSections {
    pub opaque: Vec<u64>,
    pub alpha_tested: Vec<u64>,
    pub transparent: Vec<u64>,
}

/// What placing one model's draw calls achieved.
///
/// Reported rather than asserted. A low placement rate is not a failure - it
/// means more geometry always draws, which is slower and still correct - but it
/// is the number that says whether the association rule is worth keeping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlacementStats {
    /// Draw calls governed by a declared section.
    pub placed: usize,
    /// Draw calls no declared section governs; they always draw.
    pub unplaced: usize,
}

impl PlacementStats {
    /// Draw calls considered.
    #[must_use]
    pub fn total(&self) -> usize {
        self.placed + self.unplaced
    }

    /// Fraction of draw calls a section governs, `0.0` when there are none.
    #[must_use]
    pub fn placed_fraction(&self) -> f32 {
        if self.total() == 0 {
            return 0.0;
        }
        self.placed as f32 / self.total() as f32
    }
}

impl DrawSections {
    /// Places every draw call of `model` in its authored section.
    ///
    /// `governing` is [`oag_vex::pvs::governing_sections`] over the same
    /// file the model was built from: the section id governing each scene
    /// node, by the sibling-group rule. A draw call maps to the single bit of
    /// its node's section; one with no node, no governing section, or a
    /// governing section `pvs` does not declare gets [`ALWAYS`]. The
    /// undeclared case matters: a mask can only hide what it can also show,
    /// and a bit no mask ever sets would hide the geometry from every frame -
    /// the error must point towards drawing too much instead.
    #[must_use]
    pub fn place(
        model: &Model,
        governing: &[Option<u8>],
        pvs: &TrackPvs,
    ) -> (Self, PlacementStats) {
        let mut stats = PlacementStats::default();
        let mut place_all = |draws: &[DrawCall]| -> Vec<u64> {
            draws
                .iter()
                .map(|draw| {
                    let section = draw
                        .node
                        .and_then(|node| governing.get(node as usize).copied().flatten())
                        .filter(|&id| pvs.declares(id));
                    match section {
                        Some(id) => {
                            stats.placed += 1;
                            1u64 << id
                        }
                        None => {
                            stats.unplaced += 1;
                            ALWAYS
                        }
                    }
                })
                .collect()
        };
        let sections = Self {
            opaque: place_all(&model.draws),
            alpha_tested: place_all(&model.alpha_tested_draws),
            transparent: place_all(&model.transparent_draws),
        };
        (sections, stats)
    }

    /// The sections the `index`-th draw call of one list belongs to.
    ///
    /// Out of range answers [`ALWAYS`] rather than panicking: the lists are
    /// built index-parallel, and if they ever drift the failure should be
    /// "draws too much" and not "crashes mid-frame".
    #[must_use]
    pub fn at(list: &[u64], index: usize) -> u64 {
        list.get(index).copied().unwrap_or(ALWAYS)
    }
}

/// Section pairs authored as alternatives of each other: never named
/// together by any single visibility mask, while their geometry occupies
/// overlapping space.
///
/// This is the LOD-swap relation, recovered from data rather than named by
/// it: nothing on the disc says "section 62 is section 50's far-LOD copy",
/// but the two facts that make the swap work are both authored - the copy
/// coincides with the detail, and no viewpoint's mask ever shows both. The
/// original can never violate the exclusion because it culls with exactly
/// one mask; [`VisibleSet::around`] builds a union for a camera the original
/// does not have, and uses this table to keep the union from re-admitting an
/// alternative the primary mask already decided against.
///
/// Geometry overlap means **shared vertex positions**, not shared space. Two
/// weaker tests were tried and both misfire. Bounding boxes *touching* made
/// 155 pairs of one 58-section circuit and cut its visible set by a third -
/// almost all of them mask-exclusive sections whose boxes merely graze.
/// Boxes *mostly containing each other* still cannot tell a stacked copy
/// from an interior nested inside an exterior - a tunnel's section shares
/// its box with the mountain it runs through, and the two are rightly
/// mask-exclusive without being alternatives; filtering that pair would pop
/// the mountain out of a straddling camera's view for nothing, since nested
/// surfaces do not fight. What only true alternatives share is *geometry*:
/// bit-identical vertex positions, thousands of them on a real copy (the
/// same authored surface, exported twice), and none at all on a nesting.
/// [`Self::SHARED_POSITIONS`] is the guard against numerical accidents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapConflicts {
    partners: [u64; MAX_SECTIONS],
}

impl Default for SwapConflicts {
    fn default() -> Self {
        Self::none()
    }
}

impl SwapConflicts {
    /// How many bit-identical vertex positions two exclusive sections must
    /// share before they count as alternatives.
    ///
    /// The two populations sit far apart: measured stacked copies share
    /// hundreds to thousands of positions (Moa Therma's loop pair shares
    /// ~4,900), unrelated sections share none - exclusive pairs are never
    /// spline neighbours, so not even seam vertices connect them. Four is a
    /// guard against a stray coincidence, not a tuning value; there is no
    /// recovered number to match, because the original never unions masks
    /// and needs no such table.
    pub const SHARED_POSITIONS: usize = 4;

    /// No conflicts - every contribution unions freely, as before.
    #[must_use]
    pub fn none() -> Self {
        Self {
            partners: [0; MAX_SECTIONS],
        }
    }

    /// Finds the swap pairs of one track.
    ///
    /// `sections` is the authored placement of `model`'s draw calls
    /// ([`DrawSections::place`]); only draws governed by a single declared
    /// section contribute to the per-section boxes, which is exactly the set
    /// a mask can hide.
    #[must_use]
    pub fn find(pvs: &TrackPvs, sections: &DrawSections, model: &Model) -> Self {
        // Which sections have a vertex at each exact position. A `BTreeMap`
        // rather than a hash map so the walk below is ordered and the result
        // is identical across platforms, which the screenshot comparisons
        // rely on.
        let mut sections_at: std::collections::BTreeMap<[u32; 3], u64> =
            std::collections::BTreeMap::new();
        let lists = [
            (&sections.opaque, &model.draws),
            (&sections.alpha_tested, &model.alpha_tested_draws),
            (&sections.transparent, &model.transparent_draws),
        ];
        for (masks, draws) in lists {
            for (mask, draw) in masks.iter().zip(draws) {
                if mask.count_ones() != 1 {
                    continue;
                }
                for index in draw.range.clone() {
                    let Some(&vertex) = model.indices.get(index as usize) else {
                        continue;
                    };
                    let Some(v) = model.vertices.get(vertex as usize) else {
                        continue;
                    };
                    let key = [
                        v.position[0].to_bits(),
                        v.position[1].to_bits(),
                        v.position[2].to_bits(),
                    ];
                    *sections_at.entry(key).or_default() |= mask;
                }
            }
        }

        // Shared positions per section pair.
        let mut shared = std::collections::BTreeMap::<(u8, u8), usize>::new();
        for &at in sections_at.values() {
            if at.count_ones() < 2 {
                continue;
            }
            let bits: Vec<u8> = oag_vex::pvs::set_bits(at).collect();
            for (i, &a) in bits.iter().enumerate() {
                for &b in &bits[i + 1..] {
                    *shared.entry((a, b)).or_default() += 1;
                }
            }
        }

        // Which pairs any single authored mask names together.
        let ids: Vec<u8> = pvs.ids().collect();
        let named_together = |a: u8, b: u8| -> bool {
            let both = (1u64 << a) | (1u64 << b);
            ids.iter().any(|&id| pvs.visible_from(id) & both == both)
        };

        let mut partners = [0u64; MAX_SECTIONS];
        for ((a, b), count) in shared {
            if count >= Self::SHARED_POSITIONS && !named_together(a, b) {
                partners[usize::from(a)] |= 1u64 << b;
                partners[usize::from(b)] |= 1u64 << a;
            }
        }
        Self { partners }
    }

    /// Every section that is an alternative of some section in `mask`.
    ///
    /// [`ALL_VISIBLE`] short-circuits to zero: an unknown viewpoint already
    /// draws everything, and a filter derived from "every section at once"
    /// would be meaningless.
    #[must_use]
    pub fn partners_of(&self, mask: u64) -> u64 {
        if mask == ALL_VISIBLE {
            return 0;
        }
        oag_vex::pvs::set_bits(mask).fold(0, |acc, id| acc | self.partners[usize::from(id)])
    }

    /// How many swap pairs were found, for the load report.
    #[must_use]
    pub fn pair_count(&self) -> usize {
        self.partners
            .iter()
            .map(|p| p.count_ones() as usize)
            .sum::<usize>()
            / 2
    }
}

/// Where the camera and the craft are, and what that makes visible.
///
/// Built once a frame, before the draw loop. Everything expensive about it -
/// the adjacency table, the masks - was computed at load; per frame this is a
/// handful of array reads and ORs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleSet {
    mask: u64,
}

impl Default for VisibleSet {
    fn default() -> Self {
        Self::everything()
    }
}

impl VisibleSet {
    /// Hide nothing. The value every uncertain path falls back to.
    #[must_use]
    pub fn everything() -> Self {
        Self { mask: ALL_VISIBLE }
    }

    /// The set visible from a craft in `craft_section` with its camera in
    /// `camera_section`, padded by `adjacency`.
    ///
    /// Three separate reasons this unions more than one section, all of them
    /// things that actually happen in a race:
    ///
    /// - **The camera is not the craft.** The chase spring lags through a
    ///   corner and swings wide on a hit, so it routinely sits in a section the
    ///   craft has already left or not yet reached. Culling to the craft's
    ///   section alone would cut geometry out of the shot the camera is
    ///   actually framing.
    /// - **A boundary is crossed between frames**, and the section that
    ///   *becomes* current has to have been drawn on the frame before. That is
    ///   what the adjacency padding buys.
    /// - **Either section may be unknown**, in which case
    ///   [`oag_vex::pvs::TrackPvs::visible_from`] answers all-ones and this
    ///   degrades to drawing everything.
    ///
    /// **The union is ordered, and later sources cannot override an authored
    /// exclusion.** The original culls with exactly one mask - the craft's -
    /// so it can never draw both halves of a LOD swap: masks encode swaps by
    /// mutual exclusion (Moa Therma's start-valley sections see the far-LOD
    /// copy, section 62, precisely while *not* seeing the detailed sections
    /// the copy coincides with, and no shipped mask names both). Every
    /// source this adds on top of the craft's mask exists for a camera the
    /// original does not cull to, and each is filtered against the swap
    /// partners of what is already in the set:
    ///
    /// 1. **The craft's mask**, whole - the authoritative source, the one
    ///    the original uses.
    /// 2. **The camera's mask**, minus alternatives of anything accepted so
    ///    far. While craft and camera straddle a swap boundary, the union
    ///    would otherwise show both copies for those frames and they would
    ///    z-fight exactly like the placement bug did.
    /// 3. **The padding bits** - the spline neighbours' own geometry, never
    ///    their masks (a neighbour's mask is authored against a viewpoint
    ///    nothing is at; unioning it re-admitted Moa Therma's far-LOD from
    ///    two hops away) - again minus alternatives of anything accepted.
    ///
    /// What a newly-entered section *sees* still arrives the moment it
    /// becomes the craft's or the camera's own, one frame later at most.
    #[must_use]
    pub fn around(
        pvs: &TrackPvs,
        adjacency: &SectionPadding,
        swaps: &SwapConflicts,
        craft_section: u8,
        camera_section: u8,
    ) -> Self {
        let mut mask = pvs.visible_from(craft_section);
        mask |= pvs.visible_from(camera_section) & !swaps.partners_of(mask);
        let near = adjacency.near(craft_section) | adjacency.near(camera_section);
        mask |= near & !swaps.partners_of(mask);
        Self { mask }
    }

    /// The raw mask, for the performance overlay and for tests.
    #[must_use]
    pub fn mask(&self) -> u64 {
        self.mask
    }

    /// How many of the 64 sections this set draws.
    ///
    /// Named `section_count` rather than `len`: this is a popcount over a
    /// fixed-width mask, not the length of a container, and there is no
    /// meaningful empty state to pair it with - a set that allows nothing would
    /// draw nothing, which no code path here can produce.
    #[must_use]
    pub fn section_count(&self) -> u32 {
        self.mask.count_ones()
    }

    /// Whether every section is visible - the conservative state.
    #[must_use]
    pub fn is_everything(&self) -> bool {
        self.mask == ALL_VISIBLE
    }

    /// Whether a draw call touching `sections` may be drawn.
    ///
    /// True when the two masks share any bit: the draw call reaches at least
    /// one section this frame can see. [`ALWAYS`] shares a bit with anything
    /// non-empty, so unplaced geometry is never excluded. This is step one of
    /// the two-tier loop - one `and` and one compare, no floating point and no
    /// memory beyond two values already in registers.
    #[must_use]
    pub fn allows(&self, sections: u64) -> bool {
        self.mask & sections != 0
    }
}

/// The load-time padding table, wrapping [`oag_vex::pvs::SectionAdjacency`].
///
/// Thin on purpose: the interesting part is that adjacency comes from the
/// authored spline rather than from arithmetic on ids, and that lives in
/// `oag-formats` next to the data it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionPadding {
    inner: oag_vex::pvs::SectionAdjacency,
}

impl Default for SectionPadding {
    fn default() -> Self {
        Self {
            inner: oag_vex::pvs::SectionAdjacency::none(),
        }
    }
}

impl SectionPadding {
    /// How far padding spreads along the spline.
    ///
    /// Two, matching the task this was built for. One hop covers a boundary
    /// crossed between two frames; the second covers a camera that has swung
    /// into the section beyond the one the craft is entering, which a hit at
    /// speed does. There is no recovered value to match here - the original
    /// pads nothing, because it does not have this problem: it looks the craft
    /// up in its own partition every frame rather than culling to a camera.
    pub const HOPS: u32 = 2;

    /// Builds the table from a track's spline graph.
    #[must_use]
    pub fn from_track(track: &AiTrack) -> Self {
        Self {
            inner: oag_vex::pvs::SectionAdjacency::from_track(track, Self::HOPS),
        }
    }

    /// Sections within [`Self::HOPS`] of `id`, including `id`.
    #[must_use]
    pub fn near(&self, id: u8) -> u64 {
        self.inner.near(id)
    }
}

/// How far either viewpoint's neighbourhood is unioned into a PS3 chunk set,
/// in world units.
///
/// **Two median cell spacings.** Every circuit's `track.pvs` places its cells
/// 12.0 units apart along the racing line (median over Talon's Junction's 621
/// and Anulpha Pass's 763, measured identically on both), so this reaches the
/// two cells either side of the one a point lands in. That covers the boundary
/// a craft crosses between two frames, which is the same thing
/// [`SectionPadding::HOPS`] buys on the PSP - and the reason it is needed here
/// too is that the original looks the *craft* up in its own partition each
/// frame and never culls to a camera, so nothing on the disc pads for a chase
/// spring that trails the craft.
pub const CHUNK_PAD: f32 = 24.0;

/// How far a viewpoint may be from the nearest cell and still be located.
///
/// Beyond this the set degrades to drawing everything, on the same rule as
/// [`UNPLACED`]: a craft that has fallen off the circuit or been knocked into
/// scenery is somewhere the partition was not authored around, and culling to
/// a stale cell exactly then is the most visible way this could go wrong. 64
/// units is five cell spacings and comfortably wider than any circuit's track
/// half-width, so it cannot trip during ordinary racing.
pub const CHUNK_TRUST_RADIUS: f32 = 64.0;

/// Which of a PS3 circuit's chunks this frame may draw - the HD counterpart of
/// [`VisibleSet`].
///
/// **A bitmap rather than a mask, because HD partitions by chunk and not by
/// section.** `track.pvs` carries one bit per `.rcsmodel` chunk per cell (see
/// [`oag_rcs::hd_pvs`]), which is 983 to 1,902 bits rather than the PSP's
/// 64, so there is no mask to `and` against and the per-draw test is a byte
/// load and a shift instead. Everything else about the two tiers is the same,
/// including that this one runs first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChunkSet {
    bits: Vec<u8>,
}

impl ChunkSet {
    /// The union of what the craft's cell and the camera's cell can see, each
    /// padded by [`CHUNK_PAD`].
    ///
    /// `None` when neither viewpoint is within [`CHUNK_TRUST_RADIUS`] of any
    /// cell, which the caller turns into no first tier at all - draw
    /// everything. The union is a plain `or` with no equivalent of
    /// [`SwapConflicts`]: nothing here has been measured to encode a LOD swap
    /// by mutual exclusion the way Moa Therma's sections do, so subtracting
    /// anything would be a rule invented rather than found.
    #[must_use]
    pub fn around(pvs: &oag_rcs::hd_pvs::Pvs, craft: Vec3, camera: Vec3) -> Option<Self> {
        let mut bits: Option<Vec<u8>> = None;
        for point in [craft, camera] {
            let Some(nearest) = pvs.nearest_cell(point.to_array()) else {
                continue;
            };
            let Some(at) = pvs.position(nearest) else {
                continue;
            };
            if Vec3::from_array(at).distance(point) > CHUNK_TRUST_RADIUS {
                continue;
            }
            for cell in 0..pvs.cells() {
                let Some(p) = pvs.position(cell) else {
                    continue;
                };
                if cell != nearest && Vec3::from_array(p).distance(point) > CHUNK_PAD {
                    continue;
                }
                let Some(cell_bits) = pvs.cell_bits(cell) else {
                    continue;
                };
                match &mut bits {
                    None => bits = Some(cell_bits.to_vec()),
                    Some(acc) => {
                        for (a, b) in acc.iter_mut().zip(cell_bits) {
                            *a |= b;
                        }
                    }
                }
            }
        }
        bits.map(|bits| Self { bits })
    }

    /// Whether this frame may draw `draw`.
    ///
    /// A draw call with no chunk index is always allowed, which is the same
    /// direction of error [`ALWAYS`] points in: an unplaced batch costs a
    /// frustum test, a wrongly-excluded one disappears from the shot.
    #[must_use]
    pub fn allows(&self, draw: &DrawCall) -> bool {
        match draw.chunk {
            None => true,
            Some(chunk) => oag_rcs::hd_pvs::allows(&self.bits, chunk as usize),
        }
    }

    /// How many chunks this set draws, for the load report and the overlay.
    #[must_use]
    pub fn chunk_count(&self) -> u32 {
        self.bits.iter().map(|b| b.count_ones()).sum()
    }
}

/// The two-tier test, in the order that makes it worth doing.
///
/// **PVS first, frustum second**, and the ordering is the point rather than a
/// detail: the mask test is an integer and-compare against a value already in a
/// register, while [`Frustum::intersects_sphere`] is six plane evaluations of
/// three multiplies and two adds each. Running the cheap test first means the
/// expensive one only ever sees geometry the track says could be on screen at
/// all.
///
/// `visible_set` is `None` when PVS culling is off, `frustum` is `None` when
/// frustum culling is off or when the model carries a transform its bounds are
/// not valid against - see [`crate::mesh::Bounds`]. Both off is the default and
/// draws everything.
#[must_use]
pub fn visible(
    draw: &DrawCall,
    sections: u64,
    visible_set: Option<&VisibleSet>,
    chunks: Option<&ChunkSet>,
    frustum: Option<&Frustum>,
) -> bool {
    // The authored section mask is structural - it keys on the draw's node,
    // not on where the geometry is - so it holds for a draw the shader moves
    // exactly as for a static one. **The original hides a moving mesh by its
    // section too**: at four poses on Talon's Junction, every one of the 16
    // to 78 moving draws the original's GE list contained was still drawn
    // under this rule, while the rule dropped 133 to 193 of the 214 moving
    // draws the original did not submit - among them the sixteen batches of
    // the slab transports that drew as dark roofs beside the straight. See
    // `docs/rendering/frame-audit.md` section 3.
    if let Some(set) = visible_set
        && !set.allows(sections)
    {
        return false;
    }
    // The frustum is the one test a moving draw cannot take: its bounds
    // describe where it was at time zero and nothing about where it is now.
    // See [`DrawCall::moving`].
    if draw.moving {
        return true;
    }
    // The PS3's first tier, and mutually exclusive with the one above in
    // practice: a Pulse track has sections and no chunk indices, an HD one has
    // chunk indices and no sections.
    if let Some(set) = chunks
        && !set.allows(draw)
    {
        return false;
    }
    match frustum {
        None => true,
        Some(frustum) => {
            frustum.intersects_sphere(Vec3::from_array(draw.bounds.centre), draw.bounds.radius)
        }
    }
}

#[cfg(test)]
mod tests;
