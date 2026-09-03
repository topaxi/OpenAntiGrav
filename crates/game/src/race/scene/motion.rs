//! [`MotionState`]: the previous tick's matrices, and the velocity
//! attachment they give meaning to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::super::*;

/// The previous tick's camera and model matrices - what every drawable's
/// `prev_mvp` is measured against, and therefore what the velocity buffer
/// means.
///
/// Keyed by the simulation tick, not the frame, which is the design's
/// non-obvious decision (`docs/rendering/motion-blur.md`): frames can outrun
/// the fixed 60 Hz simulation, and consecutive frames of one tick re-render
/// identical state. Snapshotting per frame would make those repeats read as
/// zero motion and pulse the blur off; promoting the snapshot only when the
/// tick advances makes every frame of a tick measure the same one-tick
/// travel. A camera or craft that genuinely holds still simply snapshots
/// equal matrices - zero velocity with no special case.
#[derive(Debug)]
pub(super) struct MotionState {
    tick: u64,
    prev: Snapshot,
    cur: Snapshot,
}

/// One tick's worth of the matrices anything draws with.
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub(super) view_projection: Mat4,
    /// The camera's translation alone, for the sky's own `prev_mvp` - see
    /// the sky write in [`Scene::render`].
    pub(super) camera_translation: Mat4,
    ships: Vec<Mat4>,
    pub(super) rockets: Vec<Mat4>,
}

impl Snapshot {
    fn take(race: &Race, view_projection: Mat4, drawn: usize) -> Self {
        Self {
            view_projection,
            camera_translation: Mat4::from_translation(race.camera_position()),
            ships: (0..drawn)
                .map(|slot| race.ship_model_matrix_of(slot))
                .collect(),
            rockets: race.rocket_model_matrices(),
        }
    }

    /// The slot's previous model matrix, or its current one when the
    /// previous tick had no entry for it - zero object velocity, never a
    /// smear from stale or missing state.
    pub(super) fn ship(&self, slot: usize, race: &Race) -> Mat4 {
        self.ships
            .get(slot)
            .copied()
            .unwrap_or_else(|| race.ship_model_matrix_of(slot))
    }
}

impl MotionState {
    /// Folds this frame in and returns the snapshot to measure against.
    pub(super) fn advance(
        cell: &std::cell::RefCell<Option<Self>>,
        race: &Race,
        view_projection: Mat4,
        drawn: usize,
    ) -> Snapshot {
        let mut state = cell.borrow_mut();
        match state.as_mut() {
            // A scene's first frame measures against itself: zero velocity,
            // not a smear from an uninitialised matrix.
            None => {
                let now = Snapshot::take(race, view_projection, drawn);
                *state = Some(Self {
                    tick: race.world.tick,
                    prev: now.clone(),
                    cur: now.clone(),
                });
                now
            }
            Some(state) if state.tick != race.world.tick => {
                let now = Snapshot::take(race, view_projection, drawn);
                state.prev = std::mem::replace(&mut state.cur, now);
                state.tick = race.world.tick;
                state.prev.clone()
            }
            // The same tick re-rendered - a frame rate above 60 Hz - keeps
            // measuring against the same previous tick.
            Some(state) => state.prev.clone(),
        }
    }
}

/// The race pass's three attachment views, rebuilt with the textures behind
/// them and not before. See [`Scene::attachment_views`].
#[derive(Debug)]
pub(super) struct Attachments {
    pub(super) depth: wgpu::TextureView,
    pub(super) velocity: wgpu::TextureView,
    /// `None` at sample count 1, matching [`Scene::msaa_color`].
    pub(super) msaa: Option<wgpu::TextureView>,
}

impl Attachments {
    pub(super) fn new(
        depth: &wgpu::Texture,
        velocity: &wgpu::Texture,
        msaa: Option<&wgpu::Texture>,
    ) -> Self {
        Self {
            depth: depth.create_view(&wgpu::TextureViewDescriptor::default()),
            velocity: velocity.create_view(&wgpu::TextureViewDescriptor::default()),
            msaa: msaa.map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default())),
        }
    }
}

/// The scene's velocity attachment - see `Scene::velocity`.
pub(super) fn velocity_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race velocity"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: oag_render::mesh_render::VELOCITY_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

