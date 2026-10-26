//! Both shadow tiers' per-frame geometry: where each craft's blob quad goes,
//! and what its authored hull projects to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/craft.rs` and
//! `frame/attachments.rs` already set; a move, with no behaviour change.

use crate::Uploads;
use crate::drawable::HullLayers;
use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_render::shadow::Placement;

use crate::Race;
use crate::scene::FrameWork;
use crate::scene::motion::Snapshot;
use crate::scene::viewports::Layer;

/// How dark a covered texel draws where there is no sun to take out.
///
/// **Not Wipeout HD's number, and it no longer reaches an HD race.** The
/// original's shadowed track redraw is the road lit *without its sun* blended in
/// by the map's coverage (`Shadow_CompileShadowedTrackRedraw`,
/// [`shadow-track-redraw.md`](../../../../../docs/ghidra/functions/ps3-hdfury-eu/shadow-track-redraw.md)),
/// and `mesh`'s `shadowed` does exactly that wherever an authored light rig is
/// on, which is every HD circuit - there is no darkness constant in it. This
/// multiply is what remains for a scene with no rig, where there is no sun to
/// remove. **Chosen, not measured.**
const MAP_STRENGTH: f32 = 0.5;

/// How dark the `mapped` tier draws a shadowed pixel.
///
/// **Ours, like everything else in that tier**, which is this project's own
/// and has no original to be faithful to. Lighter than [`MAP_STRENGTH`]
/// because it applies to *every* surface rather than to the road alone: a
/// craft's own hull shadowed at the same weight reads as a black panel.
const MAPPED_STRENGTH: f32 = 0.4;

/// How far a receiver is pushed towards the light before its depth is
/// compared, in the map's own `0..1` units.
///
/// **Ours, and small on purpose**: the slope-scaled bias in the caster
/// pipeline (`shadow::map::Map::new`) does the work, because it can see how
/// steeply a surface runs away from the light and a shader-side constant
/// cannot. This is the floor under it, for the flat cases that slope scaling
/// leaves at zero.
const MAPPED_DEPTH_BIAS: f32 = 0.0004;

/// How far ahead of the player the `mapped` tier's near box is centred, and how
/// wide it is.
///
/// **Ours.** The near layer spends its texels around the craft and the road
/// just ahead of it, where a shadow is largest on screen.
const MAPPED_AHEAD: f32 = 40.0;
const MAPPED_RADIUS: f32 = 70.0;

/// The same for the far layer: a box wide enough to reach the road towards the
/// horizon, centred far enough ahead that the near box sits inside it with
/// room behind the craft for a chase camera's view down to the road.
///
/// **Ours.** At [`oag_render::shadow::map::DEPTH_SIZE`] texels across, a texel
/// here is about 0.4 units against the near layer's 0.07 - coarse up close,
/// which is why the near layer covers that, and finer than a pixel at the
/// distance this layer is read from. Without it a receiver past the near box
/// read lit, so under a long roof the shadow ended in a straight line that
/// moved with the craft.
const MAPPED_FAR_AHEAD: f32 = 280.0;
const MAPPED_FAR_RADIUS: f32 = 400.0;

impl super::super::Scene {
    /// Both shadow maps, once for a frame, ahead of its first view.
    ///
    /// **A split screen's later views do not redraw them** ([`Layer::Next`], and a later
    /// window of a multi-window race, [`Scene::does_frame_work`]), bar
    /// one: the maps are a function of the craft (their model matrices and the
    /// circuit's own sun or fixed light axis), never of the camera. The casters
    /// are fixed buffers and ranges (`Drawable::caster` draws `opaque_ranges`,
    /// which no `LodGroup` switch touches) and the occlusion fit is each craft's
    /// own. The scene uniform's shadow block and the per-craft layer choice are CPU
    /// state that outlives the call, so a later view reads what the first view
    /// drew; its picture is the one it drew when it redrew them. **The `mapped`
    /// tier's depth map is fitted on the craft its view follows**, so each later
    /// view redraws that one, alone.
    pub(super) fn render_shadow_passes(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        shadows: oag_display::display::Shadows,
        pvs_cull: bool,
    ) {
        let shadows = self.resolve_shadows(shadows);
        if !self.does_frame_work() {
            // The `mapped` tier's map is fitted on the craft the view follows, so
            // it is the one pass a later view does itself. Each view is its own
            // submission, and a queue write lands before the submission that
            // follows it, so this view's fit and caster uniforms cannot reach an
            // earlier view's draws.
            if shadows == oag_display::display::Shadows::Mapped {
                let mut map = self.shadow_map.borrow_mut();
                self.render_depth_map(&mut map, queue, encoder, race);
            }
            return;
        }
        self.frame_work.update(|w| FrameWork {
            shadow_maps: w.shadow_maps + 1,
            ..w
        });
        self.update_ray_scene(encoder, race, pvs_cull);
        self.render_shadow_map(queue, encoder, race, shadows);
        self.render_sun_occlusion(queue, encoder, race, shadows);
    }

    /// What the camera-independent work has run so far.
    #[cfg(test)]
    pub(crate) fn frame_work(&self) -> FrameWork {
        self.frame_work.get()
    }

    /// Each craft's shadow map record as the last frame left it, for the tests.
    #[cfg(test)]
    pub(crate) fn craft_shadows(
        &self,
    ) -> [oag_mesh::mesh_render::CraftShadow; oag_render::shadow::map::MAX_CASTERS] {
        self.shadow_map.borrow().craft_shadows()
    }

