//! The pre-race flyby: the circuit's own camera animation, played before the countdown.
//!
//! **Pulse off a PSP disc only**, the one executable it is read off. Before a race the
//! original's `RaceMode_UpdateIntro` (`0x08829e6c`) runs a substate machine whose first
//! working substate is a *camera pass*: the render view is the `gridCamera` in the circuit's
//! `start_grid.vex`, and `grid_camera1`'s keyed animation flies it round the circuit for
//! `AnimEnd` seconds (25 on most circuits) behind the track-description panel. Then the
//! ordinary start runs: the same 272 ticks to green that a skipped flyby has always had. The
//! reading and the captures are
//! [`race-intro.md`](../../../../docs/ghidra/functions/psp-pulse-usa/race-intro.md) and
//! [`docs/gameplay/race-intro.md`](../../../../docs/gameplay/race-intro.md); the pose is
//! [`oag_vex::grid_camera`].
//!
//! # What is ported
//!
//! | Rule | Source |
//! | --- | --- |
//! | the camera is `grid_camera1`'s world pose, vertical field [`FOV_DEGREES`] | measured, two circuits, to `3e-4` |
//! | the pose is the animation's as of the **previous** tick | measured: against the same tick the median error is `0.74` units, one tick back `5e-5` |
//! | the animation holds its first frame for [`HOLD_TICKS`] ticks, then plays at the tick rate | measured once (28 ticks), and read: the camera node waits `1.0` s of its own clock, which runs two `dt` a tick |
//! | the flyby ends when the animation reaches `AnimEnd`, or when [`Button::Cross`] is **held** | read (`GridCamera_Progress`, the `Input_IsHeld(5)` test) and measured: the animation ended at `24.99` s of `25.0`, and a held cross ended it as soon as the lock lifted |
//! | neither can end it before [`LOCK_TICKS`] ticks have passed | read (`+0x1a04`, a counter from 60) and measured (it reads `57` at the third tick and `0` from the sixtieth) |
//! | the world does not tick meanwhile: the countdown's 272 ticks start when the flyby ends | measured: the race clock reads zero and the mode state `0` throughout, and the first state after the flyby is the one a skip always reached |
//! | the HUD is hidden through the flyby and for [`HUD_DELAY_TICKS`] ticks after it | read (`g_hud+0x2c` flags, cleared at the intro's start and set when its `0.5` s fade-out substate ends) and seen: no HUD at tick 30, HUD at tick 60 |
//!
//! # What is chosen, not measured
//!
//! - **The world sits still at its first tick.** The original *does* tick during the flyby (the
//!   craft hover-settle over about 40 ticks, then sit), and our world holds its placement pose
//!   instead - which keeps every tick count a golden hash or a replay was written against.
//!   Height only: the settled craft is about two units above the placement pose on `16_Track`.
//! - **A cut is every one-frame translation key pair** ([`oag_vex::grid_camera`]).
//! - The track-description panel that the original draws over the flyby is not drawn here.
//!
//! # What is not ported
//!
//! The flyby's music, the screen wash the original starts when it ends (`RaceMode_UpdateIntro`
//! calls `ScreenFlash` kind 10, which this port does not draw), the Demo mode's automatic skip,
//! and the unread byte at mode object `+0x40` that also ends a flyby without a held button.

use super::*;

use oag_core::math::Mat3;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_vex::grid_camera::{FOV_DEGREES, GridCamera};

/// Ticks before the animation starts moving: the camera node waits `1.0` s of its own clock,
/// which advances two `dt` per tick, so it is read as 30. **Measured once as 28** (the animation's
/// clock read `0.0834` s five ticks after its first non-zero reading, the counter at `57` on the
/// third tick), and 28 is what is used.
pub const HOLD_TICKS: u32 = 28;

/// Ticks before the flyby may end: the intro's frame counter (`mode+0x1a04`) starts at 60 and
/// must read zero. Measured: `57` on the third tick, `0` by the sixtieth.
pub const LOCK_TICKS: u32 = 60;

/// Ticks after the flyby ends that the HUD stays hidden: the fade-out substate (`RaceMode_
/// UpdateIntro`'s substate 2) waits `0.5` s, and the HUD flags are set when it ends.
pub const HUD_DELAY_TICKS: u64 = 30;

/// Where the flyby is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Loaded, not started: the race draws as it always did.
    Dormant,
    /// Playing.
    Playing,
    /// Over, by running out or by a held button.
    Finished,
}

/// What one tick of the flyby's clock says.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Beat {
    /// Still playing: draw the camera as of this animation time, seconds.
    Show(f32),
    /// Over this tick: draw the chase camera.
    Over,
}

/// The flyby's clock, with no camera in it: when the animation starts and when it may end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timeline {
    /// `AnimEnd`, seconds.
    length: f32,
    /// Ticks run.
    tick: u32,
}

impl Timeline {
    /// A clock for a flyby `length` seconds long.
    #[must_use]
    pub fn new(length: f32) -> Self {
        Self { length, tick: 0 }
    }

    /// The animation's clock at tick `k`, seconds: zero for the hold, then one `dt` a tick.
    fn animation_seconds(k: u32, dt: f32) -> f32 {
        k.saturating_sub(HOLD_TICKS) as f32 * dt
    }

