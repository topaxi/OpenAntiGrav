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
    /// Whether the motion-blur chain was actually encoded this frame.
    ///
    /// **Not a statistic, and it is here because it has to travel the same
    /// path.** A caller that claimed a `PassTimer` slot for the blur needs to
    /// know whether the chain wrote the timestamps into it: at `off`, and on a
    /// frame whose scratch targets have not been built yet, `MotionBlur::render`
    /// encodes nothing, and a claimed-but-unwritten slot never comes back. Four
    /// of those end measurement for the run - see `PassTimer::begin`. The
    /// caller answers that with `PassTimer::abandon`, and this is what tells it
    /// to.
    pub blur_encoded: bool,
    /// Whether the HD/Fury bloom chain was actually encoded this frame.
    ///
    /// The same shape as [`Self::blur_encoded`] and for the same reason, with
    /// one difference worth stating: `hd_bloom::Chain::run` has no early
    /// return, so a caller that claims a slot only when a scene actually holds
    /// a `Chain` (see `race::scene::frame::Scene::hd`) never needs the
    /// abandon path in the running case - this field earns its keep on the
    /// PSP/Pure titles and any HD circuit with no `HDR and Bloom` block, where
    /// no `Chain` exists to claim a slot for at all.
    pub hd_bloom_encoded: bool,
}

impl SceneStats {
    pub(super) fn add(&mut self, other: Self) {
        self.draws_submitted += other.draws_submitted;
        self.draws_culled += other.draws_culled;
        self.triangles += other.triangles;
        // Or-ed rather than summed: they are the two fields here that are not
        // counts, and "any part of this frame encoded the chain" is the
        // question a timestamp claim is asking.
        self.blur_encoded |= other.blur_encoded;
        self.hd_bloom_encoded |= other.hd_bloom_encoded;
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
    pub(super) pvs: oag_vex::pvs::TrackPvs,
    padding: SectionPadding,
    pub(super) sections: DrawSections,
    /// Mask-exclusive section pairs with overlapping geometry - authored
    /// LOD swaps, which the per-frame union must never re-join. See
    /// [`SwapConflicts`].
    swaps: SwapConflicts,
    /// What the association rule managed, for the load report.
    pub placement: PlacementStats,
    /// Wipeout HD's partition, which is a different mechanism rather than the
    /// same one in another file.
    ///
    /// A PSP track authors up to 64 `section` nodes and a 64-bit mask per
    /// section; an HD circuit authors hundreds of cells along its own racing
    /// line and **one bit per `.rcsmodel` chunk** per cell. The two never
    /// coexist - a `.vex` with `section` nodes has no chunks and a PS3
    /// circuit has no `section` nodes - so this sits beside the PSP fields
    /// rather than in an enum with them, and whichever is populated is the
    /// tier that runs. See [`oag_rcs::hd_pvs`] and
    /// `docs/formats/hd-pvs.md`.
    chunks: Option<oag_rcs::hd_pvs::Pvs>,
}

impl TrackVisibility {
    /// Reads the `section` nodes of a track and places its draw calls in them.
    ///
    /// Returns `None` when the track declares no sections at all - which is
    /// every Pure track, since Pure does not share Pulse's class numbering.
    /// The caller then draws with no first tier, exactly as before.
    #[must_use]
    pub fn build(model: &Model, blob: &[u8], ai: &AiTrack) -> Option<Self> {
        let nodes = oag_vex::vex::nodes(blob).ok()?;
        let pvs = oag_vex::pvs::TrackPvs::from_nodes(blob, &nodes).ok()?;
        if pvs.is_empty() {
            return None;
        }
        // The authored association: a `section` node governs its parent's
        // whole subtree, so each draw call inherits its scene node's group.
        // This is what hides a far-LOD copy of the track while racing on the
        // real one - see `oag_render::pvs`.
        let governing = oag_vex::pvs::governing_sections(blob, &nodes).ok()?;
        let (sections, placement) = DrawSections::place(model, &governing, &pvs);
        let swaps = SwapConflicts::find(&pvs, &sections, model);
        Some(Self {
            pvs,
            padding: SectionPadding::from_track(ai),
            sections,
            swaps,
            placement,
            chunks: None,
        })
    }

    /// The Wipeout HD partition, read from the `track.pvs` beside the circuit's
    /// `.vex`.
    ///
    /// A separate constructor rather than a branch inside [`Self::build`],
    /// because the two share nothing: this one has no `section` nodes to walk,
    /// no draw-call placement to derive and no swap relation to recover - the
    /// association it needs is the chunk index each draw call already carries
    /// out of `oag_render::mesh::rcs`.
    ///
    /// **A circuit with no readable `.pvs` draws every chunk**, which is what
    /// this project did before the file was decoded: slower, and showing
    /// scenery the artists hid, but never missing anything. Both outcomes are
    /// pushed onto `report` so a race says which one it got.
    pub fn from_hd_pvs(
        archives: &mut oag_assets::Archives,
        track: &str,
        report: &mut Vec<String>,
    ) -> Option<Self> {
        let name = pvs_name(track);
        let blob = match archives.read_name(&name) {
            Ok(blob) => blob,
            Err(_) => {
                report.push(format!("{name}: not in this source - drawing every chunk"));
                return None;
            }
        };
        let pvs = match oag_rcs::hd_pvs::Pvs::parse(&blob) {
            Ok(pvs) => pvs,
            Err(e) => {
                report.push(format!("{name}: {e} - drawing every chunk"));
                return None;
            }
        };
        report.push(format!(
            "{name}: {} visibility cell(s) over {} chunk(s){}",
            pvs.cells(),
            pvs.chunks(),
            match pvs.trailing() {
                0 => String::new(),
                n => format!(", plus {n} trailing byte(s) this parser does not read"),
            },
        ));
        Some(Self {
            pvs: oag_vex::pvs::TrackPvs::empty(),
            padding: SectionPadding::default(),
            sections: DrawSections::default(),
            swaps: SwapConflicts::none(),
            placement: PlacementStats::default(),
            chunks: Some(pvs),
        })
    }

    /// The circuit's chunk partition, when it has one.
    #[must_use]
    pub fn chunks(&self) -> Option<&oag_rcs::hd_pvs::Pvs> {
        self.chunks.as_ref()
    }

    /// Whether the PSP's section tier has anything to say.
    #[must_use]
    pub fn has_sections(&self) -> bool {
        !self.pvs.is_empty()
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
/// sections; `chunks` is `None` for anything that is not a PS3 circuit, or
/// when neither viewpoint could be located in the circuit's `track.pvs`;
/// `frustum` is `None` per [`Drawable::draw`].
pub(super) fn visible(
    draw: &DrawCall,
    sections: u64,
    set: Option<&VisibleSet>,
    chunks: Option<&ChunkSet>,
    frustum: Option<&Frustum>,
) -> bool {
    oag_render::pvs::visible(draw, sections, set, chunks, frustum)
}

/// The `track.pvs` entry name beside a `track.vex` one.
///
/// A path rewrite, the way `oag_render::mesh::rcs::sibling_name` finds the
/// `.rcsmodel`: the disc pairs the two by name in one directory, and
/// `track_reversed.vex` has a `track_reversed.pvs` of its own.
fn pvs_name(vex_name: &str) -> String {
    match vex_name.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.pvs"),
        None => format!("{vex_name}.pvs"),
    }
}