    /// Gathers this frame's blob quads and projected hulls and uploads them with
    /// the view's camera matrix.
    ///
    /// The gather (`race.shadow_placements`, and for the `original` tier a hull
    /// projection that casts a ray per vertex onto the floor) is a function of
    /// the craft and the circuit alone, so a split screen does it on its first
    /// view and every later view uploads what was gathered; only the matrix
    /// differs. Measured: it was about half of a view's CPU on Pulse's default
    /// tier.
    pub(super) fn upload_shadow_geometry(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        vp: &[[f32; 4]; 4],
        race: &Race,
        shadows: oag_display::display::Shadows,
    ) {
        let shadows = self.resolve_shadows(shadows);
        let mut gathered = self.shadow_gathered.borrow_mut();
        if self.does_frame_work() {
            *gathered = self.shadow_geometry(race, shadows);
        }
        let (quads, hull_vertices) = &*gathered;
        self.shadow
            .borrow_mut()
            .upload(queue, vp, quads, hull_vertices);
    }

    /// The quads and the projected hulls this frame draws, for the tier the
    /// caller is on.
    ///
    /// Both lists are empty at `off`, and each tier fills exactly one of them:
    /// they are alternatives, not layers.
    pub(super) fn shadow_geometry(
        &self,
        race: &Race,
        shadows: oag_display::display::Shadows,
    ) -> (Vec<Placement>, Vec<GpuVertex>) {
        // The blob shadows, gathered here with the rest of the per-frame
        // geometry and uploaded whether or not the tier is on - `upload` with
        // no placements clears the runs, which is what makes turning the row
        // off take effect on the next frame rather than leaving the last
        // frame's quads in the buffer.
        if shadows == oag_display::display::Shadows::Blob
            && self.generated_silhouettes > 0
            && !self.generated_silhouettes_said.replace(true)
        {
            log::warn!(
                "blob shadow: {} of {} slot(s) draw a generated falloff, which is this \
                 project's and not the disc's - the disc ships no silhouette for those craft",
                self.generated_silhouettes,
                self.shadow_silhouette_count
            );
        }
        // The `original` tier draws the craft's own authored hull instead of a
        // textured quad, projected onto the same surface the placement found:
        // gathered once per tick and lerped between ticks, see `hull_cache`. A
        // slot whose model authors no hull contributes nothing rather than
        // falling back to a blob: a tier that silently becomes another tier is
        // how a missing feature stops being noticed. See
        // `oag_render::shadow::hull_triangles`, and
        // `oag_pulse::shadow::WORLD_AXIS` for the direction.
        if shadows == oag_display::display::Shadows::Original {
            // The quads are not drawn at all: the two tiers are alternatives,
            // not layers.
            let axis = Vec3::from_array(oag_pulse::shadow::WORLD_AXIS);
            return (Vec::new(), self.original_hulls(race, axis));
        }
        if shadows == oag_display::display::Shadows::Blob {
            (race.shadow_placements(), Vec::new())
        } else {
            (Vec::new(), Vec::new())
        }
    }
}

impl super::super::Scene {
    /// The scene uniform's shadow block for this frame.
    ///
    /// [`ShadowMap::off`] unless the `original` tier is on *and* the map has
    /// something in it: a strength of zero is what makes the sample every
    /// pipeline carries inert, so a title with no casters draws exactly as it
    /// did before the map existed.
    pub(super) fn shadow_uniform(
        &self,
        shadows: oag_display::display::Shadows,
    ) -> oag_mesh::mesh_render::ShadowMap {
        let shadows = self.resolve_shadows(shadows);
        let map = self.shadow_map.borrow();
        match shadows {
            // The sun is the one `mapped` fits (`mapped_light`), carried in the matrix's
            // depth row for the shader to read; there is no map and no bias.
            oag_display::display::Shadows::Traced => oag_mesh::mesh_render::ShadowMap {
                matrix: mapped_fit(Mat4::IDENTITY, mapped_light(&self.light))
                    .matrix()
                    .to_cols_array_2d(),
                strength: MAPPED_STRENGTH,
                mode: oag_mesh::mesh_render::ShadowMap::TRACED,
                traced_rays: self.traced_rays(),
                ..oag_mesh::mesh_render::ShadowMap::off()
            },
            oag_display::display::Shadows::Original if map.casters() > 0 => {
                oag_mesh::mesh_render::ShadowMap {
                    // Each craft's own matrix is in `craft_shadow`; this
                    // block's is the `mapped` tier's alone.
                    matrix: Mat4::IDENTITY.to_cols_array_2d(),
                    strength: MAP_STRENGTH,
                    mode: oag_mesh::mesh_render::ShadowMap::COVERAGE,
                    ..oag_mesh::mesh_render::ShadowMap::off()
                }
            }
            oag_display::display::Shadows::Mapped if map.depth_casters() > 0 => {
                oag_mesh::mesh_render::ShadowMap {
                    matrix: map.depth_matrix().to_cols_array_2d(),
                    strength: MAPPED_STRENGTH,
                    mode: oag_mesh::mesh_render::ShadowMap::DEPTH,
                    depth_bias: MAPPED_DEPTH_BIAS,
                    far_depth_bias: far_depth_bias(),
                    far_matrix: map.far_matrix().to_cols_array_2d(),
                    ..oag_mesh::mesh_render::ShadowMap::off()
                }
            }
            // Every other case, `off` included: a strength of zero is what
            // makes the sample every pipeline carries inert.
            _ => oag_mesh::mesh_render::ShadowMap::off(),
        }
    }

