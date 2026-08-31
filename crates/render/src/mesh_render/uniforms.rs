//! The uniform blocks the mesh pipeline reads: the camera and model matrices in
//! bind group 0, and the scene's fog and light in bind group 2.
//!
//! Split out of `mesh_render.rs` for size alone. What is here is the whole of
//! how an [`Orbit`] becomes a view-projection matrix, which is worth having in
//! one place: the layout is mirrored by four `.wgsl` declarations and by the
//! asset viewer's own buffer sizing, so it moves in lockstep with them.

use oag_core::math::{Mat4, Vec3, camera};

use crate::camera::orbit::Orbit;
use crate::mesh::Model;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    /// Unused. Was the phase of a global texture-animation clock; the authored
    /// per-material keyframe blocks in [`TexAnims`] replaced it. Kept as
    /// padding because this layout is mirrored by `mesh.wgsl`, by four other
    /// pipelines in this crate and by the asset viewer's own buffer sizing.
    _unused: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    /// The previous simulation tick's `view_projection * model`,
    /// premultiplied: what the velocity target measures screen motion
    /// against. See `mesh.wgsl`'s own mirror for why this is one matrix
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
        _unused: 0.0,
        _pad0: 0.0,
        _pad1: 0.0,
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

/// The fog block `mesh.wgsl` reads from bind group 2.
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
    /// Which curve `mesh.wgsl` applies. `0.0` is the GE's linear ramp between
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
    _pad2: f32,
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
            _pad2: 0.0,
        }
    }

    /// Fog from one sampled [`oag_formats::fog::FogParams`] and the eye it was
    /// sampled at.
    #[must_use]
    pub fn new(params: &oag_formats::fog::FogParams, camera: [f32; 3]) -> Self {
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
            _pad2: 0.0,
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
            _pad2: 0.0,
        }
    }
}

/// The authored light rig a circuit's own settings file states.
///
/// # Why this exists
///
/// **`mesh.wgsl`'s two-light rig is a stand-in and says so**, with invented
/// directions chosen so geometry reads clearly. Wipeout HD authors the real
/// thing in plain text, one file per circuit - see
/// [`oag_formats::envsettings`] - and `CLAUDE.md`'s rule about not inventing
/// what the assets already author applies directly.
///
/// # What is the disc's and what is this project's
///
/// **The combination is the disc's**, read out of the circuit materials' own
/// fragment microcode
/// (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`) rather than assumed:
///
/// ```text
/// light  = ambient + prelit_scale * lightmap^prelit_power
///        + sun * max(dot(N, L), 0) * lightmap.a
/// colour = diffuse * light
///        + sun * specular_scale * pow(max(dot(H, N), 0), 32)
///          * max(dot(N, L), 0) * lightmap.a * diffuse.a
/// ```
///
/// The lightmap's alpha gating the direct sun - a baked shadow mask - and the
/// prelit power curve are the two halves this project used to replace with a
/// plain multiply. **The magnitudes are the disc's too, and the arithmetic is
/// done in linear light** as the RSX does - samples sRGB-decoded, lit, the
/// result saturated and encoded back into the gamma target - per
/// [ADR-0026](../../../docs/architecture/adr/0026-hd-authored-lighting-is-linear.md),
/// which narrows ADR-0020 to the titles its GE argument is about. The
/// saturate is this project's stand-in for HD's unread exposure stage and is
/// judged against an rpcs3 reference frame; the per-vertex additive term the
/// original's vertex programs interpolate on top (dynamic lights among them)
/// is not reproduced.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    /// Unit direction **towards** the light, which is what a normal dots with.
    pub direction: [f32; 3],
    /// `1.0` to use this rig, `0.0` for `mesh.wgsl`'s stand-in.
    pub enabled: f32,
    /// Constant ambient, as authored and clamped into range.
    pub ambient: [f32; 3],
    _pad0: f32,
    /// The sun's colour with its magnitude divided out: the hue alone.
    pub sun: [f32; 3],
    _pad1: f32,
    /// `Lighting.Prelit ambient colour scale`, applied exactly where the
    /// circuit's own fragment microcode applies it:
    /// `prelit = scale * lightmap^power`. See [`Light::authored`] for what
    /// of the equation is read and what is this project's.
    pub prelit_scale: [f32; 3],
    /// `Lighting.Sun specular scale`, weighting the read specular term
    /// `pow(max(dot(H, N), 0), 32) * max(dot(N, L), 0) * lightmap.a`.
    /// The exponent 32 is an **inline** microcode constant, not a patched
    /// parameter, so it lives in `mesh.wgsl` rather than here.
    pub specular_scale: f32,
    /// `Lighting.Prelit ambient colour power` - the exponent in the same
    /// prelit term.
    pub prelit_power: [f32; 3],
    _pad2: f32,
}

