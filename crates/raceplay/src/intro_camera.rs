//! The pre-race flyby: the circuit's own camera animation, played before the countdown.
//!
//! **Pulse off a PSP disc, and Wipeout HD with most of its numbers chosen** (see
//! [`oag_title::pre_race`] for which numbers are which): the rules below are Pulse's measured
//! ones, and each title's own are its `PreRace` data. Before a race the
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
//! | the camera is `grid_camera1`'s world pose, vertical field the title's `fov_degrees` | measured, two circuits, to `3e-4` |
//! | the pose is the animation's as of the **previous** tick | measured: against the same tick the median error is `0.74` units, one tick back `5e-5` |
//! | the animation holds its first frame for the title's hold ticks, then plays at the tick rate | measured on three runs (the animation clock first non-zero at tick 29 each time), and read: the camera node waits `1.0` s of its own clock, which runs two `dt` a tick |
//! | the flyby ends when the animation reaches `AnimEnd`, or when [`Button::Cross`] is **held** | read (`GridCamera_Progress`, the `Input_IsHeld(5)` test) and measured: the animation ended at `24.99` s of `25.0`, and a held cross ended it as soon as the lock lifted |
//! | neither can end it before the title's lock ticks have passed | read (`+0x1a04`, a counter from 60) and measured (it reads `57` at the third tick and `0` from the sixtieth) |
//! | the world does not tick meanwhile: the countdown's 272 ticks start when the flyby ends | measured: the race clock reads zero and the mode state `0` throughout, and the first state after the flyby is the one a skip always reached |
//! | the HUD is hidden through the flyby and for the title's HUD delay ticks after it | read (`g_hud+0x2c` flags, cleared at the intro's start and set when its `0.5` s fade-out substate ends) and seen: no HUD at tick 30, HUD at tick 60 |
//!
//! # What is chosen, not measured
//!
//! - **The world sits still at its first tick.** The original *does* tick during the flyby (the
//!   craft hover-settle over about 40 ticks, then sit), and our world holds its placement pose
//!   instead - which keeps every tick count a golden hash or a replay was written against.
//!   Height only: the settled craft is about two units above the placement pose on `16_Track`.
//! - **A cut is every one-frame translation key pair** ([`oag_vex::grid_camera`]).
//! - **The scenery's animation clock is the world's tick**, so it holds still under the
//!   flyby; the original's runs on.
//! - **The panel's leave starts on the tick the flyby ends**; the original's started one or two
//!   ticks after its substate changed. The panel itself is [`oag_ui_screens::track_panel`].
//!
//! # What is not ported
//!
//! The flyby's music, the screen wash the original starts when it ends (`RaceMode_UpdateIntro`
//! calls `ScreenFlash` kind 10, which this port does not draw), the Demo mode's automatic skip,
//! and the unread byte at mode object `+0x40` that also ends a flyby without a held button.

use super::*;

use log::debug;
use oag_core::math::Mat3;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::Button;
use oag_title::pre_race::{Ending, PreRace, Skip};
use oag_vex::grid_camera::GridCamera;

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
    /// The title's hold, lock and ending.
    rule: &'static PreRace,
}

impl Timeline {
    /// A clock for a flyby `length` seconds long under `rule`.
    #[must_use]
    pub fn new(length: f32, rule: &'static PreRace) -> Self {
        Self {
            length,
            tick: 0,
            rule,
        }
    }

    /// The animation's clock at tick `k`, seconds: zero for the hold, then one `dt` a tick,
    /// wrapping at `AnimEnd` for a flyby only a skip ends.
    fn animation_seconds(&self, k: u32, dt: f32) -> f32 {
        let seconds = k.saturating_sub(self.rule.hold_ticks.value) as f32 * dt;
        match self.rule.ending.value {
            Ending::AtAnimationEnd => seconds,
            Ending::OnlyBySkip => seconds % self.length,
        }
    }

    /// Ticks the HUD stays hidden once it is over.
    #[must_use]
    pub fn hud_delay(&self) -> u64 {
        u64::from(self.rule.hud_delay_ticks.value)
    }

    /// Ticks run so far.
    #[must_use]
    pub fn ticks(&self) -> u32 {
        self.tick
    }

    /// One tick: whether to show a pose, and as of when, or that the flyby ends.
    ///
    /// It ends once the lock has passed and the animation has reached its end (where the title
    /// ends that way), or has started and `skip` is true. The pose is the animation's as of the
    /// tick before.
    pub fn step(&mut self, skip: bool, dt: f32) -> Beat {
        let k = self.tick;
        self.tick += 1;
        let seconds = self.animation_seconds(k, dt);
        let ran_out = self.rule.ending.value == Ending::AtAnimationEnd && seconds >= self.length;
        let started = k >= self.rule.hold_ticks.value;
        if k >= self.rule.lock_ticks.value && (ran_out || (started && skip)) {
            return Beat::Over;
        }
        Beat::Show(self.animation_seconds(k.saturating_sub(1), dt))
    }
}

