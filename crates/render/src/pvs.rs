//! Placing render geometry into the track's authored visibility sections.
//!
//! **This module is an engine decision, not a reproduction**, and the
//! distinction is the whole reason it lives here rather than in
//! [`oag_formats::pvs`]. That module reads what the artists authored: the
//! partition, the per-section masks, the boxes. What no read of either original
//! binary has recovered is **which section a given `Mesh` node belongs to** -
//! sections attach to spline control points, not to meshes, and the class-table
//! walk that produced the layout says nothing about the association.
//!
//! So the association is invented here, and labelled as invented. It is
//! packaged the way `[graphics] frustum_culling` is - config-file only, no menu
//! row - and defaults on, having cleared the same bar: thirty captures across
//! three tracks and every combination of the two tiers, all byte-identical to
//! culling nothing. See
//! `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
//!
//! # The rule: a draw call belongs to every section it touches
//!
//! Each draw call gets its **own 64-bit mask** of the sections whose authored
//! box its bounding sphere intersects, and it is drawn when that mask and the
//! frame's visible set share a bit. A draw call touching no box gets
//! [`ALWAYS`], all ones, and is drawn every frame.
//!
//! **The obvious cheaper rule is wrong, and it was tried first.** Placing a
//! draw call in the single section containing its bounding-sphere *centre*
//! costs one `u8` instead of a `u64` and makes the test a shift-and-and. It
//! also loses background scenery: a long building or a ridge line spans a dozen
//! sections, its centre lands in exactly one, and the whole mesh disappears the
//! moment that one section drops out of the set - while remaining plainly in
//! shot. A screenshot comparison at 600 ticks on the default track caught it:
//! 212,808 differing bytes, a ridge line and a large building missing down the
//! left of the frame. Recorded here because the failure is invisible in
//! aggregate statistics - centre placement reports a *lower* count of draw
//! calls surviving, which reads as better culling and is simply wrong.
//!
//! So the extra 56 bits per draw call are not an optimisation that was skipped;
//! they are what makes the association correct for geometry larger than a
//! section. At ~2,000 draw calls a track that is 16 KB.
//!
//! **Expect a modest saving.** Because a `DrawCall` is one material run, a
//! batch typically reaches 6 to 14 of the 64 sections, so the first tier
//! removes 22% to 66% of the frustum test's input depending on the track -
//! about half on a median one - and nothing at all from the worst section of
//! most of them. The authored data is
//! far more selective than that; our batching is what caps it. The ADR has the
//! per-track table and what would lift the ceiling.
//!
//! # What this does not do yet
//!
//! Draw calls are tested one at a time. Sorting the lists by section at load
//! and walking only the visible ranges would make an excluded batch cost
//! nothing rather than one mask test. It is not done here because a draw call
//! now belongs to *several* sections, so there is no single key to sort by
//! without duplicating entries - and because the transparent list is drawn in
//! authored order, where blending makes reordering change the picture. The ADR
//! records this as the next step.

use oag_core::math::Vec3;
use oag_core::math::frustum::Frustum;
use oag_formats::pvs::{ALL_VISIBLE, Aabb, TrackPvs};
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
///
/// `spanning` is the count that matters for correctness rather than for speed:
/// draw calls touching more than one section are exactly the ones a
/// centre-based rule got wrong.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlacementStats {
    pub placed: usize,
    pub unplaced: usize,
    pub spanning: usize,
    /// Sections touched, summed over placed draw calls, for a mean.
    pub sections_touched: usize,
}

impl PlacementStats {
    /// Draw calls considered.
    #[must_use]
    pub fn total(&self) -> usize {
        self.placed + self.unplaced
    }

    /// Fraction of draw calls an authored box touched, `0.0` when there are
    /// none.
    #[must_use]
    pub fn placed_fraction(&self) -> f32 {
        if self.total() == 0 {
            return 0.0;
        }
        self.placed as f32 / self.total() as f32
    }

    /// Mean sections touched by a placed draw call, `0.0` when there are none.
    #[must_use]
    pub fn mean_sections(&self) -> f32 {
        if self.placed == 0 {
            return 0.0;
        }
        self.sections_touched as f32 / self.placed as f32
    }
}