impl super::Scene {
    /// Records what this frame is being drawn with, for the upscaler that
    /// resolves it afterwards and can derive none of it.
    ///
    /// **Called before [`super::Scene::jittered`] applies the offset**, and
    /// with the *unjittered* projection: the camera FSR 3.1 wants is the one
    /// the scene was framed by, and the offset it wants is a number rather than
    /// a matrix it would have to factor back out.
    ///
    /// `camera_from_projection` recovers the three values from the matrix
    /// rather than re-deriving them - the projection is the tick's own
    /// speed-widened field of view, and computing that twice would be a second
    /// answer to the same question. Its one lossy term, `far`, is not one the
    /// depth transform is sensitive to; see its own documentation.
    pub(super) fn record_frame(&self, phases: Option<u32>, projection: Mat4) {
        let phase = self.frame_index.get();
        self.last_frame.set(phases.map(|phase_count| TemporalFrame {
            camera: oag_render::post::fsr3::camera_from_projection(projection),
            jitter: oag_render::jitter::offset_pixels(phase, phase_count),
            phase_count,
            // The sequence's first frame has nothing behind it. A camera *cut*
            // mid-race is not detected here and would want the same treatment -
            // see the handover thread.
            reset: phase == 0,
        }));
    }

    /// The depth and velocity attachments, and what the last frame was drawn
    /// with - everything a temporal upscaler needs and cannot derive.
    ///
    /// `None` before the first [`Scene::render`], which is the same frame on
    /// which there would be no history anyway.
    #[must_use]
    pub fn temporal(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView, TemporalFrame)> {
        self.last_frame.get().map(|frame| {
            (
                &self.attachment_views.depth,
                &self.attachment_views.velocity,
                frame,
            )
        })
    }

    /// How many samples this scene's attachments carry.
    ///
    /// **What the scene was built with, not what the setting says.** MSAA's
    /// sample count is baked into every pipeline at `Scene::new`, so the row
    /// and the scene disagree for a whole race after a player moves it - which
    /// is what that row's `restart_required` note exists for, and which would
    /// otherwise hand a temporal upscaler a bind group of the wrong shape.
    #[must_use]
    pub fn sample_count(&self) -> u32 {
        self.anti_aliasing.msaa_samples()
    }
}

/// What one frame was drawn with, for the upscaler that resolves it.
///
/// `Copy` and four numbers, so [`Scene`] can hold it in a `Cell` beside the
/// frame counter it is derived from.
#[derive(Debug, Clone, Copy)]
pub struct TemporalFrame {
    /// The camera, in the terms FSR 3.1 asks for.
    pub camera: oag_render::post::fsr3::Camera,
    /// The sub-pixel offset this frame was actually drawn with, in pixels.
    pub jitter: (f32, f32),
    /// The jitter sequence's length.
    pub phase_count: u32,
    /// Whether this was the first frame of the sequence, and so has no history
    /// behind it.
    pub reset: bool,
}

impl super::Scene {
    /// Applies this frame's camera jitter, or nothing when it is off.
    ///
    /// **Here rather than inline in `Scene::render` because the ordering is the
    /// decision** ([ADR-0039](../../../../docs/architecture/adr/0039-camera-jitter-post-multiplies-onto-the-view-projection.md)),
    /// and a named call is harder to move by accident than four lines of matrix
    /// arithmetic. It must be called *after* the frustum is built and after
    /// [`MotionState::advance`] has stored the tick's snapshot, so that neither
    /// culling nor the previous-tick matrix a future frame measures against ever
    /// sees a sub-pixel wobble.
    ///
    /// Both matrices take the **same** phase, which is what cancels the offset out
    /// of the velocity target - `oag_render::jitter`'s own tests carry that claim.
    /// The counter advances whether or not jitter is on, so turning it on does not
    /// restart the sequence.
    ///
    /// Returned as a pair for the caller to shadow its own bindings with: the
    /// dozen use sites downstream then pick the jittered matrices up without one
    /// of them being missed, and a miss would be wrong only with jitter on.
    ///
    /// `phases` is `None` for off, and otherwise the sequence length - which is
    /// a property of whatever is *resolving* these frames, not of the camera.
    /// A temporal upscaler wants more phases the further it is magnifying
    /// ([`oag_render::jitter::phases`]); nothing at all wants
    /// [`oag_render::jitter::DEFAULT_PHASES`].
    pub(super) fn jittered(
        &self,
        phases: Option<u32>,
        viewport: (f32, f32, f32, f32),
        view_projection: Mat4,
        prev_vp: Mat4,
    ) -> (Mat4, Mat4) {
        let frame = self.frame_index.get();
        self.frame_index.set(frame.wrapping_add(1));
        let Some(phases) = phases else {
            return (view_projection, prev_vp);
        };
        let jitter = oag_render::jitter::matrix(frame, phases, (viewport.2, viewport.3));
        (jitter * view_projection, jitter * prev_vp)
    }
}
