//! FSR 3.1's per-frame constant buffer, and the camera arithmetic behind two
//! of its fields.
//!
//! Transliterated from `Fsr3UpscalerConstants` in
//! `ffx_fsr3upscaler_private.h` and the block of
//! `ffxFsr3UpscalerContextDispatch` that fills it, in AMD's MIT-licensed
//! FidelityFX SDK `v1.1.4` (`c6efa6bf7f2027b3ec94f28578bb5965eabb9e55`). See
//! `licences/AMD-FidelityFX-MIT.txt` and
//! [fsr3.md](../../../../docs/rendering/fsr3.md).
//!
//! **The field order is upstream's declaration order and must stay that way.**
//! HLSL packs a constant buffer into 16-byte rows with no member straddling a
//! row boundary, and WGSL's uniform layout rules produce the same offsets for
//! this particular sequence - every `vec2` lands 8-aligned and the one `vec4`
//! lands at 48. That agreement is what lets `fsr3/common.wesl` declare the
//! struct in upstream's order too, and it is why reordering a field here to
//! "group related things" would silently move every field after it.

use oag_core::math::Mat4;

/// The camera this frame was drawn with, in the terms FSR 3.1 asks for.
///
/// Upstream takes near, far and the **vertical** field of view as dispatch
/// parameters rather than taking the projection matrix, and derives what it
/// needs from them; this carries the same three so the derivation can stay a
/// transliteration rather than becoming a matrix inversion of our own devising.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The near plane's view-space distance.
    pub near: f32,
    /// The far plane's view-space distance.
    pub far: f32,
    /// Vertical field of view, in radians - the argument
    /// `oag_core::math::camera::perspective` itself takes.
    pub fov_y: f32,
}

impl Camera {
    /// `setupDeviceDepthToViewSpaceDepthParams`, for this project's one depth
    /// convention.
    ///
    /// **Upstream branches four ways and this takes one branch**, which is a
    /// deliberate narrowing rather than an omission: `FFX_FSR3UPSCALER_ENABLE_DEPTH_INVERTED`
    /// and `..._INFINITE` select between reversed and infinite projections, and
    /// this renderer has exactly one - `glam`'s right-handed DirectX-style
    /// `perspective`, non-reversed and finite, via
    /// [`oag_core::math::camera`]. Carrying the other three branches would be
    /// carrying code no call site can reach and no test can cover. If a
    /// reversed-Z projection ever lands, `matrix_elem_c` and `matrix_elem_e`
    /// upstream are where the other rows are.
    ///
    /// The four values answer two separate questions, and the shader uses them
    /// in two separate places: `xy` reconstruct view-space depth from the depth
    /// buffer's non-linear value, `zw` scale a screen position back out to view
    /// space so a pixel's neighbour is a known distance away in metres.
    #[must_use]
    fn device_to_view_depth(self, aspect: f32) -> [f32; 4] {
        let min = self.near.min(self.far);
        let max = self.near.max(self.far);
        // fQ, and `d` is upstream's own named -1 "for clarity".
        let q = max / (min - max);
        let d = -1.0f32;
        let cot_half_fov_y = (0.5 * self.fov_y).cos() / (0.5 * self.fov_y).sin();
        let a = cot_half_fov_y / aspect;
        let b = cot_half_fov_y;
        [d * q, q * min, 1.0 / a, 1.0 / b]
    }

    /// `tanHalfFOV`: the tangent of **half the horizontal** field of view.
    ///
    /// Upstream converts the vertical angle it was handed into a horizontal one
    /// through the aspect ratio and then takes the tangent of half of it, which
    /// is not the same as scaling `tan(fovY/2)` by the aspect - the conversion
    /// goes through `atan`. Kept as upstream wrote it.
    #[must_use]
    fn tan_half_fov(self, aspect: f32) -> f32 {
        let horizontal = ((self.fov_y / 2.0).tan() * aspect).atan() * 2.0;
        (horizontal * 0.5).tan()
    }
}

