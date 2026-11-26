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
//! the speed terms of [`crate::RaceView::vertical_fov`], and [`crate::RaceView::ship_model_matrix_of`]
//! (the hull, its shield, flame, plume, shadow map, exhaust nozzle and HD
//! engine lights), and [`crate::RaceView::body_between`] for what reads the body (the
//! blob shadow's ray). Then what
//! moves on its own: every projectile's body and billboard
//! ([`Race::projectile_between`]), the replay ghost
//! ([`Race::ghost_draw_matrix`]), each engine trail ([`Race::trail_head_of`]:
//! the nozzle as drawn, the older samples at fractional ages), and the
//! animation clock texture scroll and `Anim Transform` nodes ride
//! ([`Race::clock_seconds`]). What the originals step - HUD numbers, sprite
//! frames, particle spawns - stays stepped. Particles move on the GPU
//! (`oag_fx::psys`'s `between_ticks`).
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
//! that is not motion: for the camera a cut ([`crate::RaceView::camera_cuts`]: a respawn,
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
//! - **The shadow hulls lerp between two ticks' projections** instead of
//!   projecting from the blended pose, so a gather costs once a tick and not once
//!   a frame (`scene/frame/hull_cache.rs`). Within a world unit's few hundredths
//!   of the live gather on a real lap; see `docs/rendering/split-screen.md`.
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
    /// Each projectile slot's pose on the same tick, `None` for a free slot.
    /// Beside [`Self::previous`] rather than in it so the 128 slots are one
    /// buffer reused tick after tick.
    projectiles: Vec<Option<ProjectilePose>>,
    /// How far between [`Self::previous`] and the current tick a draw is:
    /// `1.0` is the current tick exactly, and is the value outside a draw.
    alpha: f32,
    /// Which tick path last moved the poses, and which one before it: a draw-side
    /// cache of anything gathered from a tick's poses keys on these. See
    /// [`Race::pose_generations`].
    generations: PoseGenerations,
}

/// Names the tick path that produced the poses a draw reads, and the one before.
///
/// A number from [`next_generation`], so it is unique across every race in the
/// process and a cache that outlives a race restart cannot mistake the new race's
/// poses for the old one's. It moves on every tick path ([`Race::tick`], the intro
/// and cosmetic ticks) and on [`Race::settle_poses`], which is not the same thing
/// as [`World::tick`]: the flyby and the cosmetic ticks hold that still.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoseGenerations {
    /// The poses a draw at alpha `1.0` shows.
    pub current: u64,
    /// The poses [`Race::set_draw_alpha`] blends from, `0` where there are none
    /// (before the first tick path and after [`Race::settle_poses`]).
    pub previous: u64,
}

fn next_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl Default for Poses {
    fn default() -> Self {
        Self {
            previous: None,
            projectiles: Vec::new(),
            alpha: 1.0,
            generations: PoseGenerations {
                current: next_generation(),
                previous: 0,
            },
        }
    }
}

/// One tick's draw poses.
#[derive(Debug, Clone, Copy)]
struct TickPose {
    /// [`World::tick`] as it stood, so a reader keyed on it can tell whether
    /// the latest tick path stepped the world (the flyby and the cosmetic
    /// ticks do not).
    world_tick: u64,
    camera: CameraPose,
    ships: [ShipPose; MAX_SHIPS],
}

/// One projectile slot's draw pose, and what says it is still the same shot
/// a tick later: a slot freed and refilled inside one tick holds a fresh
/// lifetime and a different owner or kind as often as not.
#[derive(Debug, Clone, Copy)]
struct ProjectilePose {
    kind: oag_tables::weapons::Weapon,
    owner: u8,
    lifetime: f32,
    age: f32,
    matrix: Mat4,
}

impl ProjectilePose {
    fn same_shot(&self, now: &oag_weapons::projectile::Projectile) -> bool {
        now.kind == Some(self.kind)
            && now.owner == self.owner
            && now.lifetime <= self.lifetime
            && now.age >= self.age
    }
}