/// The flyby's own state. View-side: nothing here reaches the simulation or its hash.
#[derive(Debug, Clone)]
pub struct IntroCamera {
    grid: GridCamera,
    rule: &'static PreRace,
    /// Whether the skip button was down on the previous tick, for a title that skips on a press.
    held_before: bool,
    phase: Phase,
    timeline: Timeline,
    /// How many times the picture has jumped: each key-pair cut, and the flyby ending.
    cuts: u32,
    /// The animation time shown on the previous tick, so a cut is a step across a key pair.
    last_shown: Option<f32>,
    /// A pointer press asked to skip. It stands in for a held Cross and **stays held until the
    /// lock lifts**: a tap is one tick long and the lock is 60, so an edge alone would miss it.
    skip_requested: bool,
}

impl IntroCamera {
    /// A dormant flyby over `grid`.
    #[must_use]
    pub fn new(grid: GridCamera, rule: &'static PreRace) -> Self {
        let timeline = Timeline::new(grid.length(), rule);
        Self {
            grid,
            rule,
            held_before: true,
            phase: Phase::Dormant,
            timeline,
            cuts: 0,
            last_shown: None,
            skip_requested: false,
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

    /// Ticks the HUD stays hidden after the flyby: the title's own.
    #[must_use]
    pub fn hud_delay(&self) -> u64 {
        self.timeline.hud_delay()
    }

    /// A pointer press: skip as a held Cross would, as soon as the lock allows.
    pub fn request_skip(&mut self) {
        self.skip_requested = true;
    }

    /// Starts it. A no-op once started.
    pub fn begin(&mut self) {
        if self.phase == Phase::Dormant {
            self.phase = Phase::Playing;
            self.timeline = Timeline::new(self.grid.length(), self.rule);
        }
    }

    /// One tick. The pose to draw from, or `None` once the flyby is over - the tick it ends on
    /// already shows the chase camera.
    pub fn step(&mut self, skip_held: bool, dt: f32) -> Option<CameraOverride> {
        if self.phase != Phase::Playing {
            return None;
        }
        let pressed = skip_held && !self.held_before;
        self.held_before = skip_held;
        let skip = match self.rule.skip.value {
            Skip::Held => skip_held,
            Skip::Press => pressed,
        };
        let shown = match self.timeline.step(skip || self.skip_requested, dt) {
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
            fov_deg: Some(self.rule.fov_degrees.value),
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
                let was = intro.playing();
                intro.begin();
                if intro.playing() && !was {
                    debug!("pre-race flyby: begins");
                }
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

    /// A pointer press on the flyby: skips it as a held Cross does, as soon as the lock lifts.
    /// The pointer's half of the panel's input - the panel itself has no button to press.
    pub fn skip_intro(&mut self) {
        if let Some(intro) = &mut self.view.intro {
            intro.request_skip();
        }
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
        let was = intro.playing();
        self.view.camera_override = intro.step(held, dt);
        if was && !intro.playing() {
            debug!(
                "pre-race flyby: ends after {} tick(s){}",
                intro.ticks(),
                if held { ", Cross held" } else { "" }
            );
        }
    }

    /// The key the motion blur's previous-tick snapshot is promoted on: the world's tick plus the
    /// flyby's ticks, so the blur keeps measuring one-tick travel while the world is held.
    ///
    /// Keyed on [`World::tick`] alone it would never promote during the flyby (the world sits at
    /// tick 0), and every frame of it would be blurred against the first.
    #[must_use]
    pub fn motion_tick(&self) -> u64 {
        self.sim.world.tick + self.view.intro.as_ref().map_or(0, |i| u64::from(i.ticks()))
    }

    /// Where the track-description panel is, while it is on screen: up through the flyby, then
    /// fading out as the chase view returns. `None` before the flyby, with none, and once it has
    /// faded. The panel's own timing is [`oag_ui_screens::track_panel`]; the clock is the 60 Hz tick.
    #[must_use]
    pub fn track_panel_progress(&self) -> Option<oag_ui_screens::track_panel::Progress> {
        let intro = self.view.intro.as_ref()?;
        let tick = 1.0 / 60.0;
        let progress = oag_ui_screens::track_panel::Progress {
            entered: intro.ticks() as f32 * tick,
            left: match intro {
                intro if intro.playing() => None,
                intro if intro.finished() => Some(self.sim.world.tick as f32 * tick),
                _ => return None,
            },
        };
        progress.visible().then_some(progress)
    }

    /// Whether the HUD is drawn: not through the flyby, and not for [`IntroCamera::hud_delay`] ticks of
    /// the race after it.
    #[must_use]
    pub fn hud_shown(&self) -> bool {
        match &self.view.intro {
            Some(intro) if intro.playing() => false,
            Some(intro) if intro.finished() => self.sim.world.tick >= intro.hud_delay(),
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests;