    /// Renders every craft into the shadow map, for a title whose `original`
    /// tier is a shadow map.
    ///
    /// **Wipeout HD only, and by data rather than by name**: the casters are
    /// the craft, the receiver is the track, and the tier draws nothing at all
    /// where the craft author their own occluder hulls instead - that is
    /// Pulse's mechanism and `hull_triangles` is where it lives. A title with
    /// neither gets an empty map and the report has already said so.
    ///
    /// Returns how many casters it drew, so a caller can tell a cleared map
    /// from one that was never rendered.
    pub(super) fn render_shadow_map(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        shadows: oag_display::display::Shadows,
    ) -> usize {
        let mut map = self.shadow_map.borrow_mut();
        if shadows == oag_display::display::Shadows::Mapped {
            return self.render_depth_map(&mut map, queue, encoder, race);
        }
        // A hull the craft authors is Pulse's mechanism; if any slot has one,
        // this title shadows that way and not this one.
        let hulls = self.shadow_hulls.iter().any(Option::is_some);
        if shadows != oag_display::display::Shadows::Original || hulls {
            map.render(queue, encoder, &[]);
            map.render_depth(queue, encoder, &default_fit(), &[]);
            map.render_depth_far(queue, encoder, &default_fit(), &[]);
            return 0;
        }
        let drawn = usize::from(race.ship_count());
        let references = self.rank_references(race);
        let models: Vec<(usize, Mat4)> = (0..drawn.min(self.ships.len()))
            .filter(|&slot| race.ship_active(slot))
            .map(|slot| (slot, race.ship_model_matrix_of(slot)))
            .collect();
        let ranked = rank_by_distance(
            models
                .iter()
                .map(|&(slot, model)| (slot, model.transform_point3(Vec3::ZERO))),
            &references,
        );
        let crafts: Vec<_> = ranked
            .iter()
            .enumerate()
            .filter_map(|(rank, slot)| {
                let model = models.iter().find(|(held, _)| held == slot)?.1;
                Some(oag_render::shadow::map::CraftCaster {
                    slot: *slot,
                    rank,
                    // The box the sun-occlusion map uses, which the original
                    // shares between a ship's maps, around the craft alone.
                    fit: craft_fit(model, &self.light),
                    caster: self.ships[*slot].caster(model),
                })
            })
            .collect();
        map.render(queue, encoder, &crafts);
        crafts.len()
    }

    /// The points each craft's map rank is measured from, this frame.
    ///
    /// **The camera's eye, in the original** (`Ship_RankAndAssignMapSlots` ranks
    /// by squared distance to a vec4 its job carries, which is the eye at
    /// confidence 62 and only that). The ranking is one per frame, not one per
    /// view, so a split's later views keep reading what the first drew
    /// ([`Self::render_shadow_passes`]); in a race with more than one person it
    /// is therefore the nearest of this view's eye and every person's craft, so
    /// that each seat's own craft takes one of the two 512 squares. **Chosen,
    /// not measured**: the original has one camera.
    fn rank_references(&self, race: &Race) -> Vec<Vec3> {
        let mut points = vec![race.camera_position()];
        if race.people() > 1 {
            points.extend(
                race.sim
                    .world
                    .human_slots()
                    .map(|slot| race.ship_model_matrix_of(slot).transform_point3(Vec3::ZERO)),
            );
        }
        points
    }
}

/// The slots of `craft`, nearest first by squared distance to the nearest of
/// `references`, as `Ship_RankAndAssignMapSlots` sorts them; the slot breaks a
/// tie, so two equidistant craft keep their order frame to frame instead of
/// trading squares.
fn rank_by_distance(
    craft: impl IntoIterator<Item = (usize, Vec3)>,
    references: &[Vec3],
) -> Vec<usize> {
    let mut ranked: Vec<(usize, f32)> = craft
        .into_iter()
        .map(|(slot, at)| {
            let nearest = references
                .iter()
                .map(|point| (at - *point).length_squared())
                .fold(f32::INFINITY, f32::min);
            (slot, nearest)
        })
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    ranked.into_iter().map(|(slot, _)| slot).collect()
}

/// The box a craft's shadow maps are fitted to: [`oag_render::shadow::map::Fit::craft`]
/// around the craft, along the circuit's own sun - which HD authors in its
/// `.envsettings` and this renderer already lights through, not a direction of
/// this module's own.
fn craft_fit(model: Mat4, light: &oag_mesh::mesh_render::Light) -> oag_render::shadow::map::Fit {
    oag_render::shadow::map::Fit::craft(
        model.transform_point3(Vec3::ZERO),
        Vec3::from_array(light.direction),
    )
}

/// The environment variable that, naming a file, writes the player's
/// sun-occlusion map out beside a `--screenshot` capture - see
/// `Scene::dump_offscreen_if_asked`.
const DUMP_VAR: &str = "OAG_DUMP_SUN_OCCLUSION";

/// The same for the craft shadow atlas, every craft's map in its rank's square -
/// see `Scene::dump_offscreen_if_asked`, which writes all three.
const DUMP_SHADOW_MAP_VAR: &str = "OAG_DUMP_SHADOW_MAP";

/// The same for the player's self-shadow depth map - see
/// `Scene::dump_offscreen_if_asked`, which writes all three.
const DUMP_SELF_SHADOW_VAR: &str = "OAG_DUMP_SELF_SHADOW";