/// One tick's camera, as the readers [`crate::RaceView::view_between`] and
/// [`crate::RaceView::camera_scalar_between`] pick from.
#[derive(Debug, Clone, Copy)]
pub(super) struct CameraPose {
    /// [`Race::view`]'s tick value, shake included.
    pub(super) view: Mat4,
    /// [`Race::view_unshaken`]'s tick value.
    pub(super) unshaken: Mat4,
    /// The two speed terms [`crate::RaceView::vertical_fov`] widens the field by.
    pub(super) forward_speed: f32,
    pub(super) boost_kick: f32,
    /// A module's `fov-offset` for the player's view, degrees.
    pub(super) fov_offset: f32,
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
    /// The body's own pose, for what reads the body rather than the drawn
    /// hull (the blob shadow's ray): the model matrix carries the barrel roll
    /// and the model's yaw, the body neither.
    position: Vec3,
    orientation: Quat,
    /// The per-tick scalars drawn on the hull: the airbrake flaps' levels and
    /// the shield shell's swell.
    flaps: [f32; 2],
    shield_swell: f32,
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

impl RaceView {
    /// `slot`'s drawn flaps at `levels` on this tick and on the previous one, so a draw between
    /// ticks shows them as given.
    pub(super) fn put_flaps(&mut self, slot: usize, levels: [f32; 2]) {
        self.flaps[slot] = levels;
        if let Some(previous) = self.poses.previous.as_mut() {
            previous.ships[slot].flaps = levels;
        }
    }
}

impl Race {
    /// Sets how far between the previous tick and this one the next reads of
    /// [`Self::view`], [`crate::RaceView::vertical_fov`] and [`crate::RaceView::ship_model_matrix_of`]
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

    /// Forgets the previous tick, so draws show the current one exactly until
    /// the next tick runs. For a race taken back out from behind the menus,
    /// whose last tick is long past.
    pub fn settle_poses(&mut self) {
        self.view.hold_particles_still();
        self.view.poses.previous = None;
        self.view.poses.projectiles.clear();
        self.view.poses.generations = PoseGenerations {
            current: next_generation(),
            previous: 0,
        };
    }

    /// Which tick path the next draw's poses come from, and which the one it
    /// blends from. Changes exactly when a tick path or [`Self::settle_poses`]
    /// runs, and never during a draw.
    #[must_use]
    pub fn pose_generations(&self) -> PoseGenerations {
        self.view.poses.generations
    }

    /// The craft in `slot`'s model matrix at this tick exactly, whatever the draw
    /// alpha is: what [`crate::RaceView::ship_model_matrix_of`] returns at `1.0`.
    #[must_use]
    pub fn ship_tick_model(&self, slot: usize) -> Mat4 {
        self.view.drawn_model(&self.core, slot)
    }

    /// The craft in `slot`'s model matrix on the previous tick, when a draw
    /// between ticks would sweep from it to this one: `None` where the draw shows
    /// this tick as it is (no previous tick, a respawn or a change of whether it
    /// is racing, or a move past [`SNAP_DISTANCE`] in the model or the body).
    /// The same cases [`crate::RaceView::ship_model_matrix_of`] and [`crate::RaceView::body_between`]
    /// skip a blend on.
    #[must_use]
    pub fn ship_swept_from(&self, slot: usize) -> Option<Mat4> {
        let previous = self.view.poses.previous.as_ref()?;
        let from = previous.ships[slot];
        let moved =
            |a: Vec3, b: Vec3| !a.is_finite() || !b.is_finite() || a.distance(b) > SNAP_DISTANCE;
        let model = self.view.drawn_model(&self.core, slot);
        (from.key == self.core.sim.ship_key(slot)
            && !moved(from.model.w_axis.truncate(), model.w_axis.truncate())
            && !moved(from.position, self.view.drawn_position(&self.core, slot)))
        .then_some(from.model)
    }
}

impl RaceSim {
    fn ship_key(&self, slot: usize) -> ShipKey {
        ShipKey {
            respawns: self.placements_of(slot),
            // Out of the world in a transit: not drawn, and a change either way is a cut.
            active: self.world.ships[slot].in_world(),
        }
    }
}

impl Race {
    /// Whether the latest tick cut the camera: the shot, the view or the imposed pose changed
    /// from the tick before, which is exactly when [`crate::RaceView::camera_between`] draws the current
    /// tick as it is. Whatever the draw alpha, so a draw at `1.0` reports it too.
    #[must_use]
    pub fn camera_cut(&self) -> bool {
        self.view
            .poses
            .previous
            .as_ref()
            .is_some_and(|previous| previous.camera.key != self.view.camera_key(&self.core))
    }