impl Light {
    /// The stand-in rig: what every title but HD binds.
    ///
    /// A value rather than an unbound group, for the reason [`Fog::off`] is
    /// one: WGSL has no optional bindings.
    #[must_use]
    pub fn stand_in() -> Self {
        Self {
            direction: [0.0, 1.0, 0.0],
            enabled: 0.0,
            ambient: [0.0; 3],
            _pad0: 0.0,
            sun: [0.0; 3],
            _pad1: 0.0,
            prelit_scale: [1.0; 3],
            specular_scale: 0.0,
            prelit_power: [1.0; 3],
            _pad2: 0.0,
        }
    }

    /// The rig a circuit authors.
    ///
    /// `direction` must already be normalised;
    /// [`oag_formats::envsettings::EnvSettings::direction`] does it and answers
    /// `None` for the degenerate triples four circuits write, which is why this
    /// takes a direction rather than a settings file.
    ///
    /// Every value is passed through as authored - the equation these feed is
    /// the one read out of the circuit's own microcode, and reducing a term of
    /// it would un-read it. Where the sum leaves the range this target holds,
    /// the target saturates; that clamp is the documented stand-in for HD's
    /// missing tonemap stage - see the type-level docs.
    #[must_use]
    pub fn authored(
        direction: [f32; 3],
        colour: [f32; 3],
        ambient: [f32; 3],
        prelit_scale: [f32; 3],
        prelit_power: [f32; 3],
        specular_scale: f32,
    ) -> Self {
        Self {
            direction,
            enabled: 1.0,
            ambient,
            _pad0: 0.0,
            sun: colour,
            _pad1: 0.0,
            prelit_scale,
            specular_scale,
            // A power of zero would turn an unsampled black lightmap texel
            // into full white; the floor keeps the curve monotone in the
            // lightmap without changing any authored value (the corpus
            // authors 1.0 to 2.0).
            prelit_power: std::array::from_fn(|i| prelit_power[i].max(1e-3)),
            _pad2: 0.0,
        }
    }
}

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
/// surface = zoneTex(zoneUV).rgb * zoneEffect<I|O>.rgb
///         + zoneBase<I|O>.rgb    * rim^10
///         + zoneBaseAlt<I|O>.rgb * rim^5
/// ```
///
/// **That is the whole surface colour: the material's own albedo does not
/// enter it.** Which is the finding that carries the look - the original's
/// Zone frame is near-monochrome because the circuit's diffuse is *gone*, not
/// because a tint was added over it. The `10` and `5` are inline literals in
/// the microcode, not parameters, so only the two colours are per-stage.
///
/// # Which of the disc's two Zone shapes this is
///
/// A census of every `.rcsmaterial` in `DATA00.PSARC` - 20,092 fragment
/// blocks that name a Zone parameter or sampler - finds exactly two shapes,
/// with no block in both:
///
/// | shape | blocks | `rim^10`/`rim^5` | `blackMask` | albedo RGB |
/// | --- | ---: | ---: | ---: | ---: |
/// | `zoneBase` (this one) | 19,958 | all | **none** | 1,832 |
/// | `zoneAniso` | 134 | none | all | 90 |
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
/// # Two approximations, stated
/// **The recolour reaches every surface this renderer draws, and the original
/// splits its surfaces in two.** HD publishes these parameters twice, and the
/// two publications are paired end to end: one block binds the `zoneMode*`
/// textures beside the `Scene.*` colours, the other binds `zoneModeTrack*`
/// beside the `Track.*` ones. So "which texture set" and "which colour group"
/// are one choice, not two - and the reading, from the names, from the general
/// set being fifteen flat whites, and from the maintainer's own observation
/// that the *floor* shows the equaliser, is that `Scene` is scenery and
/// `Track` is the track surface.
///
/// **This build takes the `Track` half of that pair and applies it to
/// everything**, because it has no scenery/track distinction to branch on. So
/// scenery here gets the track's recolour where the original would give it the
/// blank set and a colour of its own. Preferred over the `Scene` half, which
/// would be self-inconsistent - it would feed the track texture set colours
/// the original only ever pairs with the blank one. Which block a draw goes
/// through is genuinely unread: the two sit in one function whose basic blocks
/// the scheduler has reordered, so telling them apart needs control-flow
/// reconstruction rather than peephole reading.
///
/// On top of that, the original applies the variant per material at all:
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
/// specular exponent and the blanket sun term `mesh.wgsl` already carries.
///
/// **The sum happens in each shading path's own colour space.** `zoneTex` is a
/// texture and takes the same `pow(x, 2.2)` decode every other sample here
/// takes; `zoneEffect` is a shader parameter and takes none - it is authored
/// past `1.0` (`Track.Texture Colour` reaches `9.0`), which is a
/// multiplier's range. Getting
/// that wrong is invisible whenever `zoneEffect` is exactly `1.0`, which is
/// why `crates/render/tests/zone_recolour.rs` binds `2.0`.
///
/// **Three terms of the recovered rule are deliberately absent**, because
/// nothing on the disc feeds them and this project does not invent:
///
/// 1. The whole `zoneAniso` shape - `2 * zoneAnisoPalette[pow(rim,
///    zoneAnisoPower)]` and the `blackMask` gate that comes with it. Its
///    palette textures (`0x00c81330`-`0x00c81350`) have no located filling
///    write, and it applies to the four Zone arenas rather than to a circuit,
///    so it is absent by scope as much as for want of a source.
/// 2. The visualiser glow, `saturate(N.y - 0.5) * ... * zoneTexVis[band]`: the
///    256-entry lookup is built at runtime from a static table inside the
///    executable, which is game content this project may not carry, and the
///    build loop's own arithmetic does not yet close.
/// 3. The inner/outer sphere test. `zoneOrigin` has no located writer at all,
///    and the radius is a field of a per-environment struct whose own writer is
///    unfound. It does not matter here: the split is a *stage-transition
///    wavefront* between stage `n` and stage `n - 1`, and this build never has
///    a transition in flight, so both sides read the same stage and the test is
///    a no-op whichever way it would have gone.
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
    pub enabled: f32,
    pub _pad: f32,
    /// `zoneEffect<Inner|Outer>` - the showing stage's `Track.Texture Colour`
    /// in `.rgb`. `Track`, not `Scene`, because the texture set decides it -
    /// see the paired publication above. The `.w` scales the glow this build
    /// does not draw and is left at zero rather than filled with a stand-in.
    pub effect: [f32; 4],
    /// `zoneBase<Inner|Outer>` in `.rgb` - the showing stage's
    /// `Track.Base Colour Highlight`, the `rim^10` summand's colour. `.w`
    /// unused; the exponent is an inline literal in the microcode, not a
    /// parameter, so there is nothing per-stage to carry for it.
    ///
    /// **Carried, not drawn.** `zoneBase*` belongs to the Zone variant's
    /// *untextured* material family (`cf_constantcolourglow` and its kin),
    /// where the whole surface is the two rim terms and there is no albedo to
    /// add them to. Adding them on this path - the textured one - would light
    /// every surface the original leaves alone. The field exists so the value
    /// reaches the GPU in one piece the day a per-material branch can select
    /// that family; `mesh.wgsl` declares it and reads it nowhere.
    pub base: [f32; 4],
    /// `zoneBaseAlt<Inner|Outer>` in `.rgb` - the showing stage's
    /// `Track.Base Colour`, the `rim^5` summand's colour. `.w` unused, and
    /// carried rather than drawn, as [`Self::base`].
    pub base_alt: [f32; 4],
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
        }
    }
}

const _: () = assert!(
    std::mem::size_of::<Zone>() == 64,
    "mesh.wgsl's Zone is four vec4s"
);
const _: () = assert!(
    std::mem::offset_of!(Scene, zone).is_multiple_of(16),
    "WGSL puts Scene.zone at a 16-aligned offset; Rust must agree"
);
const _: () = assert!(
    std::mem::size_of::<Scene>().is_multiple_of(16),
    "a uniform buffer's own size is 16-aligned in WGSL too"
);

/// Size, in bytes, of bind group 2's uniform buffer.
///
/// **[`Scene`]'s size, which is [`Fog`] *and* [`Light`]** - 96 bytes, not the
/// 32 the name `FOG_SIZE` promised until finding R3 of the 2026-08-18 review.
/// The layout itself was right the whole time and matches `mesh.wgsl` field for
/// field; what was wrong was that anyone adding to `Scene` read a constant
/// claiming to measure one of its two halves.
pub const SCENE_SIZE: u64 = std::mem::size_of::<Scene>() as u64;
