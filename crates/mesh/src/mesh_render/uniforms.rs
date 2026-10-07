//! The uniform blocks the mesh pipeline reads: the camera and model matrices in
//! bind group 0, and the scene's fog and light in bind group 2.
//!
//! Split out of `mesh_render.rs` for size alone. What is here is the whole of
//! how an [`Orbit`] becomes a view-projection matrix, which is worth having in
//! one place: the layout is mirrored by four `.wgsl` declarations and by the
//! asset viewer's own buffer sizing, so it moves in lockstep with them.

use oag_core::math::{Mat4, Vec3, camera};

use crate::mesh::Model;
use crate::orbit::Orbit;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    /// Which layer of the per-craft sun-occlusion array this model samples,
    /// plus one - `0.0` for none. See `shaders/shadow.wesl`'s `sun_occlusion` and
    /// [`crate::shadow::occlusion`]. Was a global texture-animation phase
    /// before [`TexAnims`] replaced it and padding after; reused so the
    /// layout `mesh.wesl`, four other pipelines and the asset viewer mirror
    /// stays the same size.
    sun_occlusion_layer: f32,
    /// The model's own animation clock in seconds - what an HD material's
    /// `UV_offset` is bound to (`node + 0xc0`, see
    /// [`crate::mesh::slots::CLOCK_SCROLL_RING`]). `0.0` for every draw that
    /// is not a clock-scrolled material. Padding until 2026-10-05.
    model_clock: f32,
    /// A second per-draw scalar, for the one HD material that reads a value
    /// the blast owns rather than the model's clock: `ColourAnim` on the
    /// Bomb's fireball ([`crate::mesh::slots::BOMB_FIRE`]). `0.0` for every
    /// other draw. Padding until 2026-10-07.
    model_colour: f32,
    _pad2: f32,
    /// The previous simulation tick's `view_projection * model`,
    /// premultiplied: what the velocity target measures screen motion
    /// against. See `mesh.wesl`'s own mirror for why this is one matrix
    /// rather than a second `view_projection`/`model` pair - fog and
    /// lighting need world position, velocity needs only clip position, and
    /// a split pair here would add 64 bytes to every draw for nothing.
    prev_mvp: [[f32; 4]; 4],
}

/// Builds the camera and model matrices for a given orbit angle.
///
/// The model is framed from its own bounding sphere, so any model fills the
/// view regardless of the scale baked into the file. That also means a wrong
/// scale looks *right* here, so this is not a check on the scale factor; the
/// bounding-box assertion in `oag-formats` is.
///
/// `orbit.zoom` scales that distance; 1.0 is the default framing described
/// above, and `orbit.pan` moves the look-at point off the model's own centre in
/// the same bounding-sphere radii. The eye follows the look-at point, so
/// panning slides across the model rather than aiming past it.
fn matrices(model: &Model, aspect: f32, orbit: Orbit) -> Uniforms {
    let centre = Vec3::from_array(model.centre);
    let view_projection = view_projection(model, aspect, orbit);
    let model_matrix = Mat4::from_translation(-centre);

    Uniforms {
        view_projection: view_projection.to_cols_array_2d(),
        model: model_matrix.to_cols_array_2d(),
        sun_occlusion_layer: 0.0,
        model_clock: 0.0,
        model_colour: 0.0,
        _pad2: 0.0,
        // The viewer has no previous tick; previous equals current, which is
        // zero velocity. Its pipelines are built `Velocity::None` and never
        // read this, but a zeroed matrix would still be the wrong value to
        // leave lying in a mirrored layout.
        prev_mvp: (view_projection * model_matrix).to_cols_array_2d(),
    }
}

/// Where the camera is and what it can see, for a given orbit.
///
/// Separate from [`matrices`] so it can be checked without a GPU: what it
/// returns is a matrix, and a matrix is testable by projecting a point through
/// it. See this module's tests.
#[must_use]
pub fn view_projection(model: &Model, aspect: f32, orbit: Orbit) -> Mat4 {
    let distance = model.radius * 3.0 * orbit.zoom;
    let (yaw, pitch) = (orbit.yaw, orbit.pitch);
    // In model-recentred space, which is what the model matrix puts the
    // geometry into - so the origin here is the model's own centre, and
    // `orbit.pan` is an offset from it in bounding-sphere radii.
    let target = orbit.pan * model.radius;
    let eye = target
        + Vec3::new(
            distance * yaw.cos() * pitch.cos(),
            distance * pitch.sin(),
            distance * yaw.sin() * pitch.cos(),
        );

    // The far plane covers the pan too: an eye dragged toward one side of the
    // model is that much further from the other.
    let far = distance * 10.0 + target.length();
    camera::perspective(45f32.to_radians(), aspect, 0.01, far)
        * camera::look_at(eye, target, Vec3::Y)
}