impl super::super::Scene {
    /// Renders each active craft's sun-occlusion map - the track within
    /// `occlusion::RADIUS` of the craft, from the sun, as its own baked mask -
    /// and its self-shadow depth map from the same box.
    ///
    /// **Wipeout HD's `original` tier, and by data rather than by name**: it
    /// needs the circuit's own sun (`light.enabled`, which only an authored
    /// `.envsettings` rig sets) and a title that shadows by map rather than
    /// by authored hull. Anywhere else every layer is cleared and no craft
    /// names one, so the hull draws exactly as before the map existed. See
    /// `oag_render::shadow::occlusion` and
    /// `docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md`.
    ///
    /// The per-craft layer is the craft's grid slot, and
    /// [`Self::sun_occlusion_layer`] is what turns a rendered layer into a
    /// uniform field.
    pub(super) fn render_sun_occlusion(
        &self,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        shadows: oag_display::display::Shadows,
    ) {
        let mut maps = self.sun_occlusion.borrow_mut();
        let hulls = self.shadow_hulls.iter().any(Option::is_some);
        // `mapped` keeps the road's half: that tier casts nothing static, so a
        // craft under a deck would read full sun where the road around it is
        // dark, and this map is the title's own answer to exactly that - the
        // road's baked shadow under the craft, read off the disc's lightmap.
        let mapped = shadows == oag_display::display::Shadows::Mapped;
        let active = (shadows == oag_display::display::Shadows::Original || mapped)
            && !hulls
            && self.light.enabled > 0.5;
        let drawn = usize::from(race.ship_count());
        let track = self.track.occlusion_track();
        for slot in 0..(oag_mesh::mesh_render::OCCLUSION_LAYERS as usize) {
            if self.ships.get(slot).is_none() || !active || slot >= drawn || !race.ship_active(slot)
            {
                // Cleared once, not every frame: a layer nothing drew into
                // since its last clear is already black, and a title that
                // never renders these must not pay eight clears a frame for
                // a map its hulls never sample.
                if maps.drawn(slot) > 0 {
                    maps.clear(encoder, slot);
                }
                continue;
            }
            let model = race.ship_model_matrix_of(slot);
            // Fitted to the craft alone, in the box the coverage maps use
            // ([`oag_render::shadow::map::Fit::craft`]). **Not the drawable's
            // bounding radius**: an HD `Ship.vex` measures over 200 units
            // across by that, its authored geometry reaching far past the
            // hull, and a box that size is a map of the neighbourhood rather
            // than of the craft.
            let fit = craft_fit(model, &self.light);
            maps.render(queue, encoder, slot, &fit, &track);
            // The craft's own depth map from the same box: its opaque ranges
            // and nothing else, so a wing shadows the fuselage and no other
            // craft shadows either. See `oag_render::shadow::self_shadow`.
            // Under `mapped` the depth map already shadows the hull with
            // itself; an empty caster clears this layer to far, which reads lit,
            // so the hull is not self-shadowed twice.
            let ship = &self.ships[slot];
            let caster = if mapped {
                ship.caster_of(&[], model)
            } else {
                ship.caster(model)
            };
            maps.render_self_shadow(queue, encoder, slot, &caster);
            if slot == 0 && std::env::var_os(DUMP_VAR).is_some() {
                log::info!(
                    "sun occlusion layer 0: craft at {:?}, radius {:.1}, sun {:?}",
                    fit.centre,
                    fit.radius,
                    fit.towards_light
                );
                for line in self
                    .track
                    .describe_occlusion_draws(fit.centre, fit.towards_light.normalize_or_zero())
                {
                    log::info!("sun occlusion layer 0: {line}");
                }
            }
        }
    }

    /// Writes every active craft's model uniform for this frame, naming the
    /// sun-occlusion layer its hull samples.
    ///
    /// Slot-indexed rather than `zip`ped over `race.ship_model_matrices()`,
    /// whose filter-then-collect drops out of slot order the moment a craft
    /// below `drawn` goes inactive - see `Race::ship_active`. A tail slot past
    /// `drawn` is simply never in this range, so it keeps last frame's
    /// uniforms and is not drawn, the same as before.
    pub(super) fn write_hull_uniforms(
        &self,
        queue: &Uploads<'_>,
        race: &Race,
        view_projection: Mat4,
        prev_vp: Mat4,
        prev: &Snapshot,
    ) {
        let drawn = usize::from(race.ship_count());
        for slot in 0..drawn {
            if !race.ship_active(slot) {
                continue;
            }
            if let Some(drawable) = self.ships.get(slot) {
                // The hull's environment light: the circuit's baked probes blended at the
                // craft, one array layer per grid slot, where the circuit ships any.
                let env_layer = self.env_probes.as_ref().map(|probes| {
                    probes.write(queue, slot, race.ship_probe_position(slot));
                    slot
                });
                drawable.write_hull(
                    queue,
                    view_projection,
                    race.ship_model_matrix_of(slot),
                    prev_vp * prev.ship(slot, race),
                    HullLayers {
                        sun_occlusion: self.sun_occlusion_layer(slot),
                        scale: race.ship_hull_scale(slot),
                        point: race.ship_hull_points(slot),
                        env: env_layer,
                        env_tint: if self.zone_grade.is_some() {
                            crate::env_probes::ProbeTint::ZONE
                        } else {
                            crate::env_probes::ProbeTint::ENGINE
                        },
                    },
                );
            }
        }
        self.write_wrecks(queue, race, view_projection, prev_vp, prev);
    }