    /// [`Self::camera_cut`] for the view being drawn: the player's own, or an extra seat's chase
    /// camera, which jumps when that craft is placed anew (a respawn, a jump, a transit's exit)
    /// and for nothing else of the player's. The transit's entrance moves no seat's camera.
    #[must_use]
    pub fn viewed_camera_cut(&self) -> bool {
        let Some(slot) = self.view.extra_viewpoint() else {
            return self.camera_cut();
        };
        self.view.poses.previous.as_ref().is_some_and(|previous| {
            previous.ships[slot].key.respawns != self.core.sim.placements_of(slot)
        })
    }

    /// The alpha to blend by when the latest tick path stepped the world by
    /// exactly one tick, `None` otherwise - a flyby or cosmetic tick holds the
    /// world still, and a reader keyed on [`World::tick`] must hold with it.
    fn world_alpha(&self) -> Option<f32> {
        let poses = &self.view.poses;
        let previous = poses.previous.as_ref()?;
        (poses.alpha < 1.0 && previous.world_tick + 1 == self.core.sim.world.tick)
            .then_some(poses.alpha)
    }

    /// Where the newest sample of the engine trail in `slot` is drawn: the
    /// nozzle as this draw places the hull, while that is between ticks, and
    /// `None` when the hull is drawn at this tick and the recorded sample is
    /// already where the nozzle is. See `oag_fx::exhaust::Exhaust::extend_trail_vertices`.
    ///
    /// Its alpha reads the older samples at fractional ages, and is `1.0` (the
    /// ring as recorded) unless the last tick path stepped the world, the one
    /// path that pushes a trail sample: a flyby or cosmetic tick pushes none.
    #[must_use]
    pub fn trail_head_of(&self, slot: usize) -> Option<oag_fx::exhaust::TrailHead> {
        self.view.ship_alpha(&self.core, slot)?;
        Some(oag_fx::exhaust::TrailHead {
            position: self.view.nozzle_of(&self.core, slot)?,
            alpha: self.world_alpha().unwrap_or(1.0),
        })
    }

    /// Puts the craft in `slot`'s drawn airbrake flaps at `levels` (`0..=100`, left then right),
    /// on this tick and on the previous one, so a draw between ticks shows them as given.
    /// For a still posed at a captured craft's own flap values (`--pose-flaps`), which runs
    /// no tick that would swing them there.
    pub fn force_flaps(&mut self, slot: usize, levels: [f32; 2]) {
        self.view
            .put_flaps(slot, levels.map(|level| level.clamp(0.0, 100.0)));
    }

    /// Keeps every craft's flaps stowed through the grid state, from this call on.
    ///
    /// A **capture-only** switch: in play the grid always holds the flaps out. A comparison
    /// capture uses it for a craft posed from a row read after the green light, which stands for
    /// a craft long off the grid, and `--no-grid-flaps` for a frame of the flaps alone. A title
    /// whose original draws them stowed on the grid would use it too; none measured does.
    pub fn stow_grid_flaps(&mut self) {
        self.view.grid_flaps = false;
        for slot in 0..MAX_SHIPS {
            self.force_flaps(slot, [0.0; 2]);
        }
    }