/// The depth format `build`'s pipeline is fixed to; a caller's own depth
/// texture must match it.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Recomputes the camera and model matrices and uploads them to `buffer`.
///
/// Returns the view-projection matrix it just wrote, so a caller can build a
/// [`oag_core::math::frustum::Frustum`] from the same camera without
/// recomputing it.
pub fn write_uniforms(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    model: &Model,
    aspect: f32,
    orbit: Orbit,
) -> Mat4 {
    let uniforms = matrices(model, aspect, orbit);
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniforms));
    Mat4::from_cols_array_2d(&uniforms.view_projection)
}

/// [`write_uniforms`], for a caller that already has both matrices rather than
/// an [`Orbit`] to frame one from.
///
/// [`matrices`] always derives its camera from a model's own bounding sphere,
/// which is right for the asset viewer and wrong for a HUD overlay: a
/// `<Mode3D><Model>` widget is placed in the layout's own pixel space, not
/// framed to fill the screen. This is the same uniform layout with that
/// assumption removed - a raw view-projection and model matrix, written
/// verbatim.
///
/// `prev_mvp` is set equal to the current frame's, the same choice
/// [`matrices`] makes for the viewer: there is no previous tick to compare
/// against and every pipeline built with [`crate::mesh_render::Velocity::None`]
/// never reads it anyway.
pub fn write_uniforms_raw(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    view_projection: Mat4,
    model: Mat4,
) {
    let mvp = view_projection * model;
    let uniforms = Uniforms {
        view_projection: view_projection.to_cols_array_2d(),
        model: model.to_cols_array_2d(),
        sun_occlusion_layer: 0.0,
        model_clock: 0.0,
        model_colour: 0.0,
        _pad2: 0.0,
        prev_mvp: mvp.to_cols_array_2d(),
    };
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniforms));
}

/// Size, in bytes, of the uniform buffer `write_uniforms` expects.
pub const UNIFORMS_SIZE: u64 = std::mem::size_of::<Uniforms>() as u64;

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Vec4;

    /// A model the size of a track, centred somewhere that is not the origin -
    /// which is the ordinary case, and the one a recentring bug hides in.
    fn model() -> Model {
        Model {
            centre: [100.0, -20.0, 300.0],
            radius: 50.0,
            ..Model::none("camera fixture")
        }
    }

    /// Where a point in model-recentred space lands on screen, in normalised
    /// device coordinates, or `None` if it is behind the eye.
    fn on_screen(orbit: Orbit, point: Vec3) -> Option<(f32, f32)> {
        let clip =
            view_projection(&model(), 1.0, orbit) * Vec4::new(point.x, point.y, point.z, 1.0);
        (clip.w > 0.0).then(|| (clip.x / clip.w, clip.y / clip.w))
    }

    /// With no pan the model's centre is dead centre, at every angle.
    #[test]
    fn an_unpanned_camera_looks_at_the_models_own_centre() {
        for step in -4..=4 {
            let orbit = Orbit {
                yaw: step as f32 * 0.7,
                pitch: step as f32 * 0.3,
                ..Orbit::default()
            };
            let (x, y) = on_screen(orbit, Vec3::ZERO).expect("in front of the eye");
            assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "at {orbit:?}: ({x}, {y})");
        }
    }

    /// Panning right moves the model left on screen, and up moves it down.
    ///
    /// The sign convention the viewer's drag handling depends on: the point
    /// under the cursor comes with the cursor, so the *camera* goes the other
    /// way. Getting it backwards is the classic orbit-viewer bug and it is
    /// invisible in a still.
    #[test]
    fn panning_moves_the_model_the_opposite_way_on_screen() {
        let right = on_screen(Orbit::default().panned(0.5, 0.0), Vec3::ZERO).expect("visible");
        assert!(
            right.0 < -1e-3,
            "panning right should push it left, got {right:?}"
        );
        assert!(right.1.abs() < 1e-4, "and not move it vertically");

        let up = on_screen(Orbit::default().panned(0.0, 0.5), Vec3::ZERO).expect("visible");
        assert!(up.1 < -1e-3, "panning up should push it down, got {up:?}");
        assert!(up.0.abs() < 1e-4, "and not move it sideways");
    }

    /// The panned-to point is what ends up dead centre.
    ///
    /// The check that ties the camera maths to `Orbit::pan`'s own units: pan by
    /// `p` radii and the point `p * radius` away from the model's centre is the
    /// one in the middle of the frame, from any angle.
    #[test]
    fn the_point_panned_to_is_the_one_in_the_middle_of_the_frame() {
        for step in -3..=3 {
            let orbit = Orbit {
                yaw: step as f32 * 0.9,
                pitch: step as f32 * 0.35,
                ..Orbit::default()
            }
            .panned(0.7, -0.4);

            let target = orbit.pan * model().radius;
            let (x, y) = on_screen(orbit, target).expect("in front of the eye");
            assert!(x.abs() < 1e-3 && y.abs() < 1e-3, "at {orbit:?}: ({x}, {y})");
        }
    }

    /// Zoom does not move what is being looked at, only how much of it fits.
    #[test]
    fn zoom_leaves_the_look_at_point_where_it_is() {
        let panned = Orbit::default().panned(0.4, 0.2);
        let target = panned.pan * model().radius;
        for zoom in [0.3, 1.0, 4.0] {
            let (x, y) = on_screen(Orbit { zoom, ..panned }, target).expect("visible");
            assert!(x.abs() < 1e-3 && y.abs() < 1e-3, "zoom {zoom}: ({x}, {y})");
        }
    }
}