    /// Writes the player's sun-occlusion map - layer 0 - as a greyscale PNG
    /// when [`DUMP_VAR`] names a file, and its self-shadow depth map when
    /// [`DUMP_SELF_SHADOW_VAR`] does (far white, the craft darker the nearer
    /// the sun it is), for a headless capture.
    ///
    /// The maps' only other observable is a sample inside the hull's shader,
    /// so this is how "the craft went dark" is told apart from "the road under
    /// it is dark" - the same reason `OAG_RENDER_BENCH` exists for a different
    /// question.
    ///
    /// The behind-the-glass target too, when `OAG_DUMP_BEHIND_GLASS` names a
    /// file - see `Scene::dump_behind_glass_if_asked`.
    pub fn dump_offscreen_if_asked(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<()> {
        let write = |dump: &str, texels: &[u8], size: u32, what: &str| -> anyhow::Result<()> {
            let rgba: Vec<u8> = texels.iter().flat_map(|t| [*t, *t, *t, 255]).collect();
            std::fs::write(dump, oag_texture::png::encode_rgba(size, size, &rgba))
                .map_err(|error| anyhow::anyhow!("writing {dump}: {error}"))?;
            println!("wrote {dump} ({what} layer 0, {size}x{size})");
            Ok(())
        };
        if let Ok(dump) = std::env::var(DUMP_VAR) {
            let texels = self.sun_occlusion.borrow().read_back(device, queue, 0);
            let size = oag_render::shadow::occlusion::SIZE;
            write(&dump, &texels, size, "sun occlusion")?;
        }
        if let Ok(dump) = std::env::var(DUMP_SHADOW_MAP_VAR) {
            let texels = self.shadow_map.borrow().read_back(device, queue);
            let rgba: Vec<u8> = texels.iter().flat_map(|t| [*t, *t, *t, 255]).collect();
            let (width, height) = (
                oag_render::shadow::map::ATLAS_WIDTH,
                oag_render::shadow::map::ATLAS_HEIGHT,
            );
            std::fs::write(&dump, oag_texture::png::encode_rgba(width, height, &rgba))
                .map_err(|error| anyhow::anyhow!("writing {dump}: {error}"))?;
            println!("wrote {dump} (shadow atlas, {width}x{height})");
        }
        if let Ok(dump) = std::env::var(DUMP_SELF_SHADOW_VAR) {
            let maps = self.sun_occlusion.borrow();
            let depths = maps.self_shadow().read_back(device, queue, 0);
            let texels: Vec<u8> = depths.iter().map(|d| (d * 255.0) as u8).collect();
            let size = oag_render::shadow::self_shadow::SIZE;
            write(&dump, &texels, size, "self shadow")?;
        }
        self.dump_behind_glass_if_asked(device, queue)
    }

    /// The scene uniform's per-layer sun-occlusion projections, as the last
    /// [`Self::render_sun_occlusion`] left them.
    pub(super) fn sun_occlusion_matrices(
        &self,
    ) -> [[[f32; 4]; 4]; oag_mesh::mesh_render::OCCLUSION_LAYERS as usize] {
        let maps = self.sun_occlusion.borrow();
        std::array::from_fn(|layer| maps.matrix(layer).to_cols_array_2d())
    }

    /// Which sun-occlusion layer the craft in `slot` samples this frame:
    /// its own, when [`Self::render_sun_occlusion`] drew anything into it -
    /// a cleared layer counts as nothing drawn.
    ///
    /// A layer that drew nothing is left unnamed rather than sampled: the
    /// original's black clear would read as no sun there, but on this side
    /// an empty layer is far more often a craft off the circuit's authored
    /// geometry - a synthetic track, a test fixture - than a craft over a
    /// chasm, and "lit as before" is the honest answer for a map that has
    /// no road in it.
    pub(in crate::scene) fn sun_occlusion_layer(&self, slot: usize) -> Option<usize> {
        (self.sun_occlusion.borrow().drawn(slot) > 0).then_some(slot)
    }

    /// The craft the `mapped` map is fitted on: the one the view being drawn
    /// follows in a split, and slot 0 for a race that is not split, as it always
    /// was.
    fn mapped_slot(&self, race: &Race) -> usize {
        if self.layer.get() == Layer::Whole {
            0
        } else {
            race.viewed_slot()
        }
    }

    /// The `mapped` tier's fit around the craft in `slot`.
    pub(crate) fn mapped_fit_for(&self, race: &Race, slot: usize) -> oag_render::shadow::map::Fit {
        mapped_fit(race.ship_model_matrix_of(slot), mapped_light(&self.light))
    }

    /// The `mapped` tier's far-layer fit around the craft in `slot`.
    pub(crate) fn mapped_far_fit_for(
        &self,
        race: &Race,
        slot: usize,
    ) -> oag_render::shadow::map::Fit {
        mapped_far_fit(race.ship_model_matrix_of(slot), mapped_light(&self.light))
    }

    /// The projection the `mapped` depth map was last drawn with.
    #[cfg(test)]
    pub(crate) fn mapped_matrix(&self) -> Mat4 {
        self.shadow_map.borrow().depth_matrix()
    }

    /// The `mapped` tier's own pass: **what moves casts**, into a two-layer depth
    /// map, and everything receives.
    ///
    /// The casters are the craft (a wreck in place of an eliminated one), the
    /// weapon bodies in flight - rockets, mines, bombs, shurikens; the plasma
    /// ball and cannon round are light, not bodies, and cast nothing - and the
    /// circuit's animated nodes at this frame's matrices. **The static circuit
    /// casts nothing**: every title already has its shadow in the disc's own
    /// lighting (Pulse's vertex colours, the lightmaps of HD and later), so a
    /// map that cast it too darkened that shadow a second time from a sun of
    /// our choosing, and a box that held a roof ended its shadow in a straight
    /// line that moved with the craft.
    fn render_depth_map(
        &self,
        map: &mut oag_render::shadow::map::Map,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
    ) -> usize {
        let slot = self.mapped_slot(race);
        let fit = self.mapped_fit_for(race, slot);
        let far_fit = self.mapped_far_fit_for(race, slot);
        let bodies = self.moving_bodies(race);
        let nodes = self.track.node_matrices(self.anim_clock.get());
        // Each layer takes the bodies and the animated nodes whose bound reaches
        // its own box: a node's world extent is its whole authored path, so
        // only this frame's matrix says whether it is near.
        let casters_in = |fit: &oag_render::shadow::map::Fit| {
            let frustum = oag_core::math::frustum::Frustum::from_view_projection(fit.matrix());
            let mut casters: Vec<oag_render::shadow::map::Caster<'_>> = bodies
                .iter()
                .map(|(drawable, model)| drawable.caster(*model))
                .collect();
            for node in self.track.moving_casters() {
                let Some(matrix) = nodes.get(node.xform as usize - 1) else {
                    continue;
                };
                let matrix = Mat4::from_cols_array(matrix);
                if node.reaches(matrix, &frustum) {
                    casters.push(self.track.caster_of(&node.ranges, matrix));
                }
            }
            if casters.len() > oag_render::shadow::map::MAX_DEPTH_CASTERS {
                static SAID: std::sync::Once = std::sync::Once::new();
                SAID.call_once(|| {
                    log::warn!(
                        "shadows mapped: {} casters in one layer, drawing the first {}",
                        casters.len(),
                        oag_render::shadow::map::MAX_DEPTH_CASTERS
                    );
                });
            }
            casters
        };
        let casters = casters_in(&fit);
        let far_casters = casters_in(&far_fit);
        // Every range drawn from the circuit's own buffers, whatever put it in
        // the list, so a static draw cast by any route shows up here.
        #[cfg(test)]
        {
            let track = self.track.caster(Mat4::IDENTITY).vertices;
            let ranges: usize = casters
                .iter()
                .filter(|caster| std::ptr::eq(caster.vertices, track))
                .map(|caster| caster.ranges.len())
                .sum();
            self.frame_work.update(|w| FrameWork {
                track_ranges_in_depth_map: ranges as u32,
                ..w
            });
        }
        // The coverage map stays cleared while this tier is on: two maps are
        // bound at once and only one of them may have anything in it, or a
        // switch between tiers would show the other's leftovers.
        // A later view of a split finds it already cleared by the first view's.
        if self.does_frame_work() {
            map.render(queue, encoder, &[]);
        }
        map.render_depth(queue, encoder, &fit, &casters);
        map.render_depth_far(queue, encoder, &far_fit, &far_casters);
        casters.len()
    }

    /// Every moving body this frame, with the drawable it casts from and its
    /// model matrix: each active craft (its wreck once eliminated), then each
    /// live rocket, mine, bomb and shuriken.
    fn moving_bodies(&self, race: &Race) -> Vec<(&crate::Drawable, Mat4)> {
        let mut bodies = Vec::new();
        let drawn = usize::from(race.ship_count());
        for slot in 0..drawn.min(self.ships.len()) {
            let model = race.ship_model_matrix_of(slot);
            if self.is_wrecked(race, slot) {
                if let Some(wreck) = self.wrecks.hull(slot) {
                    bodies.push((wreck, model));
                }
            } else if race.ship_active(slot) {
                bodies.push((&self.ships[slot], model));
            }
        }
        let pools: [(&[crate::Drawable], Vec<Mat4>); 4] = [
            (&self.rockets, race.rocket_model_matrices()),
            (&self.mines, race.mine_model_matrices()),
            (&self.bombs, race.bomb_model_matrices()),
            (&self.plasma_blast.shuriken, race.shuriken_model_matrices()),
        ];
        for (drawables, matrices) in pools {
            bodies.extend(drawables.iter().zip(matrices));
        }
        bodies
    }
}

/// Where the `mapped` tier's depth map is centred: ahead of the player rather
/// than on it, so one cascade's texels go where the player is heading. See
/// `MAPPED_AHEAD`.
///
/// A pure function of the player's own model matrix - no camera state reaches
/// it, which is what makes "the box follows the camera" and "the box follows
/// the craft" different, testable claims rather than a matter of reading the
/// code closely enough. See `shadow::tests::mapped_centre` for the regression
/// guard.
///
/// **Only the horizontal heading leads, not the full 3D forward.** The
/// player's own `transform_vector3(Vec3::Z)` carries pitch as well as yaw, and
/// a craft pitches constantly - cresting a jump, diving into a dip -
/// independently of where it is actually travelling. At `MAPPED_AHEAD`'s
/// lever arm, a 10-degree pitch alone swings the lead by about 7 world units,
/// upward of a hundred texels of this tier's `shadow::map::DEPTH_SIZE` map:
/// far more than any sub-texel jitter `Fit::snapped` exists to remove, and
/// enough on its own to read as the box sliding around under the player.
///
/// **Not renormalised after flattening.** A pure heading - normalising the
/// horizontal part back to unit length - flips 180 degrees the instant the
/// craft crosses vertical pitch, since the horizontal component's *sign*
/// carries the direction on either side of it: at 89 degrees of pitch it
/// points one way, at 91 the other, with nothing between to interpolate
/// through. Pulse has loops and corkscrews, so a craft passes through
/// vertical - that flip would swing the centre by a full `2 *
/// MAPPED_AHEAD`, further than the pitch bug this replaces. Leaving the
/// projection unnormalised instead makes its own magnitude - not its sign -
/// carry the craft through vertical: the lead foreshortens smoothly to zero
/// as pitch approaches 90 degrees (`cos(pitch) * MAPPED_AHEAD`, about nine
/// texels of shortening per ten degrees) and grows back out the other side,
/// with no discontinuity anywhere. It still never adds a vertical
/// component - the lead's own `y` is always zero, pitch or no pitch.
fn mapped_centre(player: oag_core::math::Mat4) -> Vec3 {
    let position = player.transform_point3(Vec3::ZERO);
    let forward = player.transform_vector3(Vec3::Z).normalize_or_zero();
    let heading = Vec3::new(forward.x, 0.0, forward.z);
    position + heading * MAPPED_AHEAD
}

/// The `mapped` tier's own fit, built and snapped in one place - so a caller
/// cannot reach `render_depth` with an unsnapped `Fit` by skipping a step.
///
/// Snapped against `shadow::map::DEPTH_SIZE`, the map this tier actually
/// fills - not `shadow::map::SIZE`, which belongs to the coverage tier and
/// breathes with its own caster grid every frame, so it has no fixed texel
/// size worth snapping against. See `Fit::snapped`.
fn mapped_fit(player: oag_core::math::Mat4, towards_light: Vec3) -> oag_render::shadow::map::Fit {
    oag_render::shadow::map::Fit {
        centre: mapped_centre(player),
        radius: MAPPED_RADIUS,
        towards_light,
    }
    .snapped(oag_render::shadow::map::DEPTH_SIZE)
}

/// Which way the `mapped` tier's light points, per title.
///
/// **The circuit's own sun where there is one**, which on Wipeout HD there is;
/// **the direction that title's own shadows are cast along** where there is
/// not, which on Pulse is `oag_pulse::shadow::WORLD_AXIS` - so switching
/// between `original` and `mapped` changes what is shadowed and not where the
/// light is.
///
/// The fallback matters more than it looks: `Light::stand_in`'s direction is
/// straight up, and a vertical light puts every craft's shadow exactly beneath
/// it, where the craft itself hides it. That is what the first capture of this
/// tier showed - a frame with shadows on the scenery and nothing under the
/// ship.
fn mapped_light(light: &oag_mesh::mesh_render::Light) -> Vec3 {
    if light.enabled != 0.0 {
        return Vec3::from_array(light.direction);
    }
    // The axis points *along* the shadow; the fit wants the direction towards
    // the light.
    -Vec3::from_array(oag_pulse::shadow::WORLD_AXIS)
}

/// The far layer's fit: [`mapped_fit`]'s lead and snap, at the far layer's
/// own distance and width.
fn mapped_far_fit(
    player: oag_core::math::Mat4,
    towards_light: Vec3,
) -> oag_render::shadow::map::Fit {
    let position = player.transform_point3(Vec3::ZERO);
    let lead = mapped_centre(player) - position;
    oag_render::shadow::map::Fit {
        centre: position + lead * (MAPPED_FAR_AHEAD / MAPPED_AHEAD),
        radius: MAPPED_FAR_RADIUS,
        towards_light,
    }
    .snapped(oag_render::shadow::map::DEPTH_SIZE)
}

/// [`MAPPED_DEPTH_BIAS`] in the far layer's own depth units.
///
/// [`oag_render::shadow::map::Fit::matrix`] makes a box's depth grow with its
/// radius, so the same world-space floor is a smaller fraction of the far
/// layer's deeper box. Scaled by the ratio, so both layers push a receiver the
/// same distance towards the light.
fn far_depth_bias() -> f32 {
    let depth = |radius: f32| radius + oag_render::shadow::map::NEAR_MARGIN;
    MAPPED_DEPTH_BIAS * depth(MAPPED_RADIUS) / depth(MAPPED_FAR_RADIUS)
}

/// The fit an empty pass uses: anywhere, one unit across, straight down.
///
/// A cleared map needs a projection for nothing to be drawn with, and this is
/// the one that cannot accidentally cover a receiver - `shadow_uniform` returns
/// `off` for the same frame regardless.
fn default_fit() -> oag_render::shadow::map::Fit {
    oag_render::shadow::map::Fit {
        centre: Vec3::ZERO,
        radius: 1.0,
        towards_light: Vec3::Y,
    }
}

#[cfg(test)]
mod tests {
    use oag_core::math::Mat4;