    /// The craft in `slot`'s airbrake flap levels (`0..=100`) at the draw alpha:
    /// the flaps swing between ticks with the hull they hang on.
    pub(super) fn flaps_between(&self, slot: usize) -> [f32; 2] {
        let current = self.view.flaps[slot];
        match (
            self.view.ship_alpha(&self.core, slot),
            &self.view.poses.previous,
        ) {
            (Some(alpha), Some(previous)) => {
                let from = previous.ships[slot].flaps;
                [0, 1].map(|i| from[i] + (current[i] - from[i]) * alpha)
            }
            _ => current,
        }
    }

    /// The scale the craft in `slot`'s shield shell is drawn at: the swell at
    /// the draw alpha plus this tick's flicker, which stays stepped (the
    /// original's flicker is a per-frame random draw, discrete).
    #[must_use]
    pub fn shield_scale_of(&self, slot: usize) -> f32 {
        let state = &self.view.shield[slot];
        match (
            self.view.ship_alpha(&self.core, slot),
            &self.view.poses.previous,
        ) {
            (Some(alpha), Some(previous)) => {
                let from = previous.ships[slot].shield_swell;
                state.scale() - (state.swell() - from) * (1.0 - alpha)
            }
            _ => state.scale(),
        }
    }

    /// One live projectile's model matrix at the draw alpha, `current` being
    /// this tick's. Drawn at this tick when the slot held nothing, or a
    /// different shot, a tick ago.
    pub(super) fn projectile_between(
        &self,
        slot: usize,
        projectile: &oag_weapons::projectile::Projectile,
        current: Mat4,
    ) -> Mat4 {
        match self.projectile_previous(slot, projectile) {
            Some((previous, alpha)) => blend_transform(previous.matrix, current, alpha),
            None => current,
        }
    }

    /// One live projectile's position at the draw alpha, for the billboard
    /// drawn where a projectile has no model.
    pub(super) fn projectile_position_between(
        &self,
        slot: usize,
        projectile: &oag_weapons::projectile::Projectile,
    ) -> Vec3 {
        match self.projectile_previous(slot, projectile) {
            Some((previous, alpha))
                if previous
                    .matrix
                    .w_axis
                    .truncate()
                    .distance(projectile.position)
                    <= SNAP_DISTANCE =>
            {
                previous
                    .matrix
                    .w_axis
                    .truncate()
                    .lerp(projectile.position, alpha)
            }
            _ => projectile.position,
        }
    }

    fn projectile_previous(
        &self,
        slot: usize,
        projectile: &oag_weapons::projectile::Projectile,
    ) -> Option<(ProjectilePose, f32)> {
        let poses = &self.view.poses;
        poses.previous.as_ref()?;
        let previous = poses.projectiles.get(slot).copied().flatten()?;
        (poses.alpha < 1.0 && previous.same_shot(projectile)).then_some((previous, poses.alpha))
    }

    /// The replay ghost's model matrix at the draw alpha: its pose a tick ago
    /// blended toward `now`, while the world is stepping.
    #[must_use]
    pub fn ghost_draw_matrix(&self, now: &oag_replay::Pose, before: &oag_replay::Pose) -> Mat4 {
        let current = RaceSim::ghost_model_matrix(now);
        match self.world_alpha() {
            Some(alpha) => blend_transform(RaceSim::ghost_model_matrix(before), current, alpha),
            None => current,
        }
    }

    /// The race clock in seconds, at the draw alpha: what texture scroll,
    /// `Anim Transform` nodes and the flame surface ride. Exactly
    /// `World::tick / 60` outside a draw and whenever the latest tick path did
    /// not step the world.
    #[must_use]
    pub fn clock_seconds(&self) -> f32 {
        let tick = self.core.sim.world.tick as f32;
        match self.world_alpha() {
            Some(alpha) => (tick - (1.0 - alpha)) / 60.0,
            None => tick / 60.0,
        }
    }

