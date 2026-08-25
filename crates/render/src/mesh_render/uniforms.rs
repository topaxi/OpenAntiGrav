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

    Uniforms {
        view_projection: view_projection(model, aspect, orbit).to_cols_array_2d(),
        model: Mat4::from_translation(-centre).to_cols_array_2d(),
        _unused: 0.0,
        _pad0: 0.0,
        _pad1: 0.0,
        _pad2: 0.0,
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
        }
    }
}

/// Size, in bytes, of bind group 2's uniform buffer.
///
/// **[`Scene`]'s size, which is [`Fog`] *and* [`Light`]** - 96 bytes, not the
/// 32 the name `FOG_SIZE` promised until finding R3 of the 2026-08-18 review.
/// The layout itself was right the whole time and matches `mesh.wgsl` field for
/// field; what was wrong was that anyone adding to `Scene` read a constant
/// claiming to measure one of its two halves.
pub const SCENE_SIZE: u64 = std::mem::size_of::<Scene>() as u64;
