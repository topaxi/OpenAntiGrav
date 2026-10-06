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
    /// [`Race::camera_cuts`] as of the last promotion: a change is a cut, and the blur must not
    /// read the jump as motion.
    cuts: u32,
    prev: Snapshot,
    cur: Snapshot,
}

/// One tick's worth of the matrices anything draws with.
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub(super) view_projection: Mat4,
    /// The impact shake as a view-space rotation that tick - identity with
    /// none - for the blur to take out of its velocities. See
    /// [`Race::shake_screen_motion`].
    pub(super) shake: Mat4,
    /// The camera's translation alone, for the sky's own `prev_mvp` - see
    /// the sky write in [`Scene::render`].
    pub(super) camera_translation: Mat4,
    ships: Vec<Mat4>,
    pub(super) rockets: Vec<Mat4>,
    /// A live Plasma bolt's own head, same terms - HD only, empty on every
    /// other source. See `Loaded::plasma_ball_model`.
    pub(super) plasma_balls: Vec<Mat4>,
    /// A live Shuriken blade's own, same terms.
    pub(super) shurikens: Vec<Mat4>,
    /// The Mine's own, same terms as [`Self::rockets`].
    pub(super) mines: Vec<Mat4>,
    /// The Bomb's own, same terms.
    pub(super) bombs: Vec<Mat4>,
    /// The Cannon round's own, same terms.
    pub(super) cannon_rounds: Vec<Mat4>,
}