    /// The seconds the pre-race flyby has run: zero with none, frozen at its length once it is
    /// over. The world is held at tick 0 under it, but the original's scenery clock runs through
    /// it (`docs/gameplay/race-intro.md`), so the world meshes add this to the race clock. The
    /// gantry's clock does not: the original starts it at the flyby's end, so it reads
    /// [`Self::clock_seconds`]. Steps at the 60 Hz tick (no draw alpha of its own).
    #[must_use]
    pub fn flyby_seconds(&self) -> f32 {
        self.view
            .intro
            .as_ref()
            .map_or(0.0, |intro| intro.ticks() as f32 / 60.0)
    }

    /// The clock the world meshes' texture and node animation run on: [`Self::clock_seconds`] plus
    /// [`Self::flyby_seconds`] plus the capture's [`crate::Options::world_clock_offset`] (zero in
    /// every race).
    #[must_use]
    pub fn world_clock_seconds(&self) -> f32 {
        self.clock_seconds() + self.flyby_seconds() + self.view.world_clock_offset
    }

    /// The gantry time a capture imposes, [`crate::Options::gantry_seconds`] (`None` in every race).
    #[must_use]
    pub fn gantry_seconds(&self) -> Option<f32> {
        self.view.gantry_seconds
    }

    /// The flame clock a capture imposes, [`crate::Options::flame_seconds`] (`None` in every race).
    #[must_use]
    pub fn flame_seconds(&self) -> Option<f32> {
        self.view.flame_seconds
    }