impl Default for Camera {
    /// Nothing meaningful - a unit frustum, so that a `Default` constants block
    /// is well-formed rather than full of infinities.
    fn default() -> Self {
        Self {
            near: 0.1,
            far: 1000.0,
            fov_y: std::f32::consts::FRAC_PI_2,
        }
    }
}

/// What a frame gives the upscaler that changes every frame.
///
/// Split from [`Constants`] because a `Constants` is *derived* from one of
/// these plus the previous frame's - the four `previousFrame*` fields are not
/// something a caller supplies, and asking a caller for them is asking it to
/// keep state the upscaler already keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dispatch {
    /// The rectangle of the scene target actually drawn this frame -
    /// upstream's `renderSize`.
    pub render: (u32, u32),
    /// The scene target's allocation - upstream's `maxRenderSize`. Larger than
    /// [`Dispatch::render`] whenever a controller is varying the extent, per
    /// [ADR-0037](../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md).
    pub max_render: (u32, u32),
    /// The presentation rectangle - upstream's `upscaleSize`.
    pub upscale: (u32, u32),
    /// This frame's sub-pixel camera offset in pixels, `-0.5..0.5` on each
    /// axis - exactly what `oag_post::jitter::offset_pixels` returned for the
    /// matrix the scene was actually drawn with.
    pub jitter: (f32, f32),
    /// The jitter sequence's length, `oag_post::jitter::phases`.
    pub phase_count: u32,
    /// The camera the scene was drawn with.
    pub camera: Camera,
    /// Seconds since the previous frame. Upstream clamps this to `0..=1`.
    pub delta_time: f32,
    /// Whether the history is meaningless and must be thrown away - a race
    /// restart, a camera cut, the first frame after a stage change.
    pub reset: bool,
    /// How many samples the scene's depth and velocity attachments carry - 1,
    /// or MSAA's 4.
    ///
    /// **The only thing it selects is which build of `prepare_inputs` runs**,
    /// because that is the one pass reading the scene's own attachments;
    /// everything after it reads targets this port wrote, which are never
    /// multisampled. It is not part of the constant buffer.
    pub sample_count: u32,
    /// How hard the final RCAS pass sharpens.
    ///
    /// The same [`super::super::fsr1::Sharpness`] the FSR 1 path takes, and
    /// deliberately so: it is upstream's own scale in both, so a player moving
    /// the UPSCALER SHARPNESS row means the same thing whichever upscaler is
    /// running.
    pub sharpness: crate::fsr1::Sharpness,
}

impl Default for Dispatch {
    fn default() -> Self {
        Self {
            render: (1, 1),
            max_render: (1, 1),
            upscale: (1, 1),
            jitter: (0.0, 0.0),
            phase_count: crate::jitter::DEFAULT_PHASES,
            camera: Camera::default(),
            delta_time: 1.0 / 60.0,
            reset: true,
            sample_count: 1,
            sharpness: crate::fsr1::Sharpness::DEFAULT,
        }
    }
}

