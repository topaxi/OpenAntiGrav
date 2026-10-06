//! Both shadow tiers' per-frame geometry: where each craft's blob quad goes,
//! and what its authored hull projects to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/craft.rs` and
//! `frame/attachments.rs` already set; a move, with no behaviour change.

use oag_core::math::{Mat4, Vec3};
use oag_mesh::mesh::GpuVertex;
use oag_render::shadow::Placement;

use crate::Race;
use crate::scene::motion::Snapshot;

/// How dark a fully covered texel of the shadow map draws the track.
///
/// **Ours, and the same kind of number `HULL_DARKNESS` is.** Wipeout HD's own
/// track material computes `1 - shadow` into the fragment's *alpha* and a later
/// compositing pass consumes it; that pass is unread, so how dark the result
/// lands is not something this project can read off the disc. `0.5` is chosen
/// to sit where the craft reads as shadowed without the road going black under
/// it, and it is the first thing to replace once the compositing pass is read.
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

/// How far ahead of the player the `mapped` tier's own box is centred, and how
/// wide it is.
///
/// **Ours.** One cascade has to choose where its texels go; putting the box
/// around the craft and the road ahead of it spends them where the player is
/// heading. A cascade ladder would not have to choose - see
/// `docs/rendering/shadows.md`.
const MAPPED_AHEAD: f32 = 40.0;
const MAPPED_RADIUS: f32 = 70.0;

/// How far past the grid's own bounds the light's view is fitted.
///
/// **Ours.** The map covers the casters and nothing else - a track-sized
/// orthographic box at `shadow::map::SIZE` is metres per texel - and a small
/// margin keeps a craft that is banking or airborne inside it rather than
/// clipping its own shadow at the border.
const FIT_MARGIN: f32 = 6.0;

impl super::super::Scene {
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
        let placements = if shadows.draws() {
            race.shadow_placements()
        } else {
            Vec::new()
        };
        // The `original` tier draws the craft's own authored hull instead of a
        // textured quad, projected onto the same surface the placement found.
        // A slot whose model authors no hull contributes nothing rather than
        // falling back to a blob: a tier that silently becomes another tier is
        // how a missing feature stops being noticed. See
        // `oag_render::shadow::hull_triangles`, and
        // `oag_pulse::shadow::AUTHORED_AXIS` for the direction.
        let mut hull_vertices = Vec::new();
        if shadows == oag_display::display::Shadows::Original {
            for placement in &placements {
                let Some(Some(hull)) = self.shadow_hulls.get(placement.silhouette) else {
                    continue;
                };
                let first = hull_vertices.len();
                oag_render::shadow::hull_triangles(
                    &oag_render::shadow::Cast {
                        hull,
                        model: race.ship_model_matrix_of(placement.silhouette),
                        axis: Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS),
                        contact: placement.contact,
                        normal: placement.normal,
                        strength: placement.strength,
                    },
                    &mut hull_vertices,
                );
                oag_render::shadow::conform_to_floor(&mut hull_vertices, first, |at, normal| {
                    race.floor_above(at, normal)
                });
            }
        }
        // At `original` the quads are not drawn at all: the two tiers are
        // alternatives, not layers.
        let quads: &[Placement] = if shadows == oag_display::display::Shadows::Blob {
            &placements
        } else {
            &[]
        };
        (quads.to_vec(), hull_vertices)
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
        let map = self.shadow_map.borrow();
        match shadows {
            oag_display::display::Shadows::Original if map.casters() > 0 => {
                oag_mesh::mesh_render::ShadowMap {
                    matrix: map.matrix().to_cols_array_2d(),
                    strength: MAP_STRENGTH,
                    mode: oag_mesh::mesh_render::ShadowMap::COVERAGE,
                    depth_bias: 0.0,
                    _pad: 0.0,
                }
            }
            oag_display::display::Shadows::Mapped if map.depth_casters() > 0 => {
                oag_mesh::mesh_render::ShadowMap {
                    matrix: map.depth_matrix().to_cols_array_2d(),
                    strength: MAPPED_STRENGTH,
                    mode: oag_mesh::mesh_render::ShadowMap::DEPTH,
                    depth_bias: MAPPED_DEPTH_BIAS,
                    _pad: 0.0,
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
        queue: &wgpu::Queue,
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
            map.render(queue, encoder, &default_fit(), &[]);
            map.render_depth(queue, encoder, &default_fit(), &[]);
            return 0;
        }
        let drawn = usize::from(race.ship_count());
        let mut casters = Vec::with_capacity(drawn);
        let mut centre = Vec3::ZERO;
        let mut count = 0.0;
        for slot in 0..drawn.min(self.ships.len()) {
            if !race.ship_active(slot) {
                continue;
            }
            let model = race.ship_model_matrix_of(slot);
            casters.push(self.ships[slot].caster(model));
            centre += model.transform_point3(Vec3::ZERO);
            count += 1.0;
        }
        if casters.is_empty() {
            map.render(queue, encoder, &default_fit(), &[]);
            return 0;
        }
        centre /= count;
        // Fitted to the grid's own spread **plus each craft's own size**, and
        // not to the track - see `shadow::map::Fit`. Leaving the hull's radius
        // out is not a small error: with one craft it makes the box smaller
        // than the caster, and the shadow comes out the size of the overlap.
        let mut radius = FIT_MARGIN;
        for slot in 0..drawn.min(self.ships.len()) {
            if !race.ship_active(slot) {
                continue;
            }
            let model = race.ship_model_matrix_of(slot);
            let at = model.transform_point3(Vec3::ZERO);
            // The matrix carries the craft's own render scale, so the world
            // radius is the model's through it - see
            // `oag_fx::exhaust::CRAFT_ROW_SCALE`.
            let scale = model.x_axis.truncate().length();
            let hull = self.ships[slot].radius() * scale;
            radius = radius.max((at - centre).length() + hull + FIT_MARGIN);
        }
        let fit = oag_render::shadow::map::Fit {
            centre,
            radius,
            // The circuit's own sun, which HD authors in its `.envsettings`
            // and this renderer already lights through. Not a direction of
            // this module's own.
            towards_light: Vec3::from_array(self.light.direction),
        };
        map.render(queue, encoder, &fit, &casters);
        casters.len()
    }
}

