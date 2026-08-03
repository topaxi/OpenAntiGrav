//! Placing render geometry into the track's authored visibility sections.
//!
//! # The rule: a draw call belongs to its authored section
//!
//! The association is on the disc after all: **a `section` node governs its
//! parent's whole subtree** - a track is authored as sibling groups, one
//! transform per group with the `section` as one child and the group's
//! geometry as the rest. [`oag_formats::pvs::governing_sections`] derives it;
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
use oag_formats::pvs::{ALL_VISIBLE, MAX_SECTIONS, TrackPvs};
use oag_formats::track::AiTrack;

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
/// section, and [`oag_formats::pvs::TrackPvs::visible_from`] answers
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
    /// `governing` is [`oag_formats::pvs::governing_sections`] over the same
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
            let bits: Vec<u8> = oag_formats::pvs::set_bits(at).collect();
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
        oag_formats::pvs::set_bits(mask).fold(0, |acc, id| acc | self.partners[usize::from(id)])
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
    ///   [`oag_formats::pvs::TrackPvs::visible_from`] answers all-ones and this
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

/// The load-time padding table, wrapping [`oag_formats::pvs::SectionAdjacency`].
///
/// Thin on purpose: the interesting part is that adjacency comes from the
/// authored spline rather than from arithmetic on ids, and that lives in
/// `oag-formats` next to the data it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionPadding {
    inner: oag_formats::pvs::SectionAdjacency,
}

impl Default for SectionPadding {
    fn default() -> Self {
        Self {
            inner: oag_formats::pvs::SectionAdjacency::none(),
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
            inner: oag_formats::pvs::SectionAdjacency::from_track(track, Self::HOPS),
        }
    }