/// The fog block `mesh.wesl` reads from bind group 2.
///
/// Deliberately **not** part of [`Uniforms`]. That struct is mirrored by every
/// pipeline in this crate and by the asset viewer, so growing it means moving
/// four `.wgsl` declarations and `oag-view`'s own buffer sizing in lockstep;
/// only the pipelines that fog need these fields.
///
/// The field order is the WGSL declaration's, and the padding is real: a WGSL
/// `vec3` aligns to 16 bytes, so the `f32` after each one occupies the slot that
/// alignment would otherwise waste.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Fog {
    /// Fog colour, linear.
    pub colour: [f32; 3],
    /// Distance at which fog starts.
    pub near: f32,
    /// Eye position, so the fragment stage can measure distance.
    pub camera: [f32; 3],
    /// Distance at which fog is total.
    pub far: f32,
    /// `1.0` to fog, `0.0` to pass colour through untouched.
    pub enabled: f32,
    /// Wipeout HD's fog coefficient, for [`Fog::curve`] `1.0`; unused at `0.0`.
    pub density: f32,
    /// Which curve `mesh.wesl` applies. `0.0` is the GE's linear ramp between
    /// [`Fog::near`] and [`Fog::far`], re-sampled from a `fogCube` volume each
    /// frame - Pulse's fog. `1.0` is Wipeout HD's, **read out of its own
    /// fragment microcode** rather than guessed: every fogged variant of a
    /// circuit `.rcsmaterial` computes `exp(-(density * view_depth)^2)` - a
    /// `MUL` by `log2(e)` into `EX2` with the product squared and negated - and
    /// lerps the fog colour in by that factor. The coefficient and the colour
    /// arrive together in one patched `float4` the shader interface itself
    /// names `fogColour` (a crc32 preimage, not a resemblance). See
    /// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
    pub curve: f32,
    /// The player's TEXTURE DETAIL as levels added to the slope law's, off a
    /// PSP `.vex` model's view depth - see [`super::TextureDetail::level_shift`]. `0.0`
    /// is the recovered law, which is also what every buffer nothing writes
    /// holds. Here rather than a pipeline constant so the setting applies live.
    pub texlod_shift: f32,
}

impl Fog {
    /// Fog that does nothing.
    ///
    /// What the sky, the asset viewer and a track with no `fogCube` bind. It is
    /// a value rather than an unbound group because WGSL has no optional
    /// bindings: the alternative is a second pipeline per fog state.
    #[must_use]
    pub fn off() -> Self {
        Self {
            colour: [0.0; 3],
            near: 0.0,
            camera: [0.0; 3],
            far: 1.0,
            enabled: 0.0,
            density: 0.0,
            curve: 0.0,
            texlod_shift: 0.0,
        }
    }

    /// This fog with the player's TEXTURE DETAIL in its level shift.
    #[must_use]
    pub fn with_texture_detail(self, detail: super::TextureDetail) -> Self {
        Self {
            texlod_shift: detail.level_shift(),
            ..self
        }
    }

    /// Fog from one sampled [`oag_vex::fog::FogParams`] and the eye it was
    /// sampled at.
    #[must_use]
    pub fn new(params: &oag_vex::fog::FogParams, camera: [f32; 3]) -> Self {
        Self {
            colour: params.colour,
            near: params.near,
            camera,
            // A degenerate range would divide by zero in the shader; the shader
            // clamps the span, and this keeps the ordering sane regardless.
            far: params.far.max(params.near + f32::EPSILON),
            enabled: 1.0,
            density: 0.0,
            curve: 0.0,
            texlod_shift: 0.0,
        }
    }

    /// Wipeout HD's fog, from the two values its `track.envsettings` authors.
    ///
    /// The curve is the disc's - see [`Fog::curve`]. What is **not** read is
    /// how the engine fills the shader's `fogColour.w` from
    /// `Fog.Fog Density`: this passes the authored density through unscaled,
    /// which is the plain reading and is judged against an rpcs3 reference
    /// frame rather than proven from the executable. The colour is authored
    /// for HD's linear-light pipeline and used as-is in this gamma target,
    /// the same hold `Light::authored` documents.
    #[must_use]
    pub fn authored_exp2(colour: [f32; 3], density: f32) -> Self {
        Self {
            colour,
            near: 0.0,
            camera: [0.0; 3],
            far: 1.0,
            enabled: 1.0,
            density,
            curve: 1.0,
            texlod_shift: 0.0,
        }
    }
}

