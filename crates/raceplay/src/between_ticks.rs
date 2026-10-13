//! Drawing between two simulation ticks.
//!
//! The simulation steps at a fixed 60 Hz (ADR-0007) and the window draws at
//! whatever its frame limiter allows. Before this, every frame inside one tick
//! redrew the same state, so on a 144 Hz panel or under the limiter's 240 the
//! picture moved in 60 Hz steps, each held for two to four frames unevenly.
//! Here the previous tick's draw poses are kept beside the current tick's, and
//! a draw blends the two by how far the frame clock has got into the next
//! tick, [`oag_core::TickClock::interpolation_alpha`].
//!
//! # What is blended
//!
//! The three accessors every continuous pose in the scene is read through:
//! [`Race::view`] and [`Race::view_unshaken`] (so every camera reader - the
//! sky, the fog and PVS eye, the billboards' axes, the reticle - follows),
//! the speed terms of [`Race::vertical_fov`], and [`Race::ship_model_matrix_of`]
//! (the hull, its shield, flame, plume, shadow and exhaust nozzle). What the
//! originals step - HUD numbers, sprite frames, particle spawns - stays
//! stepped.
//!
//! # Captured in the tick, not at render time
//!
//! `scene::motion::MotionState` also double-buffers view and model matrices,
//! for the motion blur. It promotes when a render sees a new tick, which below
//! 60 fps is two or more ticks apart; blending across that would sweep two
//! ticks of travel in one. This history is taken at the start of every tick
//! path instead - [`Race::tick`], [`Race::tick_intro`],
//! [`Race::tick_cosmetics`] - so the previous pose is always exactly one tick
//! old.
//!
//! # A discontinuity draws where it lands
//!
//! A blend is skipped, and the current tick drawn as it is, across anything
//! that is not motion: for the camera a cut ([`Race::camera_cuts`]: a respawn,
//! the destroy and spectator cameras, the flyby's shots), a view cycle, or a
//! pose imposed or lifted; for a craft its own respawn or a change of whether
//! it is racing. A pose that moves more than [`SNAP_DISTANCE`] in one tick is
//! drawn where it is too, as a backstop for a teleport nothing counts.
//!
//! # Byte-identical captures
//!
//! The draw alpha is `1.0` except while the window draws, and at `1.0` - or on
//! a snap - every blended reader returns the current tick's value untouched,
//! never a decomposed-and-rebuilt copy of it. A headless capture, a trace or a
//! `--screenshot`, none of which sets an alpha, draws exactly what it drew
//! before this existed. The simulation reads none of it: everything here is
//! [`RaceView`] state, out of [`RaceSim::state_hash`].
//!
//! # Chosen, not measured
//!
//! - **Interpolation, one tick late.** A blend between the last two ticks shows
//!   the state up to one tick (16.7 ms) behind. Extrapolating avoids that but
//!   overshoots on a wall hit. The originals ran a variable timestep and so did
//!   neither; nothing about this was measured against them.
//! - **Slerp, not nlerp.** A handful of matrices a frame, and slerp keeps the
//!   angular speed even across the tick.
//! - **[`SNAP_DISTANCE`]'s value.**

use super::*;

/// The furthest a pose may move in one tick and still be swept rather than
/// drawn where it lands, in world units. Chosen, not measured: about ten times
/// a craft's top speed per tick, so it never cuts real motion short, and well
/// under the distance of any respawn or camera cut.
pub const SNAP_DISTANCE: f32 = 64.0;

/// The previous tick's draw poses and how far a draw is past them.
#[derive(Debug, Clone)]
pub(super) struct Poses {
    /// The poses as they stood before the latest tick, `None` before the
    /// first and after [`Race::settle_poses`].
    previous: Option<TickPose>,
    /// How far between [`Self::previous`] and the current tick a draw is:
    /// `1.0` is the current tick exactly, and is the value outside a draw.
    alpha: f32,
}

impl Default for Poses {
    fn default() -> Self {
        Self {
            previous: None,
            alpha: 1.0,
        }
    }
}

/// One tick's draw poses.
#[derive(Debug, Clone, Copy)]
struct TickPose {
    camera: CameraPose,
    ships: [ShipPose; MAX_SHIPS],
}

/// One tick's camera, as the readers [`Race::view_between`] and
/// [`Race::camera_scalar_between`] pick from.
#[derive(Debug, Clone, Copy)]
pub(super) struct CameraPose {
    /// [`Race::view`]'s tick value, shake included.
    pub(super) view: Mat4,
    /// [`Race::view_unshaken`]'s tick value.
    pub(super) unshaken: Mat4,
    /// The two speed terms [`Race::vertical_fov`] widens the field by.
    pub(super) forward_speed: f32,
    pub(super) boost_kick: f32,
    key: CameraKey,
}

/// What has to hold across a tick for the camera to be swept: the same shot,
/// the same view, the same imposed pose or none.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CameraKey {
    cuts: u32,
    view: oag_display::display::CameraView,
    overridden: bool,
}

#[derive(Debug, Clone, Copy)]
struct ShipPose {
    model: Mat4,
    key: ShipKey,
}

/// What has to hold across a tick for a craft to be swept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShipKey {
    respawns: u32,
    active: bool,
}

/// The alpha the window draws a race at: how far `clock` is into the next tick
/// while the race steps on every tick, and the current tick exactly while it
/// does not (paused, or parked behind the menus), where a blend between two
/// frozen ticks would rock a still picture back and forth.
#[must_use]
pub fn frame_alpha(clock: &TickClock, stepping: bool) -> f32 {
    if stepping {
        clock.interpolation_alpha()
    } else {
        1.0
    }
}