    use super::{
        MAPPED_AHEAD, MAPPED_DEPTH_BIAS, far_depth_bias, mapped_centre, mapped_far_fit, mapped_fit,
        rank_by_distance,
    };
    use oag_core::math::Vec3;

    /// The rank is by squared distance to the nearest reference point, nearest
    /// first, and a tie goes to the lower slot.
    #[test]
    fn craft_rank_by_distance_to_the_nearest_reference() {
        let eye = Vec3::new(0.0, 0.0, 0.0);
        let craft = [
            (0, Vec3::new(30.0, 0.0, 0.0)),
            (1, Vec3::new(5.0, 0.0, 0.0)),
            (2, Vec3::new(-5.0, 0.0, 0.0)),
            (3, Vec3::new(500.0, 0.0, 0.0)),
        ];
        // Slots 1 and 2 are equidistant, so the lower slot ranks first.
        assert_eq!(rank_by_distance(craft, &[eye]), vec![1, 2, 0, 3]);
    }

    /// Two seats in a split each pull their own craft forward: the rank is
    /// from the nearest of the references, so the far seat's craft is not left
    /// at the back of a rank taken from the near seat's eye alone.
    #[test]
    fn a_second_seat_ranks_its_own_craft_among_the_nearest() {
        let first = Vec3::new(0.0, 0.0, 0.0);
        let second = Vec3::new(1000.0, 0.0, 0.0);
        let craft = [(0, first + Vec3::X * 10.0), (1, second + Vec3::X * 5.0)];
        assert_eq!(rank_by_distance(craft, &[first]), vec![0, 1]);
        assert_eq!(rank_by_distance(craft, &[first, second]), vec![1, 0]);
    }