/// The environment variable that, naming a file, writes the player's
/// sun-occlusion map out beside a `--screenshot` capture - see
/// `Scene::dump_sun_occlusion_if_asked`.
const DUMP_VAR: &str = "OAG_DUMP_SUN_OCCLUSION";

/// The same for the player's self-shadow depth map - see
/// `Scene::dump_sun_occlusion_if_asked`, which writes both.
const DUMP_SELF_SHADOW_VAR: &str = "OAG_DUMP_SELF_SHADOW";

/// Half the side of the cube a craft's sun-occlusion map is fitted to, in
/// world units: the original's own fallback bbox for a ship model without one.
const SUN_OCCLUSION_HALF_EXTENT: f32 = 6.0;

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
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
        shadows: oag_display::display::Shadows,
    ) {
        let mut maps = self.sun_occlusion.borrow_mut();
        let hulls = self.shadow_hulls.iter().any(Option::is_some);
        let active = shadows == oag_display::display::Shadows::Original
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
            // Fitted to the craft alone. The original fits the box to the
            // ship's own bbox, or to a fixed `(-6, -6, -6)..(6, 6, 6)` cube
            // when the model carries none (`Shadow_BuildShipShadowMatrices`);
            // this side takes that cube plus the coverage map's own margin,
            // so the hull's silhouette never clips its own map's edge - the
            // edge reads as no sun. **Not the drawable's bounding radius**:
            // an HD `Ship.vex` measures over 200 units across by that, its
            // authored geometry reaching far past the hull, and a box that
            // size is a map of the neighbourhood rather than of the craft.
            let fit = oag_render::shadow::map::Fit {
                centre: model.transform_point3(Vec3::ZERO),
                radius: SUN_OCCLUSION_HALF_EXTENT + FIT_MARGIN,
                towards_light: Vec3::from_array(self.light.direction),
            };
            maps.render(queue, encoder, slot, &fit, &track);
            // The craft's own depth map from the same box: its opaque ranges
            // and nothing else, so a wing shadows the fuselage and no other
            // craft shadows either. See `oag_render::shadow::self_shadow`.
            maps.render_self_shadow(queue, encoder, slot, &self.ships[slot].caster(model));
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
        queue: &wgpu::Queue,
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
                drawable.write_hull(
                    queue,
                    view_projection,
                    race.ship_model_matrix_of(slot),
                    prev_vp * prev.ship(slot, race),
                    self.sun_occlusion_layer(slot),
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
    pub fn dump_sun_occlusion_if_asked(
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
        if let Ok(dump) = std::env::var(DUMP_SELF_SHADOW_VAR) {
            let maps = self.sun_occlusion.borrow();
            let depths = maps.self_shadow().read_back(device, queue, 0);
            let texels: Vec<u8> = depths.iter().map(|d| (d * 255.0) as u8).collect();
            let size = oag_render::shadow::self_shadow::SIZE;
            write(&dump, &texels, size, "self shadow")?;
        }
        Ok(())
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

    /// The `mapped` tier's own pass: **everything** casts, into a depth map.
    ///
    /// The track goes in first and the craft after it, which is the whole
    /// difference from the tier above - a map that only the craft cast into
    /// cannot shadow a road with a bridge over it, and one only the track cast
    /// into cannot shadow a craft at all.
    fn render_depth_map(
        &self,
        map: &mut oag_render::shadow::map::Map,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        race: &Race,
    ) -> usize {
        let mut casters = vec![self.track.caster(oag_core::math::Mat4::IDENTITY)];
        let drawn = usize::from(race.ship_count());
        for slot in 0..drawn.min(self.ships.len()) {
            if !race.ship_active(slot) {
                continue;
            }
            casters.push(self.ships[slot].caster(race.ship_model_matrix_of(slot)));
        }
        let fit = mapped_fit(race.ship_model_matrix_of(0), mapped_light(&self.light));
        // The coverage map stays cleared while this tier is on: two maps are
        // bound at once and only one of them may have anything in it, or a
        // switch between tiers would show the other's leftovers.
        map.render(queue, encoder, &default_fit(), &[]);
        map.render_depth(queue, encoder, &fit, &casters);
        casters.len()
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
/// not, which on Pulse is `oag_pulse::shadow::AUTHORED_AXIS` - so switching
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
    -Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS)
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

    use super::{MAPPED_AHEAD, mapped_centre, mapped_fit};
    use oag_core::math::Vec3;

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
}