/// `previous` blended toward `current` by `alpha`: translation and scale
/// lerped, rotation slerped along the shorter arc. `current` itself, untouched,
/// at `alpha >= 1.0` and when the translation jumps past [`SNAP_DISTANCE`].
#[must_use]
pub fn blend_transform(previous: Mat4, current: Mat4, alpha: f32) -> Mat4 {
    between(previous, current, alpha).unwrap_or(current)
}

/// Two view matrices blended as the cameras they are the inverse of, so the
/// eye moves in a straight line and turns about itself. `current` itself,
/// untouched, wherever [`blend_transform`] would return its own.
#[must_use]
pub fn blend_view(previous: Mat4, current: Mat4, alpha: f32) -> Mat4 {
    between(previous.inverse(), current.inverse(), alpha).map_or(current, |camera| camera.inverse())
}

/// The blend, or `None` where the current value is to be drawn as it is.
fn between(previous: Mat4, current: Mat4, alpha: f32) -> Option<Mat4> {
    if alpha >= 1.0 || alpha.is_nan() {
        return None;
    }
    let (s0, r0, t0) = previous.to_scale_rotation_translation();
    let (s1, r1, t1) = current.to_scale_rotation_translation();
    if !t0.is_finite() || !t1.is_finite() || t0.distance(t1) > SNAP_DISTANCE {
        return None;
    }
    let alpha = alpha.max(0.0);
    Some(Mat4::from_scale_rotation_translation(
        s0.lerp(s1, alpha),
        r0.normalize().slerp(r1.normalize(), alpha),
        t0.lerp(t1, alpha),
    ))
}

impl Race {
    /// Sets how far between the previous tick and this one the next reads of
    /// [`Self::view`], [`Self::vertical_fov`] and [`Self::ship_model_matrix_of`]
    /// are, `0.0..=1.0`; `1.0` (and NaN) is this tick exactly. The window sets
    /// it for its draw and puts `1.0` back after; every tick path puts `1.0`
    /// back on its own too, so nothing the tick computes ever reads a blend.
    pub fn set_draw_alpha(&mut self, alpha: f32) {
        self.view.poses.alpha = if alpha.is_nan() {
            1.0
        } else {
            alpha.clamp(0.0, 1.0)
        };
    }

    /// The alpha [`Self::set_draw_alpha`] last set, `1.0` outside a draw.
    #[must_use]
    pub fn draw_alpha(&self) -> f32 {
        self.view.poses.alpha
    }

    /// Forgets the previous tick, so draws show the current one exactly until
    /// the next tick runs. For a race taken back out from behind the menus,
    /// whose last tick is long past.
    pub fn settle_poses(&mut self) {
        self.view.poses.previous = None;
    }

    /// Keeps this tick's draw poses as the previous ones, before the tick
    /// moves them: the first thing every tick path does.
    pub(super) fn capture_poses(&mut self) {
        self.view.poses.alpha = 1.0;
        let pose = TickPose {
            camera: CameraPose {
                view: self.tick_view(),
                unshaken: self.tick_view_unshaken(),
                forward_speed: self.tick_forward_speed(),
                boost_kick: self.view.boost_kick,
                key: self.camera_key(),
            },
            ships: std::array::from_fn(|slot| ShipPose {
                model: model_matrix_of(&self.sim.world.ships[slot]),
                key: self.ship_key(slot),
            }),
        };
        self.view.poses.previous = Some(pose);
    }

    fn camera_key(&self) -> CameraKey {
        CameraKey {
            cuts: self.camera_cuts(),
            view: self.view.camera_view,
            overridden: self.view.camera_override.is_some(),
        }
    }

    fn ship_key(&self, slot: usize) -> ShipKey {
        ShipKey {
            respawns: self.respawns_of(slot),
            active: self.sim.world.ships[slot].active,
        }
    }

    /// The previous tick's camera and the alpha to blend it by, `None` where
    /// this read is the current tick exactly.
    fn camera_between(&self) -> Option<(&CameraPose, f32)> {
        let poses = &self.view.poses;
        let previous = poses.previous.as_ref()?;
        (poses.alpha < 1.0 && previous.camera.key == self.camera_key())
            .then_some((&previous.camera, poses.alpha))
    }

    /// A view matrix at the draw alpha: `current` is this tick's, `previous`
    /// picks the same matrix out of the previous tick's camera.
    pub(super) fn view_between(
        &self,
        current: Mat4,
        previous: impl Fn(&CameraPose) -> Mat4,
    ) -> Mat4 {
        match self.camera_between() {
            Some((camera, alpha)) => blend_view(previous(camera), current, alpha),
            None => current,
        }
    }

    /// One of the camera's speed terms at the draw alpha, on the same terms
    /// as [`Self::view_between`].
    pub(super) fn camera_scalar_between(
        &self,
        current: f32,
        previous: impl Fn(&CameraPose) -> f32,
    ) -> f32 {
        match self.camera_between() {
            Some((camera, alpha)) => {
                let from = previous(camera);
                from + (current - from) * alpha
            }
            None => current,
        }
    }

    /// The craft in `slot`'s model matrix at the draw alpha, `current` being
    /// this tick's.
    pub(super) fn ship_between(&self, slot: usize, current: Mat4) -> Mat4 {
        let poses = &self.view.poses;
        match poses.previous.as_ref().map(|previous| previous.ships[slot]) {
            Some(previous) if poses.alpha < 1.0 && previous.key == self.ship_key(slot) => {
                blend_transform(previous.model, current, poses.alpha)
            }
            _ => current,
        }
    }

    /// The player's speed along their own nose this tick, the term
    /// [`Self::vertical_fov`] widens the field by.
    pub(super) fn tick_forward_speed(&self) -> f32 {
        let body = &self.ship().physics.body;
        body.forward().dot(body.linear_velocity)
    }
}