    /// The report under investigation was "the box follows the camera". This
    /// pins the actual dependency: [`mapped_centre`] takes a model matrix and
    /// nothing else, so no camera field exists for it to read even if some
    /// future change wanted it to. A camera-position parameter added here
    /// would be the regression this guards against - not a value this test
    /// could vary, since there is deliberately nothing to vary it with.
    #[test]
    fn the_centre_is_a_pure_function_of_the_players_own_matrix() {
        let at_origin = Mat4::from_translation(Vec3::new(3.0, 1.0, -2.0));
        assert_eq!(mapped_centre(at_origin), mapped_centre(at_origin));
    }

    /// A level player leads straight along its own forward axis.
    #[test]
    fn a_level_player_leads_along_its_forward_axis() {
        let player = Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let centre = mapped_centre(player);
        assert_eq!(centre, Vec3::new(5.0, 0.0, MAPPED_AHEAD));
    }

    /// Pitching the player - cresting a jump, diving into a dip - never adds
    /// a vertical component to the lead: whatever the pitch, the centre sits
    /// at exactly the player's own height. Before this fix the lead swung
    /// vertically by `MAPPED_AHEAD * sin(pitch)`, on the order of a hundred
    /// texels of the `mapped` tier's own map for a modest 10-degree pitch -
    /// see this module's `mapped_centre` docs for the measurement.
    #[test]
    fn pitching_the_player_never_adds_a_vertical_component_to_the_lead() {
        let position = Mat4::from_translation(Vec3::new(0.0, 12.0, 0.0));
        for pitch_degrees in [5.0_f32, 10.0, 15.0, -20.0, 45.0, 89.0, 91.0, 135.0] {
            let pitched = position * Mat4::from_rotation_x(pitch_degrees.to_radians());
            let centre = mapped_centre(pitched);
            assert!(
                (centre.y - 12.0).abs() < 1e-5,
                "pitch {pitch_degrees} put the centre at height {}, player is at 12.0",
                centre.y
            );
        }
    }