    /// Sections within [`Self::HOPS`] of `id`, including `id`.
    #[must_use]
    pub fn near(&self, id: u8) -> u64 {
        self.inner.near(id)
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
    frustum: Option<&Frustum>,
) -> bool {
    if let Some(set) = visible_set
        && !set.allows(sections)
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
mod tests {
    use super::*;
    use crate::mesh::Bounds;

    fn draw_at(centre: [f32; 3], radius: f32) -> DrawCall {
        DrawCall {
            range: 0..3,
            texture: None,
            bounds: Bounds { centre, radius },
            node: None,
        }
    }

    /// Sections with boxes, built the way a `.vex` would hold them.
    fn pvs(boxes: &[(u8, [f32; 3], [f32; 3])]) -> TrackPvs {
        let mut data = Vec::new();
        let mut nodes = Vec::new();
        for &(index, min, max) in boxes {
            nodes.push(oag_formats::vex::Node {
                class_id: oag_formats::vex::CLASS_SECTION,
                offset: data.len(),
                header_size: 0,
                data_size: 0x30,
                child_count: 0,
                unk_0x0e: 0,
                name: None,
                depth: 0,
                parent: None,
            });
            data.extend([index, 1, 0, 0, 0, 0, 0, 0]);
            data.extend(0u32.to_le_bytes());
            data.extend(0u32.to_le_bytes());
            for v in min {
                data.extend(v.to_le_bytes());
            }
            data.extend(0f32.to_le_bytes());
            for v in max {
                data.extend(v.to_le_bytes());
            }
            data.extend(0f32.to_le_bytes());
        }
        TrackPvs::from_nodes(&data, &nodes).expect("parse")
    }

    fn draw_of_node(node: Option<u32>) -> DrawCall {
        DrawCall {
            range: 0..3,
            texture: None,
            bounds: Bounds {
                centre: [0.0; 3],
                radius: 1.0,
            },
            node,
        }
    }

    fn model_of(draws: Vec<DrawCall>) -> Model {
        Model {
            label: "test".into(),
            vertices: Vec::new(),
            indices: vec![0, 1, 2],
            draws,
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            textures: Vec::new(),
            centre: [0.0; 3],
            radius: 1.0,
            mesh_count: 1,
        }
    }

    /// Placement is structural: a governed draw call gets its group's one
    /// bit, whatever its bounds - which is exactly what lets a far-LOD copy
    /// sitting *inside* the racing sections' boxes stay hidden while racing.
    #[test]
    fn a_governed_draw_call_gets_its_sections_single_bit() {
        let pvs = pvs(&[
            (0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            (3, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
        ]);
        // Nodes 0..3, where node 1 belongs to section 0's group and node 2 to
        // section 3's - the two groups' boxes overlap entirely.
        let governing = vec![None, Some(0), Some(3), None];
        let mut model = model_of(vec![draw_of_node(Some(1)), draw_of_node(Some(2))]);
        model.alpha_tested_draws.push(draw_of_node(Some(2)));

        let (sections, stats) = DrawSections::place(&model, &governing, &pvs);
        assert_eq!(sections.opaque, vec![1 << 0, 1 << 3]);
        assert_eq!(sections.alpha_tested, vec![1 << 3]);
        assert_eq!(sections.transparent, Vec::<u64>::new());
        assert_eq!(stats.placed, 3);
        assert_eq!(stats.unplaced, 0);
        assert_eq!(stats.total(), 3);
        assert!((stats.placed_fraction() - 1.0).abs() < 1e-6);
        assert_eq!(DrawSections::at(&sections.opaque, 99), ALWAYS);

        let sees_only_0 = VisibleSet { mask: 1 };
        assert!(sees_only_0.allows(sections.opaque[0]));
        assert!(
            !sees_only_0.allows(sections.opaque[1]),
            "the coincident group in the unseen section is hidden"
        );
    }

    /// The direction the error has to point, three ways: no node, no
    /// governing section, and a governing section the track does not declare
    /// all draw every frame rather than never.
    #[test]
    fn an_ungoverned_draw_call_always_draws() {
        let pvs = pvs(&[(0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0])]);
        let governing = vec![None, Some(0), Some(9)];
        let model = model_of(vec![
            draw_of_node(None),     // synthetic geometry
            draw_of_node(Some(0)),  // node no section governs
            draw_of_node(Some(2)),  // governed by undeclared section 9
            draw_of_node(Some(50)), // node index past the governance table
        ]);

        let (sections, stats) = DrawSections::place(&model, &governing, &pvs);
        assert_eq!(sections.opaque, vec![ALWAYS; 4]);
        assert_eq!(stats.placed, 0);
        assert_eq!(stats.unplaced, 4);
        assert_eq!(PlacementStats::default().placed_fraction(), 0.0);

        let hides_all_declared = VisibleSet { mask: 1 };
        assert!(
            hides_all_declared.allows(ALWAYS),
            "unplaced geometry survives any non-empty set"
        );
    }

    #[test]
    fn an_unknown_section_makes_the_visible_set_everything() {
        let pvs = pvs(&[(0, [0.0; 3], [1.0; 3])]);
        let padding = SectionPadding::default();
        let set = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), UNPLACED, UNPLACED);
        assert!(set.is_everything(), "neither section is declared");
        assert_eq!(set.section_count(), 64);
        assert!(set.allows(1 << 40));
    }

    #[test]
    fn the_visible_set_unions_the_craft_and_the_camera() {
        // Each section sees only itself.
        let pvs = pvs(&[(0, [0.0; 3], [1.0; 3]), (1, [0.0; 3], [1.0; 3])]);
        let padding = SectionPadding::default();

        let alone = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 0);
        assert_eq!(alone.mask(), 1, "just section 0");

        let split = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 1);
        assert_eq!(split.mask(), 0b11, "the camera's section is drawn too");
        assert!(split.allows(1) && split.allows(0b10) && !split.allows(0b100));
    }

    /// Padding pulls in the neighbours' *masks*, not merely the neighbours.
    #[test]
    fn padding_draws_the_neighbours_but_not_what_they_see() {
        use oag_formats::track::{AiTrack, Path, SplinePoint};
        let point = |section_id| SplinePoint {
            pos: [0.0; 3],
            tangent: [0.0, 0.0, 1.0],
            down: [0.0, -1.0, 0.0],
            lateral: [1.0, 0.0, 0.0],
            half_width_left: 1.0,
            half_width_right: 1.0,
            ai_bound_left: 1.0,
            ai_bound_right: 1.0,
            racing_line: 0.0,
            section_id,
            flags: 0,
        };
        let track = AiTrack {
            version: 0x105,
            paths: vec![Path {
                points: vec![point(0), point(1)],
                max_spacing: 1.0,
                entry: None,
                exit: None,
            }],
            junctions: Vec::new(),
        };
        // Section 1's mask names a far-away section 40 - the LOD-swap shape:
        // what a neighbour sees is authored against *its* viewpoint, and
        // pulling it in early is how a far-LOD copy got drawn over the
        // detailed track it duplicates (Moa Therma's magstrip artifact,
        // second cause).
        let pvs = pvs_with_masks(&[(0, 0), (1, 1 << 40)]);
        let padding = SectionPadding::from_track(&track);
        assert!(padding.near(0) & 0b10 != 0, "1 is next door to 0");

        let set = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 0);
        assert_eq!(
            set.mask(),
            0b11,
            "the neighbour itself is drawn before it is entered, and what \
             only the neighbour can see is not"
        );

        // Once the craft is actually in section 1, its mask applies whole.
        let entered = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 1, 1);
        assert!(entered.allows(1 << 40));
    }

    /// A LOD swap in miniature: sections 0 and 1 are ordinary neighbours
    /// (each names the other), 2 is the detail and 3 the coincident copy -
    /// named by disjoint viewpoints, never together. The draws of sections
    /// 1, 2 and 3 share bit-identical vertex positions; section 0's geometry
    /// is elsewhere.
    fn swap_fixture() -> (TrackPvs, SwapConflicts) {
        let pvs = pvs_with_masks(&[
            (0, 0b0110), // sees 1 and the detail 2
            (1, 0b1001), // sees 0 and the copy 3
            (2, 0b0001),
            (3, 0b0010),
        ]);
        let governing = vec![Some(0), Some(1), Some(2), Some(3)];
        let mut model = model_of(vec![
            draw_of_node(Some(0)),
            draw_of_node(Some(1)),
            draw_of_node(Some(2)),
            draw_of_node(Some(3)),
        ]);
        let vertex = |x: f32| crate::mesh::GpuVertex {
            position: [x, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
            colour: [1.0; 4],
            texcoord: [0.0; 2],
            lit: 1.0,
            v_cycles: 0.0,
        };
        model.indices.clear();
        for (draw, base_x) in [(0usize, 100.0f32), (1, 0.0), (2, 0.0), (3, 0.0)] {
            let start = model.vertices.len() as u32;
            for i in 0..SwapConflicts::SHARED_POSITIONS {
                model.vertices.push(vertex(base_x + i as f32));
            }
            let end = model.vertices.len() as u32;
            model.indices.extend(start..end);
            model.draws[draw].range = start..end;
        }
        let (sections, _) = DrawSections::place(&model, &governing, &pvs);
        let swaps = SwapConflicts::find(&pvs, &sections, &model);
        (pvs, swaps)
    }

    /// Only mask-exclusive pairs with overlapping geometry are swaps. Here
    /// that is 2/3 alone: 1's geometry coincides with both, but mask 0 names
    /// 1 with 2 and mask 1 names 1 with 3, so neither is exclusive - and 0
    /// is exclusive with nothing that shares its space.
    #[test]
    fn only_coincident_exclusive_pairs_are_swaps() {
        let (_, swaps) = swap_fixture();
        assert_eq!(swaps.pair_count(), 1, "the detail and its copy");
        assert_eq!(swaps.partners_of(1 << 2), 1 << 3);
        assert_eq!(swaps.partners_of(1 << 3), 1 << 2);
        assert_eq!(swaps.partners_of(1 << 1), 0, "named together is not a swap");
        assert_eq!(swaps.partners_of(1 << 0), 0, "distant geometry never pairs");
        assert_eq!(SwapConflicts::none().partners_of(ALL_VISIBLE), 0);
        assert_eq!(
            swaps.partners_of(ALL_VISIBLE),
            0,
            "unknown draws everything"
        );
    }

    /// The straddle: craft already across the boundary (sees the detail),
    /// camera still behind (sees the copy). The craft's mask wins and the
    /// copy stays hidden; the rest of the camera's mask still contributes.
    #[test]
    fn the_crafts_mask_outranks_the_cameras_across_a_swap() {
        let (pvs, swaps) = swap_fixture();
        let padding = SectionPadding::default();

        let straddle = VisibleSet::around(&pvs, &padding, &swaps, 0, 1);
        assert!(straddle.allows(1 << 2), "the craft's detail is drawn");
        assert!(
            !straddle.allows(1 << 3),
            "the camera's copy of it is not - the authored exclusion holds"
        );
        assert!(
            straddle.allows(1 << 0) && straddle.allows(1 << 1),
            "the camera's non-conflicting geometry still contributes"
        );

        // The same straddle without the table is the bug this exists for.
        let unfiltered = VisibleSet::around(&pvs, &padding, &SwapConflicts::none(), 0, 1);
        assert!(unfiltered.allows(1 << 2) && unfiltered.allows(1 << 3));

        // And from the other side of the boundary the swap flips whole.
        let flipped = VisibleSet::around(&pvs, &padding, &swaps, 1, 0);
        assert!(flipped.allows(1 << 3) && !flipped.allows(1 << 2));
    }

    /// Sections without boxes but with authored visibility masks.
    fn pvs_with_masks(sections: &[(u8, u64)]) -> TrackPvs {
        let mut data = Vec::new();
        let mut nodes = Vec::new();
        for &(index, mask) in sections {
            nodes.push(oag_formats::vex::Node {
                class_id: oag_formats::vex::CLASS_SECTION,
                offset: data.len(),
                header_size: 0,
                data_size: 0x10,
                child_count: 0,
                unk_0x0e: 0,
                name: None,
                depth: 0,
                parent: None,
            });
            data.extend([index, 0, 0, 0, 0, 0, 0, 0]);
            data.extend((mask as u32).to_le_bytes());
            data.extend(((mask >> 32) as u32).to_le_bytes());
        }
        TrackPvs::from_nodes(&data, &nodes).expect("parse")
    }

    /// The ordering claim, made executable: with no frustum at all, the mask
    /// alone decides, and geometry the mask excludes never reaches the frustum
    /// test.
    #[test]
    fn the_mask_decides_before_the_frustum_is_consulted() {
        let draw = draw_at([0.0, 0.0, 0.0], 1.0);
        let hides_section_2 = VisibleSet { mask: !(1u64 << 2) };
        assert!(!visible(&draw, 1 << 2, Some(&hides_section_2), None));
        assert!(visible(&draw, 1 << 3, Some(&hides_section_2), None));
        assert!(
            visible(&draw, 1 << 2, None, None),
            "PVS off draws everything"
        );
        assert!(
            visible(&draw, ALWAYS, Some(&hides_section_2), None),
            "unplaced geometry is never excluded"
        );
    }
}
