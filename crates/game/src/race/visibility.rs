//! What the frame decided not to draw: the culling counters the performance
//! overlay reads, and the track sections a camera can see from where it is.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// What one frame's frustum culling did, for the performance overlay.
///
/// Plain counts rather than anything richer: the point is to show the effect
/// of `--lod` and frustum culling is having, not to profile the
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
        geometry_name: Option<&str>,
        model_chunks: Option<usize>,
        report: &mut Vec<String>,
    ) -> Option<Self> {
        let name = pvs_name(track, geometry_name);
        let blob = match archives.read_name(&name) {
            Ok(blob) => blob,
            Err(_) => {
                report.push(format!("{name}: not in this source - drawing every chunk"));
                return None;
            }
        };
        let pvs = match oag_rcs::hd_pvs::Pvs::parse_detect(&blob) {
            Ok(pvs) => pvs,
            Err(e) => {
                report.push(format!("{name}: {e} - drawing every chunk"));
                return None;
            }
        };
        // **A partition of a different model is worse than none.** The 2048
        // lineage's chunk is a mesh object, and four of the Vita's DLC1
        // `_reversed` files declare a chunk count that is not their sibling
        // model's (1,015 against 1,258 on Anulpha Pass) and show no spatial
        // signal at all - culling by one would hide arbitrary scenery. HD's
        // files all agree, and its caller passes `None`.
        if let Some(model) = model_chunks
            && model != pvs.chunks()
        {
            report.push(format!(
                "{name}: declares {} chunk(s) but its model has {model} mesh object(s), so it \
                 is not this model's partition - drawing every chunk",
                pvs.chunks(),
            ));
            return None;
        }
        report.push(format!(
            "{name}: {} visibility cell(s) over {} chunk(s){}{}",
            pvs.cells(),
            pvs.chunks(),
            match pvs.dialect() {
                oag_rcs::hd_pvs::Dialect::Ps3 => "",
                oag_rcs::hd_pvs::Dialect::Psp2 => " (mesh objects, little-endian layout)",
            },
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

    /// What may be drawn with the craft in `craft` and the camera in `camera`,
    /// narrowed to the sections whose authored box `view_projection` reaches -
    /// the original's second tier, see `oag_render::pvs::sections_in_view`.
    #[must_use]
    pub(super) fn set(
        &self,
        craft: u8,
        camera: u8,
        view_projection: &oag_core::math::Mat4,
    ) -> VisibleSet {
        VisibleSet::around(&self.pvs, &self.padding, &self.swaps, craft, camera)
            .within_view(&self.pvs, view_projection)
    }

    /// What may be drawn with the camera placed by `section` alone: that section's own row,
    /// with no padding and no second viewpoint, as the original masks a camera that stands on
    /// an authored station ([`Race::station_camera_section`]).
    #[must_use]
    pub(super) fn set_exact(
        &self,
        section: u8,
        view_projection: &oag_core::math::Mat4,
    ) -> VisibleSet {
        VisibleSet::around(
            &self.pvs,
            &SectionPadding::default(),
            &self.swaps,
            section,
            section,
        )
        .within_view(&self.pvs, view_projection)
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

/// The `.pvs` entry name for a circuit.
///
/// **Beside the `.rcsmodel` that was actually found when there is one**, the
/// way `psp2_animation` finds the skeleton: the Omega Collection's
/// `track.final.rcsmodel` is paired with `track.final.pvs`, and
/// `track_reversed.vex`'s own is `track_reversed.final.pvs`. Deriving it from
/// the `.vex` name asked for `track.pvs` and `track_reversed.pvs`, which that
/// archive does not have, so neither direction of any circuit was ever read.
/// HD's `track.rcsmodel` gives `track.pvs`, the name the `.vex` gave.
///
/// Without a model name it is a path rewrite of the `.vex` one, as before.
fn pvs_name(vex_name: &str, geometry_name: Option<&str>) -> String {
    let beside = geometry_name.and_then(|model| {
        let at = model.len().checked_sub(".rcsmodel".len())?;
        model[at..]
            .eq_ignore_ascii_case(".rcsmodel")
            .then(|| format!("{}.pvs", &model[..at]))
    });
    beside.unwrap_or_else(|| match vex_name.rsplit_once('.') {
        Some((stem, _)) => format!("{stem}.pvs"),
        None => format!("{vex_name}.pvs"),
    })
}

#[cfg(test)]
mod name_tests {
    use super::pvs_name;

    #[test]
    fn the_pvs_sits_beside_the_model_that_was_found() {
        // Omega's baked outputs carry a `.final` infix, forward and reversed.
        assert_eq!(
            pvs_name(
                r"Data\environments\tech_de_ra\track_reversed.vex",
                Some(r"Data\environments\tech_de_ra\track_reversed.final.rcsmodel"),
            ),
            r"Data\environments\tech_de_ra\track_reversed.final.pvs"
        );
        // HD and 2048 use the plain name, which the `.vex` gave as well.
        assert_eq!(
            pvs_name("a/track.vex", Some("a/track.rcsmodel")),
            pvs_name("a/track.vex", None)
        );
        assert_eq!(pvs_name("a/track.vex", None), "a/track.pvs");
        // A model name that is not a `.rcsmodel` falls back to the `.vex`.
        assert_eq!(pvs_name("a/track.vex", Some("a/track.bin")), "a/track.pvs");
        assert_eq!(pvs_name("a/track.vex", Some("l")), "a/track.pvs");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::{Vec3, camera};

    /// The race scene narrows the visible set to the sections the view reaches
    /// (`Visibility::set`'s `within_view`), on a real circuit.
    ///
    /// **The wiring, not the law**: `oag_render::pvs`'s unit tests hold the
    /// corner test, and a call site that dropped `within_view` would pass all
    /// of them. A camera on the racing line looking along it must see strictly
    /// fewer sections than the same craft and camera with the view test off -
    /// `set` with a matrix that rejects every section falls back to the
    /// unnarrowed set, which is the reference.
    #[test]
    #[ignore = "needs a disc image in data/images/"]
    fn the_visible_set_is_narrowed_to_the_view_on_a_real_circuit() {
        let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
            return;
        };
        let entry = r"Data\Environments\16_Track\track.vex";
        let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
        let blob = archives.read_name(entry).expect("track.vex");
        let model = oag_render::mesh::build_with_textures(entry, &blob, None).expect("model");
        let nodes = oag_vex::vex::nodes(&blob).expect("nodes");
        let node = oag_vex::track::find_node(&blob, &nodes).expect("a WO Track node");
        let ai = oag_vex::track::parse(&blob[node.payload()]).expect("spline");
        let visibility =
            TrackVisibility::build(&model, &blob, &ai).expect("the circuit's sections");
        assert!(visibility.has_sections());

        let points: Vec<_> = ai.paths.iter().flat_map(|p| p.points.iter()).collect();
        let point = points[points.len() / 3];
        let (pos, forward) = (
            Vec3::from_array(point.pos),
            Vec3::from_array(point.tangent).normalize(),
        );
        let eye = pos - forward * 10.0 + Vec3::Y * 3.0;
        let projection = camera::perspective(60f32.to_radians(), 480.0 / 272.0, 1.0, 2500.0);
        let view_projection = projection * camera::look_at(eye, pos + forward * 20.0, Vec3::Y);

        let ids: Vec<u8> = visibility.pvs.ids().collect();
        let allowed = |set: &VisibleSet| ids.iter().filter(|&&id| set.allows(1u64 << id)).count();
        let section = point.section_id;
        let narrowed = allowed(&visibility.set(section, section, &view_projection));
        let reference = allowed(&visibility.set(section, section, &Mat4::ZERO));
        assert!(
            narrowed < reference,
            "the view test removed nothing: {narrowed} of {reference} sections"
        );
        assert!(
            VisibleSet::allows(
                &visibility.set(section, section, &view_projection),
                1u64 << section
            ),
            "the craft's own section was narrowed away"
        );
    }
}