impl Snapshot {
    fn take(race: &Race, cannon: &CannonDraw, view_projection: Mat4, drawn: usize) -> Self {
        Self {
            view_projection,
            shake: race.view_shake_rotation(),
            camera_translation: Mat4::from_translation(race.camera_position()),
            ships: (0..drawn)
                .map(|slot| race.ship_model_matrix_of(slot))
                .collect(),
            rockets: race.rocket_model_matrices(),
            plasma_balls: race.plasma_ball_model_matrices(),
            shurikens: race.shuriken_model_matrices(),
            mines: race.mine_model_matrices(),
            bombs: race.bomb_model_matrices(),
            cannon_rounds: race.cannon_model_matrices(cannon),
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
        cannon: &CannonDraw,
        view_projection: Mat4,
        drawn: usize,
    ) -> Snapshot {
        let mut state = cell.borrow_mut();
        match state.as_mut() {
            // A scene's first frame measures against itself: zero velocity,
            // not a smear from an uninitialised matrix.
            None => {
                let now = Snapshot::take(race, cannon, view_projection, drawn);
                *state = Some(Self {
                    tick: race.motion_tick(),
                    cuts: race.camera_cuts(),
                    prev: now.clone(),
                    cur: now.clone(),
                });
                now
            }
            Some(state) if state.tick != race.motion_tick() => {
                let now = Snapshot::take(race, cannon, view_projection, drawn);
                state.prev = std::mem::replace(&mut state.cur, now);
                state.tick = race.motion_tick();
                // **A cut is not motion.** On the tick the picture jumps to another shot - a
                // flyby cut, the flyby ending, the destroy or spectator camera, a respawn - the
                // camera's "previous" is taken from the new shot, so the blur measures no camera
                // travel across it. The craft and the projectiles keep their own previous
                // matrices: they did move.
                let cuts = race.camera_cuts();
                if cuts != state.cuts {
                    state.cuts = cuts;
                    state.prev.view_projection = state.cur.view_projection;
                    state.prev.shake = state.cur.shake;
                    state.prev.camera_translation = state.cur.camera_translation;
                }
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
        format: oag_gpu::formats::VELOCITY_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// Detects a camera cut mid-race: a view change or the player's own respawn,
/// either of which hands the next frame's resolve a history of a different
/// shot than the one it is about to draw - see
/// [docs/rendering/fsr3.md](../../../../../docs/rendering/fsr3.md#the-game-decides-when-a-reset-happens-not-the-port).
///
/// `None` before the first observation, so the sequence's opening frame reads
/// as a cut for free rather than needing a second initial-state rule kept in
/// sync with [`Scene::record_frame`]'s own `phase == 0`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct CutWatch {
    last: Option<(oag_display::display::CameraView, u32)>,
}

impl CutWatch {
    /// Whether `view`/`respawns` describe a different shot than the last
    /// observation, and records this one as the new baseline either way -
    /// so a caller that never checks the result still keeps the watch
    /// current for the frame after.
    ///
    /// **The player's own respawn count, not the field's** - [`RaceSim::respawns`]
    /// rather than `respawns_of` - because the camera being watched is the
    /// player's, and only the player's respawn moves it: an opponent
    /// recovering elsewhere on the circuit is not a cut in *this* shot, the
    /// same distinction `race/tests/respawn.rs`'s
    /// `a_respawn_in_flight_makes_everything_visible` draws for the PVS.
    ///
    /// [`RaceSim::respawns`]: crate::RaceSim::respawns
    pub(super) fn observe(
        &mut self,
        view: oag_display::display::CameraView,
        respawns: u32,
    ) -> bool {
        let cut = self.last != Some((view, respawns));
        self.last = Some((view, respawns));
        cut
    }
}

#[cfg(test)]
mod cut_watch_tests {
    use super::CutWatch;
    use oag_display::display::CameraView;

    #[test]
    fn the_first_observation_is_always_a_cut() {
        assert!(CutWatch::default().observe(CameraView::Far, 0));
    }

    #[test]
    fn the_same_view_and_respawn_count_twice_is_not_a_cut() {
        let mut watch = CutWatch::default();
        assert!(watch.observe(CameraView::Far, 0));
        assert!(!watch.observe(CameraView::Far, 0));
        assert!(!watch.observe(CameraView::Far, 0));
    }

    #[test]
    fn a_view_change_is_a_cut() {
        let mut watch = CutWatch::default();
        assert!(watch.observe(CameraView::Far, 0));
        assert!(watch.observe(CameraView::Close, 0));
        assert!(!watch.observe(CameraView::Close, 0));
    }

    #[test]
    fn a_player_respawn_is_a_cut() {
        let mut watch = CutWatch::default();
        assert!(watch.observe(CameraView::Internal, 3));
        assert!(watch.observe(CameraView::Internal, 4));
        assert!(!watch.observe(CameraView::Internal, 4));
    }
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
    ///
    /// `view` and `respawns` are the race's own [`oag_display::display::CameraView`]
    /// and [`crate::RaceSim::respawns`] this frame - passed rather than
    /// read off a `&Race` here, because [`CutWatch`] is the only thing that
    /// needs them and a parameter is cheaper to keep honest than a second
    /// borrow this function would otherwise take just to read two getters.
    pub(super) fn record_frame(
        &self,
        phases: Option<u32>,
        projection: Mat4,
        view: oag_display::display::CameraView,
        respawns: u32,
    ) {
        let phase = self.frame_index.get();
        let mut watch = self.cut_watch.get();
        let cut = watch.observe(view, respawns);
        self.cut_watch.set(watch);
        self.last_frame.set(phases.map(|phase_count| TemporalFrame {
            camera: oag_post::fsr3::camera_from_projection(projection),
            jitter: oag_post::jitter::offset_pixels(phase, phase_count),
            phase_count,
            // The sequence's first frame has nothing behind it, and a camera
            // cut mid-race - a view change or the player's own respawn - hands
            // the resolve a history of a different shot, so both throw it away
            // the same way. See `CutWatch`.
            reset: phase == 0 || cut,
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
        self.msaa.samples()
    }
}

/// What one frame was drawn with, for the upscaler that resolves it.
///
/// `Copy` and four numbers, so [`Scene`] can hold it in a `Cell` beside the
/// frame counter it is derived from.
#[derive(Debug, Clone, Copy)]
pub struct TemporalFrame {
    /// The camera, in the terms FSR 3.1 asks for.
    pub camera: oag_post::fsr3::Camera,
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
    /// of the velocity target - `oag_post::jitter`'s own tests carry that claim.
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
    /// ([`oag_post::jitter::phases`]); nothing at all wants
    /// [`oag_post::jitter::DEFAULT_PHASES`].
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
        let jitter = oag_post::jitter::matrix(frame, phases, (viewport.2, viewport.3));
        (jitter * view_projection, jitter * prev_vp)
    }
}

impl super::Scene {
    /// The blur's gather resolution from the next frame on. Called before
    /// every draw rather than once, so the menu row applies live, the way
    /// `Race::set_texture_detail` does.
    pub fn set_blur_resolution(&self, resolution: oag_display::display::BlurResolution) {
        self.blur_half
            .set(resolution == oag_display::display::BlurResolution::Half);
    }

    /// Encodes the motion blur over the finished frame, `false` when the
    /// pass is absent or off. `views` are the frame, the velocity attachment
    /// and the depth attachment. `camera_shake` is
    /// [`Race::shake_screen_motion`], which the pass takes out of every
    /// velocity it reads.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn encode_motion_blur(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        views: [&wgpu::TextureView; 3],
        viewport: (f32, f32, f32, f32),
        strength: f32,
        camera_shake: Mat4,
        timestamps: Option<oag_post::motion_blur::ChainTimestamps<'_>>,
    ) -> bool {
        let Some(pass) = &self.motion_blur else {
            return false;
        };
        let size = self.depth.size();
        let [scene, velocity, depth] = views;
        let mut pass = pass.borrow_mut();
        pass.set_half_resolution(self.blur_half.get());
        pass.render(
            device,
            queue,
            encoder,
            &oag_post::motion_blur::Frame {
                scene,
                velocity,
                depth,
                sample_count: self.msaa.samples(),
                size: (size.width, size.height),
                viewport,
                strength,
                camera_shake,
            },
            timestamps,
        )
    }
}

#[cfg(test)]
mod advance_tests {
    use super::*;

    fn race() -> Race {
        Race::start(Setup::headless(oag_race::Mode::TimeTrial, 1))
    }

    fn at(x: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(x, 0.0, 0.0))
    }

    /// One frame at `x` after the world has ticked once more.
    fn frame(cell: &std::cell::RefCell<Option<MotionState>>, race: &mut Race, x: f32) -> Snapshot {
        race.tick(&oag_gameplay::PlayerInputs::none());
        MotionState::advance(cell, race, &CannonDraw::default(), at(x), 1)
    }

    #[test]
    fn the_camera_of_the_previous_tick_is_what_the_blur_measures_against() {
        let cell = std::cell::RefCell::new(None);
        let mut race = race();
        let first = MotionState::advance(&cell, &race, &CannonDraw::default(), at(0.0), 1);
        assert_eq!(
            first.view_projection,
            at(0.0),
            "the first frame measures against itself"
        );
        let prev = frame(&cell, &mut race, 1.0);
        assert_eq!(prev.view_projection, at(0.0));
        let prev = frame(&cell, &mut race, 2.0);
        assert_eq!(prev.view_projection, at(1.0));
    }

    #[test]
    fn a_cut_measures_no_camera_travel() {
        let cell = std::cell::RefCell::new(None);
        let mut race = race();
        MotionState::advance(&cell, &race, &CannonDraw::default(), at(0.0), 1);
        frame(&cell, &mut race, 1.0);
        // The shot changes: the camera jumps a hundred units, and the picture says so.
        race.sim.respawns[0] += 1;
        let prev = frame(&cell, &mut race, 101.0);
        assert_eq!(
            prev.view_projection,
            at(101.0),
            "the blur's previous view is the post-cut camera"
        );
        // And the tick after it is ordinary again.
        let prev = frame(&cell, &mut race, 102.0);
        assert_eq!(prev.view_projection, at(101.0));
    }

    #[test]
    fn one_tick_rerendered_keeps_its_previous() {
        let cell = std::cell::RefCell::new(None);
        let race = race();
        // Without a flyby the key is the world's tick.
        assert_eq!(race.motion_tick(), race.sim.world.tick);
        MotionState::advance(&cell, &race, &CannonDraw::default(), at(0.0), 1);
        let same = MotionState::advance(&cell, &race, &CannonDraw::default(), at(5.0), 1);
        assert_eq!(
            same.view_projection,
            at(0.0),
            "one tick re-rendered keeps its previous"
        );
    }
}