    /// The capture's [`crate::Options::world_clock_offset`], which only the world meshes' clock carries.
    #[must_use]
    pub fn world_clock_offset(&self) -> f32 {
        self.view.world_clock_offset
    }
}

impl SimRace {
    /// The player's speed along their own nose this tick, the term
    /// [`crate::RaceView::vertical_fov`] widens the field by.
    pub(super) fn tick_forward_speed(&self) -> f32 {
        let body = &self.ship().physics.body;
        body.forward().dot(body.linear_velocity)
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// The alpha [`crate::Race::set_draw_alpha`] last set, `1.0` outside a draw.
    #[must_use]
    pub fn draw_alpha(&self) -> f32 {
        self.poses.alpha
    }

    fn camera_key(&self, core: &SimRace) -> CameraKey {
        CameraKey {
            cuts: self.camera_cuts(core),
            view: self.camera_view,
            overridden: self.camera_override.is_some(),
        }
    }

    /// The previous tick's camera and the alpha to blend it by, `None` where
    /// this read is the current tick exactly.
    fn camera_between(&self, core: &SimRace) -> Option<(&CameraPose, f32)> {
        let poses = &self.poses;
        let previous = poses.previous.as_ref()?;
        (poses.alpha < 1.0 && previous.camera.key == self.camera_key(core))
            .then_some((&previous.camera, poses.alpha))
    }

    /// A view matrix at the draw alpha: `current` is this tick's, `previous`
    /// picks the same matrix out of the previous tick's camera.
    pub(super) fn view_between(
        &self,
        core: &SimRace,
        current: Mat4,
        previous: impl Fn(&CameraPose) -> Mat4,
    ) -> Mat4 {
        match self.camera_between(core) {
            Some((camera, alpha)) => blend_view(previous(camera), current, alpha),
            None => current,
        }
    }

    /// One of the camera's speed terms at the draw alpha, on the same terms
    /// as [`Self::view_between`].
    pub(super) fn camera_scalar_between(
        &self,
        core: &SimRace,
        current: f32,
        previous: impl Fn(&CameraPose) -> f32,
    ) -> f32 {
        match self.camera_between(core) {
            Some((camera, alpha)) => {
                let from = previous(camera);
                from + (current - from) * alpha
            }
            None => current,
        }
    }

    /// The craft in `slot`'s model matrix at the draw alpha, `current` being
    /// this tick's.
    pub(super) fn ship_between(&self, core: &SimRace, slot: usize, current: Mat4) -> Mat4 {
        match (self.ship_alpha(core, slot), &self.poses.previous) {
            (Some(alpha), Some(previous)) => {
                blend_transform(previous.ships[slot].model, current, alpha)
            }
            _ => current,
        }
    }

    /// The alpha to blend by when the craft in `slot` is swept this draw,
    /// `None` when it is drawn at this tick.
    fn ship_alpha(&self, core: &SimRace, slot: usize) -> Option<f32> {
        let poses = &self.poses;
        let previous = poses.previous.as_ref()?;
        (poses.alpha < 1.0 && previous.ships[slot].key == core.sim.ship_key(slot))
            .then_some(poses.alpha)
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// The craft in `slot`'s body at this tick exactly, whatever the draw alpha
    /// is: what [`Self::body_between`] returns at `1.0`.
    #[must_use]
    pub fn ship_tick_body(&self, core: &SimRace, slot: usize) -> oag_physics::Body {
        let mut body = core.sim.world.ships[slot].physics.body;
        body.position = self.drawn_position(core, slot);
        body
    }

    /// Keeps this tick's draw poses as the previous ones, before the tick
    /// moves them: the first thing every tick path does.
    pub(super) fn capture_poses(&mut self, core: &SimRace) {
        self.poses.alpha = 1.0;
        self.hold_particles_still();
        self.poses.generations = PoseGenerations {
            current: next_generation(),
            previous: self.poses.generations.current,
        };
        let pose = TickPose {
            world_tick: core.sim.world.tick,
            camera: CameraPose {
                view: self.tick_view(core),
                unshaken: self.tick_view_unshaken(core),
                forward_speed: core.tick_forward_speed(),
                boost_kick: self.boost_kick,
                fov_offset: core.sim.extension_fov_offset(core.player_slot()),
                key: self.camera_key(core),
            },
            ships: std::array::from_fn(|slot| {
                let ship = &core.sim.world.ships[slot];
                ShipPose {
                    model: self.drawn_model(core, slot),
                    position: self.drawn_position(core, slot),
                    orientation: ship.physics.body.orientation,
                    flaps: self.flaps[slot],
                    shield_swell: self.shield[slot].swell(),
                    key: core.sim.ship_key(slot),
                }
            }),
        };
        self.poses.previous = Some(pose);
        // After the capture, so the previous pose is the one last drawn and the
        // tick's own draws start from the offset one step further out.
        self.decay_corrections();
        let mut projectiles = std::mem::take(&mut self.poses.projectiles);
        projectiles.clear();
        projectiles.extend(core.sim.world.projectiles.slots.iter().enumerate().map(
            |(slot, projectile)| {
                let kind = projectile.kind?;
                Some(ProjectilePose {
                    kind,
                    owner: projectile.owner,
                    lifetime: projectile.lifetime,
                    age: projectile.age,
                    matrix: self.projectile_matrix(kind, slot, projectile),
                })
            },
        ));
        self.poses.projectiles = projectiles;
    }

    /// The craft in `slot`'s body at the draw alpha, for a draw that reads the
    /// body rather than the hull's matrix: its position lerped and orientation
    /// slerped from a tick ago, on the same snaps as
    /// [`crate::RaceView::ship_model_matrix_of`]. The tick's body itself, untouched,
    /// outside a draw.
    pub(super) fn body_between(&self, core: &SimRace, slot: usize) -> oag_physics::Body {
        let mut body = self.ship_tick_body(core, slot);
        if let (Some(alpha), Some(previous)) = (self.ship_alpha(core, slot), &self.poses.previous) {
            let from = previous.ships[slot];
            if from.position.distance(body.position) <= SNAP_DISTANCE {
                body.position = from.position.lerp(body.position, alpha);
                body.orientation = from
                    .orientation
                    .normalize()
                    .slerp(body.orientation.normalize(), alpha);
            }
        }
        body
    }
}