    /// Ticks run so far.
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.tick
    }

    /// One tick: whether to show a pose, and as of when, or that the flyby ends.
    ///
    /// It ends once [`LOCK_TICKS`] ticks have passed and the animation has reached its end, or
    /// has started and `skip_held` is true. The pose is the animation's as of the tick before.
    pub fn step(&mut self, skip_held: bool, dt: f32) -> Beat {
        let k = self.tick;
        self.tick += 1;
        let seconds = Self::animation_seconds(k, dt);
        let progress = seconds / self.length;
        if k >= LOCK_TICKS && (progress >= 1.0 || (seconds > 0.0 && skip_held)) {
            return Beat::Over;
        }
        Beat::Show(Self::animation_seconds(k.saturating_sub(1), dt))
    }
}

/// The flyby's own state. View-side: nothing here reaches the simulation or its hash.
#[derive(Debug, Clone)]
pub struct IntroCamera {
    grid: GridCamera,
    phase: Phase,
    timeline: Timeline,
    /// How many times the picture has jumped: each key-pair cut, and the flyby ending.
    cuts: u32,
    /// The animation time shown on the previous tick, so a cut is a step across a key pair.
    last_shown: Option<f32>,
}

impl IntroCamera {
    /// A dormant flyby over `grid`.
    #[must_use]
    pub fn new(grid: GridCamera) -> Self {
        let timeline = Timeline::new(grid.length());
        Self {
            grid,
            phase: Phase::Dormant,
            timeline,
            cuts: 0,
            last_shown: None,
        }
    }

    /// Whether the flyby is on screen.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.phase == Phase::Playing
    }

    /// Whether it has been and gone.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }

    /// How many times the picture has jumped. See [`Race::camera_cuts`].
    #[must_use]
    pub fn cuts(&self) -> u32 {
        self.cuts
    }

    /// Ticks run so far.
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.timeline.ticks()
    }

    /// Starts it. A no-op once started.
    pub fn begin(&mut self) {
        if self.phase == Phase::Dormant {
            self.phase = Phase::Playing;
            self.timeline = Timeline::new(self.grid.length());
        }
    }

    /// One tick. The pose to draw from, or `None` once the flyby is over - the tick it ends on
    /// already shows the chase camera.
    pub fn step(&mut self, skip_held: bool, dt: f32) -> Option<CameraOverride> {
        if self.phase != Phase::Playing {
            return None;
        }
        let shown = match self.timeline.step(skip_held, dt) {
            Beat::Over => {
                self.phase = Phase::Finished;
                self.cuts = self.cuts.wrapping_add(1);
                return None;
            }
            Beat::Show(seconds) => seconds,
        };
        if let Some(last) = self.last_shown
            && self.grid.cuts_between(last, shown)
        {
            self.cuts = self.cuts.wrapping_add(1);
        }
        self.last_shown = Some(shown);
        let pose = self.grid.pose_at(shown);
        let [right, up, back] = pose.rows.map(Vec3::from_array);
        Some(CameraOverride {
            eye: Vec3::from_array(pose.eye),
            orientation: Quat::from_mat3(&Mat3::from_cols(right, up, back)),
            fov_deg: Some(FOV_DEGREES),
        })
    }
}

impl Race {
    /// Starts the pre-race flyby, where the circuit has one and nothing else is imposing a
    /// camera. Returns whether it is now playing.
    ///
    /// **Called by the windowed session and by nothing else**: a headless run, a capture and every
    /// test drive [`Race::tick`] from tick 0 and never see a flyby.
    pub fn begin_intro(&mut self) -> bool {
        match &mut self.view.intro {
            Some(intro) if self.view.camera_override.is_none() => {
                intro.begin();
                intro.playing()
            }
            _ => false,
        }
    }

    /// Whether there is a flyby that has not yet played out: dormant or playing, with no pose
    /// imposed from outside. What the session asks before [`Self::begin_intro`].
    #[must_use]
    pub fn intro_to_play(&self) -> bool {
        self.view.intro.as_ref().is_some_and(|intro| {
            !intro.finished() && (intro.playing() || self.view.camera_override.is_none())
        })
    }

    /// Whether the flyby is still to play or playing - the session steps it instead of the world
    /// while this holds.
    #[must_use]
    pub fn in_intro(&self) -> bool {
        self.view.intro.as_ref().is_some_and(IntroCamera::playing)
    }

    /// One flyby tick, in place of [`Race::tick`]. `skip_held` is whether the player holds
    /// [`Button::Cross`].
    pub fn tick_intro(&mut self, inputs: &PlayerInputs) {
        let held = inputs
            .get(self.sim.world.primary_slot())
            .buttons
            .is_held(Button::Cross);
        let dt = self.sim.dt;
        let Some(intro) = &mut self.view.intro else {
            return;
        };
        self.view.camera_override = intro.step(held, dt);
    }

    /// Whether the HUD is drawn: not through the flyby, and not for [`HUD_DELAY_TICKS`] ticks of
    /// the race after it.
    #[must_use]
    pub fn hud_shown(&self) -> bool {
        match &self.view.intro {
            Some(intro) if intro.playing() => false,
            Some(intro) if intro.finished() => self.sim.world.tick >= HUD_DELAY_TICKS,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests;