    /// A pitch does foreshorten the lead now, rather than leaving it
    /// untouched - `mapped_centre`'s own docs explain why leaving the
    /// horizontal projection unnormalised is what buys continuity through
    /// vertical pitch. The shortening tracks `cos(pitch)`.
    #[test]
    fn pitching_the_player_foreshortens_the_lead() {
        let pitched = Mat4::from_rotation_x(10.0_f32.to_radians());
        let centre = mapped_centre(pitched);
        let expected = MAPPED_AHEAD * 10.0_f32.to_radians().cos();
        assert!(
            (centre.z - expected).abs() < 1e-4,
            "centre.z was {}, expected {expected}",
            centre.z
        );
    }

    /// The fix this test guards: a heading re-normalised after being
    /// flattened to the horizontal plane flips 180 degrees the instant pitch
    /// crosses vertical, since the flattened vector's sign - not its
    /// magnitude - would carry the direction either side of it. That flip
    /// would swing the centre by a full `2 * MAPPED_AHEAD` in one frame, well
    /// past anything the pitch bug this replaces produced, and Pulse's own
    /// loops and corkscrews cross vertical pitch routinely. Leaving the
    /// projection unnormalised (see `mapped_centre`) instead lets its
    /// magnitude shrink smoothly to zero at vertical and grow back out the
    /// other side, so the centre either side of 90 degrees is close, not
    /// halfway across the map.
    #[test]
    fn pitching_through_vertical_does_not_discontinuously_flip_the_lead() {
        let just_under = mapped_centre(Mat4::from_rotation_x(89.0_f32.to_radians()));
        let just_over = mapped_centre(Mat4::from_rotation_x(91.0_f32.to_radians()));
        let jump = (just_over - just_under).length();
        assert!(
            jump < MAPPED_AHEAD * 0.5,
            "crossing vertical pitch moved the centre by {jump}, \
             a renormalised heading would move it by {}",
            MAPPED_AHEAD * 2.0
        );
    }

    /// Turning the player - the legitimate case, per this module's docs -
    /// does swing the lead: yaw is the direction actually being steered
    /// towards, unlike pitch.
    #[test]
    fn turning_the_player_does_move_the_lead() {
        let level = mapped_centre(Mat4::IDENTITY);
        let turned = mapped_centre(Mat4::from_rotation_y(90.0_f32.to_radians()));
        assert!((turned - level).length() > MAPPED_AHEAD * 0.5);
    }

    /// The chain `render_depth_map` actually calls: `mapped_fit` has to snap
    /// what `mapped_centre` builds, not just build it. `Fit::snapped` is
    /// tested on its own in `oag_render::shadow::map::tests`, but nothing
    /// short of this pins that `mapped_fit` actually calls it - deleting the
    /// `.snapped(...)` from `mapped_fit`'s body would leave every other test
    /// in this module passing, since none of them go through `mapped_fit` at
    /// all.
    #[test]
    fn the_mapped_fit_snaps_its_centre() {
        let towards_light = Vec3::Y;
        let base = mapped_fit(Mat4::IDENTITY, towards_light);
        // Comfortably inside this centre's own rounding margin - see the
        // `map::tests::snap_fixture` comment for why a fixed-size nudge has
        // to be checked against the specific base it is nudging, not
        // assumed safe from its size alone.
        let nudged_player = Mat4::from_translation(Vec3::new(0.0, 0.0, 1e-4));
        let nudged = mapped_fit(nudged_player, towards_light);
        assert_eq!(
            nudged.matrix().to_cols_array(),
            base.matrix().to_cols_array(),
            "a sub-texel nudge to the player's own position was not absorbed - \
             is mapped_fit still snapping?"
        );
    }

    /// The near box lies wholly inside the far one, whichever way the craft
    /// heads and whatever the light's tilt: a receiver that leaves the near
    /// layer must land in the far one, or the old straight-edged end of shadow
    /// comes back as a ring of lit road between them.
    #[test]
    fn the_near_box_sits_inside_the_far_one() {
        let lights = [
            Vec3::Y,
            Vec3::new(0.4, 1.0, 0.2).normalize(),
            Vec3::new(-0.7, 0.5, 0.3).normalize(),
        ];
        let headings = [0.0_f32, 0.8, 2.0, 3.5, 5.1];
        for towards_light in lights {
            for yaw in headings {
                let player = Mat4::from_translation(Vec3::new(120.0, -7.0, 45.0))
                    * Mat4::from_rotation_y(yaw);
                let near = mapped_fit(player, towards_light);
                let far = mapped_far_fit(player, towards_light).matrix();
                // The near box's eight corners, through the far box's own
                // projection: each must land inside its unit square and depth.
                let near_inverse = near.matrix().inverse();
                for corner in 0..8 {
                    let ndc = Vec3::new(
                        if corner & 1 == 0 { -1.0 } else { 1.0 },
                        if corner & 2 == 0 { -1.0 } else { 1.0 },
                        if corner & 4 == 0 { 0.0 } else { 1.0 },
                    );
                    let world = near_inverse.project_point3(ndc);
                    let in_far = far.project_point3(world);
                    assert!(
                        in_far.x.abs() <= 1.0 && in_far.y.abs() <= 1.0,
                        "near corner {corner} at {world} falls outside the far box \
                         ({in_far}) for light {towards_light}, yaw {yaw}"
                    );
                    assert!(
                        (0.0..=1.0).contains(&in_far.z),
                        "near corner {corner} is past the far box's depth ({in_far})"
                    );
                }
            }
        }
    }

    /// Both layers push a receiver the same world distance towards the light,
    /// so the far one's bias is smaller in its own deeper units and never zero.
    #[test]
    fn the_far_bias_is_the_near_one_in_the_far_layers_units() {
        let far = far_depth_bias();
        assert!(far > 0.0 && far < MAPPED_DEPTH_BIAS, "far bias {far}");
    }
}
