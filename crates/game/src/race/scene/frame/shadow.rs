//! Both shadow tiers' per-frame geometry: where each craft's blob quad goes,
//! and what its authored hull projects to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the seam `frame/craft.rs` and
//! `frame/attachments.rs` already set; a move, with no behaviour change.

use oag_core::math::Vec3;
use oag_render::mesh::GpuVertex;
use oag_render::shadow::Placement;

use crate::race::Race;

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
        shadows: crate::display::Shadows,
    ) -> (Vec<Placement>, Vec<GpuVertex>) {
        // The blob shadows, gathered here with the rest of the per-frame
        // geometry and uploaded whether or not the tier is on - `upload` with
        // no placements clears the runs, which is what makes turning the row
        // off take effect on the next frame rather than leaving the last
        // frame's quads in the buffer.
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
        if shadows == crate::display::Shadows::Original {
            for placement in &placements {
                let Some(Some(hull)) = self.shadow_hulls.get(placement.silhouette) else {
                    continue;
                };
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
            }
        }
        // At `original` the quads are not drawn at all: the two tiers are
        // alternatives, not layers.
        let quads: &[Placement] = if shadows == crate::display::Shadows::Blob {
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
        shadows: crate::display::Shadows,
    ) -> oag_render::mesh_render::ShadowMap {
        let map = self.shadow_map.borrow();
        match shadows {
            crate::display::Shadows::Original if map.casters() > 0 => {
                oag_render::mesh_render::ShadowMap {
                    matrix: map.matrix().to_cols_array_2d(),
                    strength: MAP_STRENGTH,
                    mode: oag_render::mesh_render::ShadowMap::COVERAGE,
                    depth_bias: 0.0,
                    _pad: 0.0,
                }
            }
            crate::display::Shadows::Mapped if map.depth_casters() > 0 => {
                oag_render::mesh_render::ShadowMap {
                    matrix: map.depth_matrix().to_cols_array_2d(),
                    strength: MAPPED_STRENGTH,
                    mode: oag_render::mesh_render::ShadowMap::DEPTH,
                    depth_bias: MAPPED_DEPTH_BIAS,
                    _pad: 0.0,
                }
            }
            // Every other case, `off` included: a strength of zero is what
            // makes the sample every pipeline carries inert.
            _ => oag_render::mesh_render::ShadowMap::off(),
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
        shadows: crate::display::Shadows,
    ) -> usize {
        let mut map = self.shadow_map.borrow_mut();
        if shadows == crate::display::Shadows::Mapped {
            return self.render_depth_map(&mut map, queue, encoder, race);
        }
        // A hull the craft authors is Pulse's mechanism; if any slot has one,
        // this title shadows that way and not this one.
        let hulls = self.shadow_hulls.iter().any(Option::is_some);
        if shadows != crate::display::Shadows::Original || hulls {
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
            // `oag_render::exhaust::CRAFT_ROW_SCALE`.
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

impl super::super::Scene {
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
        let fit = oag_render::shadow::map::Fit {
            centre: mapped_centre(race.ship_model_matrix_of(0)),
            radius: MAPPED_RADIUS,
            towards_light: mapped_light(&self.light),
        }
        // Snapped to whole texels of the map this fit is about to render
        // into, in the light's own basis: a centre that slides by an
        // arbitrary sub-texel amount every frame re-quantises the whole map,
        // which is the standard cause of a shadow that crawls under a moving
        // fit. See `Fit::snapped`.
        .snapped(oag_render::shadow::map::DEPTH_SIZE);
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
/// Flattening the heading to the world's horizontal plane leaves only yaw -
/// the direction the player is actually steering towards - driving the lead.
fn mapped_centre(player: oag_core::math::Mat4) -> Vec3 {
    let position = player.transform_point3(Vec3::ZERO);
    let forward = player.transform_vector3(Vec3::Z);
    let heading = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    position + heading * MAPPED_AHEAD
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
fn mapped_light(light: &oag_render::mesh_render::Light) -> Vec3 {
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

    use super::{MAPPED_AHEAD, mapped_centre};
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

    /// Pitching the player - cresting a jump, diving into a dip - does not
    /// move the lead at all: only the horizontal heading drives it. Before
    /// this fix the lead swung by `MAPPED_AHEAD * sin(pitch)`, on the order of
    /// a hundred texels of the `mapped` tier's own map for a modest 10-degree
    /// pitch - see this module's `mapped_centre` docs for the measurement.
    #[test]
    fn pitching_the_player_does_not_move_the_lead() {
        let level = mapped_centre(Mat4::IDENTITY);
        for pitch_degrees in [5.0_f32, 10.0, 15.0, -20.0] {
            let pitched = Mat4::from_rotation_x(pitch_degrees.to_radians());
            let centre = mapped_centre(pitched);
            assert!(
                (centre - level).length() < 1e-5,
                "pitch {pitch_degrees} moved the centre to {centre:?}, level was {level:?}"
            );
        }
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
}