/// The constant buffer every FSR 3.1 pass binds, in upstream's field order.
///
/// `i32` pairs and bare `f32`s rather than `[f32; n]` arrays wherever upstream
/// uses a two- or four-element array, because an array in a WGSL uniform has a
/// 16-byte stride and a `vec2` does not: written as arrays this would need
/// three times the padding and would stop matching the HLSL layout it is a
/// transliteration of.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Constants {
    pub render_size: [i32; 2],
    pub previous_frame_render_size: [i32; 2],
    pub upscale_size: [i32; 2],
    pub previous_frame_upscale_size: [i32; 2],
    pub max_render_size: [i32; 2],
    pub max_upscale_size: [i32; 2],
    pub device_to_view_depth: [f32; 4],
    pub jitter_offset: [f32; 2],
    pub previous_frame_jitter_offset: [f32; 2],
    pub motion_vector_scale: [f32; 2],
    pub downscale_factor: [f32; 2],
    pub motion_vector_jitter_cancellation: [f32; 2],
    pub tan_half_fov: f32,
    pub jitter_phase_count: f32,
    pub delta_time: f32,
    pub delta_pre_exposure: f32,
    pub view_space_to_meters_factor: f32,
    pub frame_index: f32,
    pub velocity_factor: f32,
    pub reactiveness_scale: f32,
    pub shading_change_scale: f32,
    pub accumulation_added_per_frame: f32,
    pub min_disocclusion_accumulation: f32,
    /// RCAS's sharpening lobe, already through `exp2(-stops)`.
    ///
    /// **Ours, and it lives in upstream's padding on purpose.** Upstream keeps
    /// a *separate* constant buffer for the RCAS pass holding this one value;
    /// carrying a second uniform and a second bind group for a single float
    /// would be more machinery than the thing it carries. The 148-byte struct
    /// rounds up to 160 either way, so this occupies space that already
    /// existed - and it is placed *after* every upstream field, so the layout
    /// this block is a transliteration of is untouched.
    pub rcas_sharpness: f32,
    /// To the 16-byte multiple a uniform buffer's size must be.
    pub padding: [f32; 2],
}

/// This renderer's velocity attachment holds the **current-minus-previous** UV
/// delta (`shaders/velocity.wesl`'s `velocity_of`), and FSR 3.1 reprojects with
/// `uv + motionVector` - so the vector it wants points the other way.
///
/// Upstream's `motionVectorScale` exists to convert whatever units an engine
/// stores into UV, by dividing the app's value by the render size. Ours are
/// already UV, so the whole conversion is the sign, and stating it as `-1`
/// here is more honest than passing `-renderWidth` through a division that
/// cancels it.
const MOTION_VECTOR_SCALE: [f32; 2] = [-1.0, -1.0];

/// Upstream's own defaults for the five tuning scalars, set in
/// `ffxFsr3UpscalerContextCreate` and adjustable through
/// `ffxFsr3UpscalerSetConstant`. Nothing here adjusts them yet; they are named
/// so that a future setting has somewhere to point.
const VELOCITY_FACTOR: f32 = 1.0;
const REACTIVENESS_SCALE: f32 = 1.0;
const SHADING_CHANGE_SCALE: f32 = 1.0;
const ACCUMULATION_ADDED_PER_FRAME: f32 = 1.0 / 3.0;
const MIN_DISOCCLUSION_ACCUMULATION: f32 = -1.0 / 3.0;