impl DrawSections {
    /// Places every draw call of `model` into `pvs`'s authored sections.
    #[must_use]
    pub fn place(model: &Model, pvs: &TrackPvs) -> (Self, PlacementStats) {
        let mut stats = PlacementStats::default();
        let mut place_all = |draws: &[DrawCall]| -> Vec<u64> {
            draws
                .iter()
                .map(|draw| {
                    let touched = place(draw, pvs);
                    if touched == 0 {
                        stats.unplaced += 1;
                        return ALWAYS;
                    }
                    stats.placed += 1;
                    stats.sections_touched += touched.count_ones() as usize;
                    if touched.count_ones() > 1 {
                        stats.spanning += 1;
                    }
                    touched
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

    /// The sections the `index`-th draw call of one list touches.
    ///
    /// Out of range answers [`ALWAYS`] rather than panicking: the lists are
    /// built index-parallel, and if they ever drift the failure should be
    /// "draws too much" and not "crashes mid-frame".
    #[must_use]
    pub fn at(list: &[u64], index: usize) -> u64 {
        list.get(index).copied().unwrap_or(ALWAYS)
    }
}

/// Every section whose authored box `draw`'s bounding sphere reaches, as a
/// mask. Zero when it reaches none.
fn place(draw: &DrawCall, pvs: &TrackPvs) -> u64 {
    let mut mask = 0u64;
    for id in pvs.ids() {
        let Some(bounds) = pvs.bounds_of(id) else {
            continue;
        };
        if sphere_touches_box(draw.bounds.centre, draw.bounds.radius, bounds) {
            mask |= 1u64 << id;
        }
    }
    mask
}

/// Whether a sphere reaches an axis-aligned box.
///
/// The standard test: clamp the centre into the box on each axis, and compare
/// the squared distance to the clamped point against the squared radius. A
/// centre inside the box clamps to itself and gives zero, so containment is the
/// degenerate case rather than a separate branch.
fn sphere_touches_box(centre: [f32; 3], radius: f32, bounds: Aabb) -> bool {
    let mut squared = 0.0f32;
    for (axis, &c) in centre.iter().enumerate() {
        let d = c - c.clamp(bounds.min[axis], bounds.max[axis]);
        squared += d * d;
    }
    squared <= radius * radius
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
    #[must_use]
    pub fn around(
        pvs: &TrackPvs,
        adjacency: &SectionPadding,
        craft_section: u8,
        camera_section: u8,
    ) -> Self {
        let near = adjacency.near(craft_section) | adjacency.near(camera_section);
        let mut mask = pvs.visible_from(craft_section) | pvs.visible_from(camera_section);
        for id in oag_formats::pvs::set_bits(near) {
            mask |= pvs.visible_from(id);
        }
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

    #[test]
    fn a_small_draw_call_touches_only_the_box_it_sits_in() {
        let pvs = pvs(&[
            (0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            (3, [20.0, 0.0, 0.0], [30.0, 10.0, 10.0]),
        ]);
        assert_eq!(place(&draw_at([5.0, 5.0, 5.0], 0.5), &pvs), 1 << 0);
        assert_eq!(place(&draw_at([25.0, 5.0, 5.0], 0.5), &pvs), 1 << 3);
    }

    /// **The regression that made this rule what it is.** A mesh wider than a
    /// section - a ridge line, a terminal building - has its centre in one
    /// section and its geometry in several. Placing it by its centre alone made
    /// it vanish whenever that one section dropped out of the visible set,
    /// while it was still plainly in shot.
    #[test]
    fn a_draw_call_wider_than_a_section_touches_every_section_it_spans() {
        let pvs = pvs(&[
            (0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            (1, [10.0, 0.0, 0.0], [20.0, 10.0, 10.0]),
            (2, [20.0, 0.0, 0.0], [30.0, 10.0, 10.0]),
        ]);
        // Centred in section 1, but 15 units of radius reaches all three.
        let sections = place(&draw_at([15.0, 5.0, 5.0], 15.0), &pvs);
        assert_eq!(sections, 0b111);

        // And it survives a set that can only see section 0 - which is exactly
        // what the centre-based rule got wrong.
        let sees_only_0 = VisibleSet { mask: 1 };
        assert!(sees_only_0.allows(sections));
    }

    /// The direction the error has to point: geometry no box touches keeps
    /// drawing rather than vanishing.
    #[test]
    fn a_draw_call_touching_no_box_always_draws() {
        let pvs = pvs(&[(0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0])]);
        assert_eq!(place(&draw_at([100.0, 100.0, 100.0], 1.0), &pvs), 0);

        let (sections, stats) =
            DrawSections::place(&model_of(vec![draw_at([100.0, 100.0, 100.0], 1.0)]), &pvs);
        assert_eq!(sections.opaque, vec![ALWAYS], "unplaced becomes all ones");
        assert_eq!(stats.unplaced, 1);

        let hides_all_declared = VisibleSet { mask: 1 };
        assert!(
            hides_all_declared.allows(ALWAYS),
            "unplaced geometry survives any non-empty set"
        );
    }

    /// A sphere that only reaches the face of a box still touches it. The
    /// clamped-distance test makes containment the degenerate case rather than
    /// a separate branch, so this is the case a `contains`-only rule missed.
    #[test]
    fn a_sphere_reaching_a_face_from_outside_touches_the_box() {
        let box_ = Aabb {
            min: [0.0; 3],
            max: [10.0, 10.0, 10.0],
        };
        assert!(
            sphere_touches_box([15.0, 5.0, 5.0], 5.0, box_),
            "just reaches"
        );
        assert!(
            !sphere_touches_box([15.0, 5.0, 5.0], 4.9, box_),
            "just misses"
        );
        assert!(
            sphere_touches_box([5.0, 5.0, 5.0], 0.0, box_),
            "centre inside"
        );
        // A corner, where all three axes contribute to the distance.
        assert!(!sphere_touches_box([13.0, 13.0, 13.0], 5.0, box_));
        assert!(sphere_touches_box([13.0, 13.0, 13.0], 5.2, box_));
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

    #[test]
    fn placing_a_model_counts_what_it_placed() {
        let pvs = pvs(&[
            (0, [0.0, 0.0, 0.0], [10.0, 10.0, 10.0]),
            (1, [10.0, 0.0, 0.0], [20.0, 10.0, 10.0]),
        ]);
        let mut model = model_of(vec![
            draw_at([5.0, 5.0, 5.0], 0.5),  // section 0 only
            draw_at([10.0, 5.0, 5.0], 3.0), // spans 0 and 1
            draw_at([90.0, 5.0, 5.0], 1.0), // nowhere
        ]);
        model
            .alpha_tested_draws
            .push(draw_at([15.0, 5.0, 5.0], 0.5));

        let (sections, stats) = DrawSections::place(&model, &pvs);
        assert_eq!(sections.opaque, vec![0b01, 0b11, ALWAYS]);
        assert_eq!(sections.alpha_tested, vec![0b10]);
        assert_eq!(sections.transparent, Vec::<u64>::new());
        assert_eq!(stats.placed, 3);
        assert_eq!(stats.unplaced, 1);
        assert_eq!(stats.spanning, 1);
        assert_eq!(stats.total(), 4);
        assert!((stats.placed_fraction() - 0.75).abs() < 1e-6);
        // 1 + 2 + 1 sections over 3 placed draw calls.
        assert!((stats.mean_sections() - 4.0 / 3.0).abs() < 1e-6);
        assert_eq!(PlacementStats::default().placed_fraction(), 0.0);
        assert_eq!(PlacementStats::default().mean_sections(), 0.0);
        assert_eq!(DrawSections::at(&sections.opaque, 99), ALWAYS);
    }

    #[test]
    fn an_unknown_section_makes_the_visible_set_everything() {
        let pvs = pvs(&[(0, [0.0; 3], [1.0; 3])]);
        let padding = SectionPadding::default();
        let set = VisibleSet::around(&pvs, &padding, UNPLACED, UNPLACED);
        assert!(set.is_everything(), "neither section is declared");
        assert_eq!(set.section_count(), 64);
        assert!(set.allows(1 << 40));
    }

    #[test]
    fn the_visible_set_unions_the_craft_and_the_camera() {
        // Each section sees only itself.
        let pvs = pvs(&[(0, [0.0; 3], [1.0; 3]), (1, [0.0; 3], [1.0; 3])]);
        let padding = SectionPadding::default();

        let alone = VisibleSet::around(&pvs, &padding, 0, 0);
        assert_eq!(alone.mask(), 1, "just section 0");

        let split = VisibleSet::around(&pvs, &padding, 0, 1);
        assert_eq!(split.mask(), 0b11, "the camera's section is drawn too");
        assert!(split.allows(1) && split.allows(0b10) && !split.allows(0b100));
    }

    /// Padding pulls in the neighbours' *masks*, not merely the neighbours.
    #[test]
    fn padding_unions_what_the_neighbours_can_see() {
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
        let pvs = pvs(&[(0, [0.0; 3], [1.0; 3]), (1, [0.0; 3], [1.0; 3])]);
        let padding = SectionPadding::from_track(&track);
        assert!(padding.near(0) & 0b10 != 0, "1 is next door to 0");

        let set = VisibleSet::around(&pvs, &padding, 0, 0);
        assert_eq!(
            set.mask(),
            0b11,
            "the neighbour is drawn before it is entered"
        );
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
