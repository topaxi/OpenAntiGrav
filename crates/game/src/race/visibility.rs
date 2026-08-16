//! What the frame decided not to draw: the culling counters the performance
//! overlay reads, and the track sections a camera can see from where it is.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// What one frame's frustum culling did, for the performance overlay.
///
/// Plain counts rather than anything richer: the point is to show the effect
/// of `[graphics] lod` and frustum culling is having, not to profile the
/// renderer.
#[derive(Debug, Clone, Copy, Default)]
pub struct SceneStats {
    pub draws_submitted: u32,
    pub draws_culled: u32,
    pub triangles: u32,
}

impl SceneStats {
    pub(super) fn add(&mut self, other: Self) {
        self.draws_submitted += other.draws_submitted;
        self.draws_culled += other.draws_culled;
        self.triangles += other.triangles;
    }
}

/// The track's authored visibility partition, and where this track's geometry
/// sits in it.
///
/// Built once at load. Per frame it answers one question - which sections may
/// be drawn - in a handful of array reads. See
/// `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
#[derive(Debug)]
pub struct TrackVisibility {
    pub(super) pvs: oag_formats::pvs::TrackPvs,
    padding: SectionPadding,
    pub(super) sections: DrawSections,
    /// Mask-exclusive section pairs with overlapping geometry - authored
    /// LOD swaps, which the per-frame union must never re-join. See
    /// [`SwapConflicts`].
    swaps: SwapConflicts,
    /// What the association rule managed, for the load report.
    pub placement: PlacementStats,
}

impl TrackVisibility {
    /// Reads the `section` nodes of a track and places its draw calls in them.
    ///
    /// Returns `None` when the track declares no sections at all - which is
    /// every Pure track, since Pure does not share Pulse's class numbering.
    /// The caller then draws with no first tier, exactly as before.
    #[must_use]
    pub fn build(model: &Model, blob: &[u8], ai: &AiTrack) -> Option<Self> {
        let nodes = oag_formats::vex::nodes(blob).ok()?;
        let pvs = oag_formats::pvs::TrackPvs::from_nodes(blob, &nodes).ok()?;
        if pvs.is_empty() {
            return None;
        }
        // The authored association: a `section` node governs its parent's
        // whole subtree, so each draw call inherits its scene node's group.
        // This is what hides a far-LOD copy of the track while racing on the
        // real one - see `oag_render::pvs`.
        let governing = oag_formats::pvs::governing_sections(blob, &nodes).ok()?;
        let (sections, placement) = DrawSections::place(model, &governing, &pvs);
        let swaps = SwapConflicts::find(&pvs, &sections, model);
        Some(Self {
            pvs,
            padding: SectionPadding::from_track(ai),
            sections,
            swaps,
            placement,
        })
    }

    /// How many authored LOD-swap pairs the track carries, for the load
    /// report.
    #[must_use]
    pub fn swap_pairs(&self) -> usize {
        self.swaps.pair_count()
    }

    /// What may be drawn with the craft in `craft` and the camera in `camera`.
    #[must_use]
    pub(super) fn set(&self, craft: u8, camera: u8) -> VisibleSet {
        VisibleSet::around(&self.pvs, &self.padding, &self.swaps, craft, camera)
    }
}

/// How far off the nearest spline sample a point may be and still be trusted to
/// name a section, as a multiple of the track's widest half-width.
///
/// **The conservative path exists because a craft can leave the partition, and
/// being outside it is not the same as looking up an id the track does not
/// have.** A craft that has fallen off, is airborne over a gap, or has been
/// knocked into scenery is usually still inside *some* authored box, so it gets
/// a valid answer that is simply wrong for what the camera is now framing - the
/// all-ones fallback never fires on its own. Culling to a stale section exactly
/// when the craft is somewhere unusual is the most visible way this could fail,
/// so distance to the racing line gates it instead.
///
/// Three half-widths is deliberately loose: it must not trip during ordinary
/// wide cornering, only when the craft is somewhere the authored partition was
/// not drawn around. Widening it costs nothing but a little culling; narrowing
/// it risks pop-in. There is no recovered value to match - the original does
/// not have this problem, because it never culls to a camera.
pub(super) const OFF_TRACK_HALF_WIDTHS: f32 = 3.0;

/// How many samples either side of the craft's own the camera is looked for in.
///
/// At four samples per control-point interval this is a couple of dozen
/// intervals of track, far more than a chase camera trails by, and about
/// thirty-five times cheaper than the whole-table scan it replaces. Missing is
/// safe - see [`Spline::nearest_within`].
pub(super) const CAMERA_SEARCH_SAMPLES: usize = 96;

/// Whether `draw` should be submitted, in two tiers.
///
/// **The authored PVS first, the frustum second**, which is the ordering the
/// ADR is about: the mask test is an integer `and` and the frustum test is six
/// plane-versus-sphere evaluations, so the cheap one has to run first for the
/// second tier to see less work. Delegated to `oag_render::pvs` so the ordering
/// lives with the types it operates on rather than being re-established at each
/// call site.
///
/// `set` is `None` when `[graphics] pvs_culling` is off or the track has no
/// sections; `frustum` is `None` per [`Drawable::draw`].
pub(super) fn visible(
    draw: &DrawCall,
    sections: u64,
    set: Option<&VisibleSet>,
    frustum: Option<&Frustum>,
) -> bool {
    oag_render::pvs::visible(draw, sections, set, frustum)
}