impl Constants {
    /// This frame's constants, given `previous`'s.
    ///
    /// `previous` is `None` on the first frame and after a reset, which is
    /// upstream's `resetAccumulation` path: the four `previousFrame*` fields
    /// then describe this frame rather than a frame that never happened, and
    /// [`Constants::frame_index`] restarts at zero.
    #[must_use]
    pub fn new(dispatch: Dispatch, previous: Option<&Self>) -> Self {
        let render = (dispatch.render.0.max(1), dispatch.render.1.max(1));
        let upscale = (dispatch.upscale.0.max(1), dispatch.upscale.1.max(1));
        let aspect = render.0 as f32 / render.1 as f32;
        let size = |(w, h): (u32, u32)| [w.max(1) as i32, h.max(1) as i32];
        let jitter = [dispatch.jitter.0, dispatch.jitter.1];

        // Upstream ramps the phase count by one per frame towards its target
        // rather than jumping to it, so that a resolution change does not
        // discontinuously re-index the sequence mid-accumulation. Kept.
        let wanted = dispatch.phase_count.max(1) as f32;
        let jitter_phase_count = match previous.map(|p| p.jitter_phase_count) {
            Some(held) if held > 0.0 && held < wanted => held + 1.0,
            Some(held) if held > wanted => held - 1.0,
            Some(held) if held > 0.0 => held,
            _ => wanted,
        };

        Self {
            render_size: size(render),
            previous_frame_render_size: previous.map_or(size(render), |p| p.render_size),
            upscale_size: size(upscale),
            previous_frame_upscale_size: previous.map_or(size(upscale), |p| p.upscale_size),
            max_render_size: size(dispatch.max_render),
            max_upscale_size: size(upscale),
            device_to_view_depth: dispatch.camera.device_to_view_depth(aspect),
            jitter_offset: jitter,
            previous_frame_jitter_offset: previous.map_or(jitter, |p| p.jitter_offset),
            motion_vector_scale: MOTION_VECTOR_SCALE,
            downscale_factor: [
                render.0 as f32 / upscale.0 as f32,
                render.1 as f32 / upscale.1 as f32,
            ],
            // **Zero, and that is a property of this renderer rather than a
            // simplification.** Upstream cancels a jitter baked into the
            // motion vectors for engines whose velocity pass sees the jittered
            // matrix on one side only. Ours applies the same offset to both the
            // current and the previous view-projection, so it cancels out of
            // the difference before the buffer is ever written - which is the
            // claim ADR-0039 is about and `jitter`'s own tests assert.
            motion_vector_jitter_cancellation: [0.0, 0.0],
            tan_half_fov: dispatch.camera.tan_half_fov(aspect),
            jitter_phase_count,
            delta_time: dispatch.delta_time.clamp(0.0, 1.0),
            // One, because nothing here pre-exposes. The field is the *ratio*
            // between this frame's exposure and the previous frame's, so one
            // means "unchanged" rather than "unset".
            delta_pre_exposure: 1.0,
            // One, because this renderer's world units are metres already -
            // `docs/physics/` measures the craft in them.
            view_space_to_meters_factor: 1.0,
            frame_index: previous.map_or(0.0, |p| p.frame_index + 1.0),
            velocity_factor: VELOCITY_FACTOR,
            reactiveness_scale: REACTIVENESS_SCALE,
            shading_change_scale: SHADING_CHANGE_SCALE,
            accumulation_added_per_frame: ACCUMULATION_ADDED_PER_FRAME,
            min_disocclusion_accumulation: MIN_DISOCCLUSION_ACCUMULATION,
            rcas_sharpness: dispatch.sharpness.factor(),
            padding: [0.0; 2],
        }
    }

    /// The view-projection-independent check that this block's Rust layout is
    /// the one `common.wesl` declares.
    ///
    /// Sixteen bytes' worth of padding is the whole reason this is worth
    /// asserting: get it wrong and every pass reads a field one row out, which
    /// produces a picture rather than an error.
    pub const SIZE: usize = size_of::<Self>();
}

/// A `Mat4`'s vertical field of view and planes, for a caller that has the
/// projection rather than the three numbers.
///
/// **Not used by the game**, which builds its projection from exactly these
/// three and can pass them straight through; it exists for `oag-view` and for
/// a test that wants to check the two agree.
///
/// **Recovering `far` is ill-conditioned and this is lossy in it.** `far` comes
/// out as `e / (c + 1)` where `c + 1` is `near / (near - far)` - a small number
/// formed by adding two that nearly cancel, so at the race's own `0.1`/`1000.0`
/// it lands about a tenth of a percent out. That does not reach
/// [`Camera::device_to_view_depth`], whose `far / (near - far)` is insensitive
/// to it, but it does mean this must not be used to *round-trip* a camera and
/// compare the result for equality.
#[must_use]
pub fn camera_from_projection(projection: Mat4) -> Camera {
    // Right-handed, 0..1 depth: m[2][2] = far / (near - far) and
    // m[3][2] = near * far / (near - far), so the two solve for near and far,
    // and m[1][1] is cot(fovY / 2).
    let c = projection.z_axis.z;
    let e = projection.w_axis.z;
    let near = e / c;
    let far = e / (c + 1.0);
    let cot_half = projection.y_axis.y;
    Camera {
        near,
        far,
        fov_y: 2.0 * (1.0 / cot_half).atan(),
    }
}