mod light;

pub use light::Light;

/// The Zone effect's per-stage shader parameters, as much of them as the disc
/// itself feeds.
///
/// # What this is
///
/// Wipeout HD/Fury's Zone mode is **not an engine program**: it is a variant
/// compiled into 1,467 of the disc's own `.rcsmaterial` files, and its rule was
/// read out of that fragment microcode - see
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`. The part reproduced
/// here is:
///
/// ```text
/// zoneUV  = zoneColourTint.xy * (1 - meshUV)
/// rim     = 1 - dot(N, toEye)
/// inside  = distance(worldPos, zoneOrigin) < zoneColourTint.w
/// surface = zoneTex(zoneUV).rgb * zoneEffect<I|O>.rgb
///         + zoneBase<I|O>.rgb    * rim^10
///         + zoneBaseAlt<I|O>.rgb * rim^5
/// ```
///
/// where `<I|O>` is the Inner parameter inside the sphere and the Outer one
/// outside it - see "the stage-transition wavefront" below.
///
/// **That is the whole surface colour: the material's own albedo does not
/// enter it.** Which is the finding that carries the look - the original's
/// Zone frame is near-monochrome because the circuit's diffuse is *gone*, not
/// because a tint was added over it. The `10` and `5` are inline literals in
/// the microcode, not parameters, so only the two colours are per-stage.
///
/// # Which of the disc's two Zone shapes this is
///
/// A census of every `.rcsmaterial` on the disc - 20,214 fragment blocks that
/// name a Zone parameter or sampler - finds exactly two shapes, with no block
/// in both. Asserted against the image itself by
/// `crates/formats/tests/zone_shader_census_ground_truth.rs`:
///
/// | shape | blocks | `rim^10`/`rim^5` | `blackMask` |
/// | --- | ---: | ---: | ---: |
/// | `zoneBase` (this one) | 20,084 | all | **none** |
/// | `zoneAniso` | 130 | none | all |
///
/// And the split is by *environment*: all twelve racing circuits are 100%
/// `zoneBase` and contain not one `zoneAniso` block, while the `zoneAniso`
/// shape lives only in `zone_1`..`zone_4`, HD's dedicated Zone arenas.
/// `zone-shader.md`'s `albedo + zoneCol * (1 - blackMask)` rule is the arena
/// shape, correct where it was read and generalised from three materials, two
/// of them arena ones. Confidence 86 on the split, 82 on the missing albedo.
///
/// # Where each field comes from, and what is left out
///
/// Every value here has a located source on the disc:
///
/// - [`Self::uv_scale`] is the two title-wide `.effectSettings` keys
///   `Texture U scale` / `Texture V scale`, which
///   `Environment_RegisterStageSchema` (`0x003d0b98`) registers straight into
///   shader parameter 52's own storage. Confidence 85.
/// - [`Self::effect`] is the showing stage's `Track.Texture Colour`, which
///   `Environment_UpdateStageBlend` (`0x003da540`) copies into
///   `zoneEffectInner`/`zoneEffectOuter`. Confidence 84.
/// - The texture is the stage's own `zoneModeTrack<n>.gtf`.
///
/// # Two parameter sets, selected per chunk by the file
///
/// **HD publishes these parameters twice, and the two publications are
/// paired end to end**: one block binds the `zoneMode*` textures beside the
/// `Scene.*` colours, the other binds `zoneModeTrack*` beside the `Track.*`
/// ones. So "which texture set" and "which colour group" are one choice, not
/// two. Both pairs travel here - [`Self::track`] and [`Self::scene`] - and
/// `mesh.wesl` picks between them per fragment on `slots::ZONE_TRACK`, which
/// `mesh::rcs` sets from the chunk's own render-block flags:
/// `oag_rcs::rcsmodel::Mesh::is_track`, bit 0 of the halfword at `+0x06` of
/// the record the chunk header's `+0x08` names. That is what `FUN_003ff860`
/// branches on - it publishes Scene when the bit is clear and Track when it
/// is set - and the bit is authored in the `.rcsmodel`: 4,365 of the disc's
/// 41,861 chunks carry it, in exactly the 37 track-shaped models, 124 of
/// Talon's Junction's 983. Confidence 85 on the record, 80 on the meaning;
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`,
/// thirtieth pass.
///
/// **Until 2026-09-15 this build bound the `Track` half to everything**,
/// because it had no scenery/track distinction to branch on; on Talon's
/// Junction that coloured 859 chunks with the set the original reserves for
/// 124. The `Scene` texture set is fifteen flat whites, so a scene chunk's
/// surface is `Scene.Texture Colour` flat plus the two rim terms - authored
/// black on `Start`, which is a dark environment around a lit road, exactly
/// what the original's opening frame shows.
///
/// # Approximations that remain, stated
///
/// The original applies the variant per material at all:
/// 1,467 of the disc's 1,590 `.rcsmaterial` files carry one, so 123 do not.
/// **Craft are the case that mattered and they are now excluded from the
/// data rather than by taste**: the twelve `data/materials/ships/*` materials
/// in `DATA03.PSARC` carry *zero* Zone blocks, as do the 39 under
/// `data/weapons/materials/`, so the original leaves hulls and rockets
/// ordinarily shaded through a Zone race. `race::scene::frame` writes
/// [`Zone::default`] to the ships' own scene buffer for exactly that reason -
/// a build that dropped the albedo everywhere would blank them.
///
/// The sky cube, the pads and the collision wireframe do still reach this
/// path, and there the recolour is a blanket one - same shape as the shared
/// specular exponent and the blanket sun term `mesh.wesl` already carries.
///
/// **The sum happens in each shading path's own colour space.** `zoneTex` is a
/// texture and takes the same `pow(x, 2.2)` decode every other sample here
/// takes; `zoneEffect` is a shader parameter and takes none - it is authored
/// past `1.0` (`Track.Texture Colour` reaches `9.0`), which is a
/// multiplier's range. Getting
/// that wrong is invisible whenever `zoneEffect` is exactly `1.0`, which is
/// why `crates/render/tests/zone_recolour.rs` binds `2.0`.
///
/// # The stage-transition wavefront
///
/// **A Zone stage change is not a colour cross-fade; it is a sphere.** Every
/// Zone parameter is published twice more, as an Inner and an Outer copy,
/// and the fragment picks one on `distance(worldPos, zoneOrigin) <
/// zoneColourTint.w` (`zone-shader.md`, 82, read with opposite compiler
/// polarity in two materials). `Environment_UpdateStageBlend` (`0x003da540`)
/// copies stage `n` into the Inner set and stage `n - 1` into the Outer,
/// restarts the radius at `0.1` when a stage commits and then grows it every
/// frame - `radius += speed; speed += 0.1`, from a speed of `0.5`, capped at
/// `20000` - while `Scene_PrepareFrame` (`0x003ad8dc`) rewrites `zoneOrigin`
/// from the local craft's own transform every frame. So the new stage's
/// colours spread out of the player's craft and overtake the old ones at a
/// quadratic pace: `207` units out after a second, `4,635` after five, the
/// cap ten and a half seconds in. Read statically, then reproduced live on
/// RPCS3 on 2026-09-15 to the tenth at two frame counts (94 on the law, 92
/// on the origin being the craft, 210 of 210 frames); the whole account is
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`,
/// passes twenty-four, thirty and thirty-one.
///
/// Here: [`Self::track`] and [`Self::scene`] are the Inner pair,
/// [`Self::track_outer`] and [`Self::scene_outer`] the Outer, [`Self::origin`]
/// is `zoneOrigin` and [`Self::radius`] is `zoneColourTint.w`. The game side
/// (`oag_raceplay::zone_grade::ZoneGrade::follow`) derives the radius from
/// the race's own zone clock by the closed form of that advance, and hands
/// the craft's position over unscaled. Before any transition the Outer pair
/// equals the Inner and the test is a no-op, which is also every draw
/// outside a Zone race ([`Zone::default`], all zero on both sides).
///
/// **One thing about it is not measured, and is stated rather than tuned:**
/// the radius's *unit*. The shader compares it to a world-space distance, and
/// whether this renderer's world units are the original's is unverified -
/// the value is passed through as read, and the live frame at radius `799`
/// (`burst-1.png` in the RPCS3 artefacts) showing the boundary a few hundred
/// units ahead of the craft is a picture, not a measurement.
///
/// **The stage texture follows the sphere too, as of 2026-09-15.** The
/// original publishes `zoneTexInner`/`zoneTexOuter` for both the Track and
/// Scene sets, and so does this renderer now -
/// [`super::zone::StageArt::track_outer`]/[`super::zone::StageArt::scene_outer`],
/// bound alongside [`super::zone::StageArt::track`]/[`super::zone::StageArt::scene`]
/// at `mesh.wesl`'s bindings 11/12 and selected per fragment on the same
/// `zone_inside` test as [`Self::track_outer`]/[`Self::scene_outer`]'s own
/// colours. The stage-change edge that swaps the showing stage also rebuilds
/// just those four texture views (`super::zone::rebind`, called from
/// `oag_raceplay::Scene::rebind_zone_art`), rather than rebuilding the
/// whole drawable - so the boundary is a step in the texture now, not only
/// in the two colour sets over one texture.
///
/// **Two terms of the recovered rule are deliberately absent**, because
/// nothing on the disc feeds them and this project does not invent:
///
/// 1. The whole `zoneAniso` shape - `2 * zoneAnisoPalette[pow(rim,
///    zoneAnisoPower)]` and the `blackMask` gate that comes with it. Its
///    palette textures (`0x00c81330`-`0x00c81350`) have no located filling
///    write, and it applies to the four Zone arenas rather than to a circuit,
///    so it is absent by scope as much as for want of a source.
/// 2. The visualiser glow's own table, `zoneTexVis[band]`: the 256-entry
///    lookup is rewritten every frame from a value whose source is unread,
///    so `shaders/zone.wesl`'s `zone_glow` samples a lookup this project fills from
///    its own mixer instead. Its additive sibling, `5.0 * saturate(1 - 0.1 *
///    (distance - zoneColourTint.w))`, now has both its inputs and is still
///    left out: as read it adds `5.0` to *every* fragment inside the sphere,
///    not just at its edge, and the live frame at radius `799` shows no such
///    flood - so the reading of that term is what is in doubt, and it stays
///    out until it is re-read. See `shaders/zone.wesl`'s `zone_glow`.
// **`align(16)` is load-bearing, not decoration.** WGSL gives this struct an
// alignment of 16 because it holds a `vec4<f32>`, so `Scene`'s `zone` field
// starts at a 16-aligned offset there. Rust's own alignment for it is 4, and
// the two layouts currently agree only because `Fog + Light + time` happens to
// end on a multiple of 16 - a field added above would silently desynchronise
// them, and `min_binding_size: None` means wgpu would not catch it. Stating
// the alignment makes Rust place it the way WGSL does; the assertion below
// checks the result.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Zone {
    /// `zoneColourTint.xy` - the zone texture's own coordinate scale. `1.0` on
    /// both lanes in `zonemode.effectsettings`, read rather than assumed.
    pub uv_scale: [f32; 2],
    /// `1.0` to draw the recolour, `0.0` to leave the albedo alone.
    ///
    /// Not a look switch: it is `0.0` for every draw that is not a Zone race on
    /// a title whose table, UV scale and stage texture all resolved, so a
    /// missing input draws nothing rather than something approximate.
    ///
    /// The `.z` lane of `zoneColourTint`, which the original never writes;
    /// this project's own gate lives in it.
    pub enabled: f32,
    /// `zoneColourTint.w` - the transition sphere's radius this frame, in
    /// world units as the game side hands them over. Zero outside a Zone
    /// race, when the Outer pair equals the Inner anyway.
    pub radius: f32,
    /// `zoneOrigin` - the sphere's centre, the local craft's world position,
    /// `.w` unused.
    pub origin: [f32; 4],
    /// The `Track.*` colours of the showing stage - `zone*Inner`, published
    /// beside `zoneModeTrack<n>.gtf` for a chunk whose render-block flags
    /// carry the track bit.
    pub track: ZoneSet,
    /// The `Scene.*` colours of the showing stage - `zone*Inner`, published
    /// beside `zoneMode<n>.gtf` for every other chunk.
    pub scene: ZoneSet,
    /// The `Track.*` colours of the stage being swept out - `zone*Outer`,
    /// read by a track chunk's fragment outside the sphere.
    pub track_outer: ZoneSet,
    /// The `Scene.*` colours of the stage being swept out - `zone*Outer`,
    /// read by every other chunk's fragment outside the sphere.
    pub scene_outer: ZoneSet,
}

/// One of the two colour groups a Zone stage authors, in the order the
/// shader's own parameters take them. See [`Zone`] for which chunk reads
/// which.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ZoneSet {
    /// `zoneEffect<Inner|Outer>` - the showing stage's `Texture Colour` in
    /// `.rgb`, `EQ brightness` in `.w`: how hard the visualiser glow drives.
    pub effect: [f32; 4],
    /// `zoneBase<Inner|Outer>` in `.rgb` - the showing stage's `Base Colour
    /// Highlight`, the `rim^10` summand's colour. `.w` unused; the exponent
    /// is an inline literal in the microcode, not a parameter, so there is
    /// nothing per-stage to carry for it.
    pub base: [f32; 4],
    /// `zoneBaseAlt<Inner|Outer>` in `.rgb` - the showing stage's `Base
    /// Colour`, the `rim^5` summand's colour. `.w` unused.
    pub base_alt: [f32; 4],
}

/// The shadow map's projection and how hard it darkens what it covers.
///
/// **Wipeout HD's own mechanism, and it is not a depth-compare shadow map.**
/// Its track material samples `shadowMapTex` *projectively* and uses the
/// sample directly - `TXP R1.x, f[TC0] unit2` then `ADD H0.w, -R1.xxxx, {1}`,
/// with no compare against any interpolated reference anywhere in the block.
/// So the texture holds shadow **coverage**, not depth, and the shader's job
/// is `1 - coverage`. Read from `talons_junction/track_surface.rcsmaterial`
/// block #9 with `scripts/ps3-microcode.py`; see
/// [`renderer.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md),
/// "the lit track material".
///
/// **Where it lands is ours.** The original puts `1 - shadow` in the fragment's
/// *alpha* - which is what the `ShadowToAlpha` material flag names - and a
/// later compositing pass consumes it; that pass is unread. This renderer's
/// alpha is the bloom's glow mask ([`super::GlowMask`]), so writing there would
/// bloom the shadow. The term multiplies colour instead, and
/// [`Self::strength`] is this project's number, not the original's.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowMap {
    /// World to shadow-map clip space, as the projective sample's coordinate.
    pub matrix: [[f32; 4]; 4],
    /// How hard a fully covered texel darkens the surface, `0.0` for off.
    pub strength: f32,
    /// Which map is bound and who may read it.
    ///
    /// [`ShadowMap::COVERAGE`] is Wipeout HD's own mechanism - a coverage map,
    /// read by the track surface alone. [`ShadowMap::DEPTH`] is the `mapped`
    /// tier, which is this project's: a depth map every lit surface compares
    /// against, so a craft can shadow another craft and a bridge can shadow
    /// the road under it.
    ///
    /// A uniform rather than a pipeline constant because the setting applies
    /// live: switching tiers must not rebuild a pipeline per model.
    pub mode: f32,
    /// How far, in the map's own depth units, a receiver is pushed towards the
    /// light before comparing.
    ///
    /// **Ours, and unavoidable**: a depth comparison against a quantised map
    /// makes a lit surface shadow itself, and the fix is either a bias or a
    /// per-texel derivative. Ignored in [`ShadowMap::COVERAGE`], which
    /// compares nothing.
    pub depth_bias: f32,
    /// Padding to the 16-byte boundary a uniform's own size needs.
    pub _pad: f32,
}

impl ShadowMap {
    /// [`Self::mode`] for Wipeout HD's coverage map, read by the track alone.
    pub const COVERAGE: f32 = 1.0;

    /// [`Self::mode`] for the `mapped` tier's depth map, read by everything.
    pub const DEPTH: f32 = 2.0;

    /// No shadow at all: the identity matrix and zero strength, which is what
    /// every title but the one being shadowed binds.
    ///
    /// **Zero strength is what makes the sample inert**, not the placeholder
    /// texture - a caller that binds a real map without asking for it still
    /// gets the picture it had before.
    #[must_use]
    pub fn off() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            strength: 0.0,
            mode: 0.0,
            depth_bias: 0.0,
            _pad: 0.0,
        }
    }
}

/// The frame's shadow maps, and the one other per-frame picture a surface
/// reads, bound together in the scene group - see `shadow_map::resources` for
/// what stands in where one is `None`.
///
/// One value rather than three arguments, so a caller that draws nothing
/// shadowed says so once ([`ShadowMaps::NONE`]) and a new map is one more
/// field rather than one more position at eight call sites.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShadowMaps<'a> {
    /// Wipeout HD's own model shadow map - the `original` tier's, coverage,
    /// read by the track.
    pub coverage: Option<&'a wgpu::TextureView>,
    /// The `mapped` tier's depth map, read by everything.
    pub depth: Option<&'a wgpu::TextureView>,
    /// The per-craft sun-occlusion array (`crate::shadow::occlusion`), a
    /// `D2Array` view, read by a hull whose uniform names a layer.
    pub occlusion: Option<&'a wgpu::TextureView>,
    /// The per-craft self-shadow depth array (`crate::shadow::self_shadow`),
    /// a `D2Array` view, compared against by the same hull at the same layer.
    pub self_shadow: Option<&'a wgpu::TextureView>,
    /// Wipeout HD's behind-the-glass target (`oag_raceplay`'s
    /// `scene::behind_glass`), which Vineta K's tunnel glass reads through
    /// [`Scene::refraction`]. Not a shadow map: it is here because it is the
    /// other picture of the frame the scene group carries.
    pub behind_glass: Option<&'a wgpu::TextureView>,
}

impl ShadowMaps<'_> {
    /// No maps: every binding gets its placeholder and nothing shadows.
    pub const NONE: Self = Self {
        coverage: None,
        depth: None,
        occlusion: None,
        self_shadow: None,
        behind_glass: None,
    };
}

/// Which shadow maps a model's surfaces may read, if any.
///
/// **Three states rather than a flag**, because the two tiers have different
/// receivers and one surface must be out of both. Wipeout HD's coverage map is
/// read by the *track surface* alone - that is what its materials declare - so
/// a craft casts and never receives it. The `mapped` tier's depth map is read
/// by everything that is lit, which is what makes it *more* shadows rather
/// than better ones. And the sky is neither: it is drawn at infinity with the
/// depth test disabled, so a shadow on it is a dark patch hanging in the air.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowReceiver {
    /// Reads no map at all. The sky.
    Never,
    /// Reads the `mapped` tier's depth map only. Craft, scenery, everything
    /// else that is lit.
    Mapped,
    /// Reads both, which on Wipeout HD is the track surface.
    Both,
}

impl ShadowReceiver {
    /// The pipeline constant `mesh.wesl` compares against.
    #[must_use]
    pub fn constant(self) -> f64 {
        match self {
            Self::Never => 0.0,
            Self::Mapped => 1.0,
            Self::Both => 2.0,
        }
    }
}

/// Everything bind group 2 carries: the fog volume and the light rig.
///
/// One buffer rather than two groups because `wgpu`'s default
/// `max_bind_groups` is 4 and this pipeline already uses 0 through 3.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Scene {
    /// The fog volume the camera is inside.
    pub fog: Fog,
    /// The circuit's authored light rig, or [`Light::stand_in`].
    pub light: Light,
    /// **Engine parameter slot 0, `time`** - a global clock in seconds, which
    /// Wipeout HD's own draw-state builders splat into four lanes and bind for
    /// every surface drawn that frame
    /// (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "the engine's own
    /// parameter table"). Four lanes here for the same reason: it is what the
    /// original binds, and a scalar field would need its own padding anyway.
    ///
    /// Only the flame surface reads it so far. This project's clock is the
    /// race tick over 60 rather than the wall clock, so it is deterministic;
    /// the original's epoch is unread and does not matter to a scroll.
    pub time: [f32; 4],
    /// The Zone effect's per-stage parameters. [`Zone::default`] - all zero,
    /// `enabled` included - for every draw that is not a Zone race, which is
    /// the identity on the albedo.
    pub zone: Zone,
    /// The shadow map's projection and strength, or [`ShadowMap::off`].
    pub shadow: ShadowMap,
    /// Wipeout HD's per-frame SPU vertex lights, summed per vertex into the
    /// pre-albedo diffuse term of every track chunk - see
    /// [`super::spu_light`]. [`SpuLights::none`] for every other draw, which
    /// is the identity on the sum.
    pub spu_lights: super::SpuLights,
    /// World to each craft's sun-occlusion map, one per layer of the array
    /// `oag_render::shadow::occlusion::Maps` renders - the `directionalLight0Proj`
    /// a Wipeout HD hull is bound. Only the layer a drawable's own uniform
    /// names is read; the identity everywhere else.
    pub sun_occlusion: [[[f32; 4]; 4]; super::shadow_map::OCCLUSION_LAYERS as usize],
    /// World to the behind-the-glass target's clip space - the original's
    /// `refractProject`, which the tunnel glass projects a point through to
    /// find where it reads that target (`mesh::rcs::refraction`). The
    /// identity for every draw that reads no target.
    pub refraction: [[f32; 4]; 4],
}

impl Scene {
    /// No fog and the stand-in rig: what the sky, the asset viewer and every
    /// title but HD bind.
    #[must_use]
    pub fn off() -> Self {
        Self {
            fog: Fog::off(),
            light: Light::stand_in(),
            time: [0.0; 4],
            zone: Zone::default(),
            shadow: ShadowMap::off(),
            spu_lights: super::SpuLights::none(),
            sun_occlusion: [Mat4::IDENTITY.to_cols_array_2d();
                super::shadow_map::OCCLUSION_LAYERS as usize],
            refraction: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

const _: () = assert!(
    std::mem::size_of::<Zone>() == 224,
    "shaders/types.wesl's Zone is two vec4s and four three-vec4 sets"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, zone).is_multiple_of(16),
    "WGSL puts Scene.zone at a 16-aligned offset; Rust must agree"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, shadow).is_multiple_of(16),
    "and Scene.shadow, whose first field is a mat4x4"
);
const _: () = assert!(
    std::mem::size_of::<ShadowMap>() == 80,
    "shaders/types.wesl's ShadowMap is a mat4x4 and one padded vec4"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, spu_lights).is_multiple_of(16),
    "and Scene.spu_lights, whose array is of two-vec4 records"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, sun_occlusion).is_multiple_of(16),
    "and Scene.sun_occlusion, an array of mat4x4"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, refraction).is_multiple_of(16),
    "and Scene.refraction, a mat4x4"
);
const _: () = assert!(
    std::mem::size_of::<Scene>().is_multiple_of(16),
    "a uniform buffer's own size is 16-aligned in WGSL too"
);

/// Size, in bytes, of bind group 2's uniform buffer.
///
/// **[`Scene`]'s size, which is [`Fog`] *and* [`Light`]** - 96 bytes, not the
/// 32 the name `FOG_SIZE` promised until finding R3 of the 2026-08-18 review.
/// The layout itself was right the whole time and matches `mesh.wesl` field for
/// field; what was wrong was that anyone adding to `Scene` read a constant
/// claiming to measure one of its two halves.
pub const SCENE_SIZE: u64 = std::mem::size_of::<Scene>() as u64;
