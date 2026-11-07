//! The replay viewer's rules: playing a recorded race back, seeking in it, and what it says when
//! the build no longer matches the recording (ADR-0087, decisions 3, 4 and 6).
//!
//! **Two races.** The race on screen is the ordinary one the loader built, with its scene, HUD
//! and camera; it is only ever moved by [`Viewer`] and drawn as any race is. The other is the
//! [`Timeline`]'s probe, which a worker thread feeds keyframes and which a seek restores from.
//! A seek puts a keyframe's state into the race on screen and runs the 60 ticks before the
//! target through `Race::tick`, so the camera spring, the exhaust trails and the particles have
//! settled by the target, as they have by any tick of a played race. Those ticks raise cues the
//! player must not hear; they are drained, and the caller rebuilds the continuous voices from
//! the target's state (`Audio::rebuild_race_sfx` in `oag-sound`).
//!
//! **Nothing here writes a record or a ghost**, and nothing in a race watched this way can: the
//! race on screen stops recording before it is handed over ([`Viewer::begin`]).
//!
//! What a player sees while the opening pass has not yet cut the first keyframe (0.2 to 1.1 s,
//! the plans a keyframe carries): the grid at tick 0, held, with an empty bar. ADR-0087
//! decision 3's "plays from tick 0 at once" is not literally true, and the bar says so
//! ([`Viewer::opening`]).
//!
//! **Chosen, not measured:** the speeds, the skips and the layout of the screen.

use oag_replay::{Desync, Replay};

use crate::Race;
use crate::replay::apply_recorded;
use crate::timeline::{OpenError, Pass, Timeline};

/// Ticks drawn but silent before a seek's target, so the view has settled there.
pub const PRE_ROLL: u64 = 60;

/// How far the skip buttons jump, in ticks (five seconds).
pub const SKIP: u64 = 300;

/// The playback speeds, as ticks per frame tick in sixteenths.
pub const SPEEDS: [u32; 4] = [8, 16, 32, 64];

/// Where playback stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The opening pass has not yet cut the first keyframe.
    Opening,
    /// Playing, or paused by the player.
    Ready,
    /// At the end of a recording that reproduced.
    Ended,
    /// At the last tick the build reproduces, in a recording it does not match past there.
    Stopped,
}

/// What a frame tick of the viewer did, for the caller's sound and screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moved {
    /// Nothing moved.
    Still,
    /// The race stepped `ticks` drawn ticks forward, at the speed it plays: its cues are the
    /// sound of the race at that speed.
    Stepped(u32),
    /// The playhead jumped to a tick: every voice is rebuilt from the state there.
    Seeked,
}

/// A recording played back.
#[derive(Debug)]
pub struct Viewer {
    replay: Replay,
    timeline: Box<Timeline>,
    /// The tick the race on screen stands at.
    at: u64,
    playing: bool,
    speed: usize,
    /// Sixteenths of a tick owed at the current speed.
    owed: u32,
    started: bool,
}

impl Viewer {
    /// Opens `replay` over the race `setup` starts and begins the opening pass.
    ///
    /// # Errors
    ///
    /// [`OpenError`]: the race cannot be started from the header, or it is not the recorded one
    /// (a tick-zero mismatch).
    pub fn open(setup: crate::Setup, replay: Replay) -> Result<Self, OpenError> {
        let timeline = Box::new(Timeline::open(setup, replay.clone())?);
        Ok(Self {
            replay,
            timeline,
            at: 0,
            playing: true,
            speed: 1,
            owed: 0,
            started: false,
        })
    }

    /// Makes the race on screen the recorded race at tick 0: the launch settings and seats the
    /// header names, no recording and no ghost. Called once, before the first frame.
    ///
    /// # Errors
    ///
    /// A launch setting this build does not know.
    pub fn begin(&self, race: &mut Race) -> anyhow::Result<()> {
        apply_recorded(race, &self.replay.header)?;
        race.stop_recording();
        Ok(())
    }

    /// The recording's length in ticks.
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.replay.ticks()
    }

    /// The tick the race on screen stands at.
    #[must_use]
    pub fn at(&self) -> u64 {
        self.at
    }

    /// Whether playback is running (not paused by the player).
    #[must_use]
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// The speed in sixteenths of real time.
    #[must_use]
    pub fn speed(&self) -> u32 {
        SPEEDS[self.speed]
    }

    /// The header the recording carries.
    #[must_use]
    pub fn header(&self) -> &oag_replay::Header {
        &self.replay.header
    }

    /// The last tick that may be played: what the pass has built so far, which is all of the
    /// recording once it ends reproduced.
    pub fn built(&mut self) -> Option<u64> {
        self.timeline.prefix()
    }

    /// How the opening pass stands.
    pub fn pass(&mut self) -> Pass {
        self.timeline.pass()
    }

    /// The hash that stopped the pass, when the build does not reproduce the recording.
    pub fn mismatch(&mut self) -> Option<Desync> {
        match self.timeline.pass() {
            Pass::Desynced(desync) => Some(desync),
            _ => None,
        }
    }

    /// Whether the pass ended because this build stopped matching the recording: a stored hash
    /// disagreed, or an event of a kind a later build wrote came up. Either way the viewer plays
    /// what agrees and says so.
    pub fn stops_matching(&mut self) -> bool {
        matches!(self.timeline.pass(), Pass::Desynced(_) | Pass::Refused(_))
    }

    /// Whether the first keyframe is not yet in: the screen holds the grid and says so.
    pub fn opening(&mut self) -> bool {
        self.timeline.prefix().is_none()
    }

    /// Where playback stands.
    pub fn state(&mut self) -> State {
        let Some(built) = self.timeline.prefix() else {
            return State::Opening;
        };
        match self.timeline.pass() {
            Pass::Desynced(_) | Pass::Refused(_) if self.at >= built => State::Stopped,
            Pass::Complete if self.at >= self.ticks() => State::Ended,
            _ => State::Ready,
        }
    }

    /// Plays or pauses; playing from the end plays from the start.
    pub fn toggle(&mut self, race: &mut Race) -> Moved {
        if self.playing {
            self.playing = false;
            return Moved::Still;
        }
        self.playing = true;
        if matches!(self.state(), State::Ended | State::Stopped) {
            return self.seek(race, 0);
        }
        Moved::Still
    }

    /// The next speed, wrapping.
    pub fn faster(&mut self) {
        self.speed = (self.speed + 1) % SPEEDS.len();
        self.owed = 0;
    }

    /// One frame tick: starts playback when the first keyframe lands, and steps the race at the
    /// speed it plays.
    pub fn update(&mut self, race: &mut Race) -> Moved {
        if !self.started {
            if self.timeline.prefix().is_none() {
                return Moved::Still;
            }
            self.started = true;
            return self.seek(race, 0);
        }
        if !self.playing {
            return Moved::Still;
        }
        self.owed += SPEEDS[self.speed];
        let mut stepped = 0;
        while self.owed >= 16 {
            self.owed -= 16;
            if !self.step(race) {
                self.owed = 0;
                break;
            }
            stepped += 1;
        }
        if stepped > 0 {
            Moved::Stepped(stepped)
        } else {
            Moved::Still
        }
    }

    /// One drawn tick forward, if the pass has built it and the recording has it.
    fn step(&mut self, race: &mut Race) -> bool {
        let Some(built) = self.timeline.prefix() else {
            return false;
        };
        if self.at >= built.min(self.ticks()) {
            return false;
        }
        self.advance(race);
        true
    }

    fn advance(&mut self, race: &mut Race) {
        for event in self.replay.events_at(self.at) {
            if race.apply_event(event).is_err() {
                // The pass refuses an event this build does not apply before the prefix reaches
                // it, so a played tick never meets one.
                break;
            }
        }
        race.tick(&self.replay.inputs_at(self.at));
        self.at += 1;
    }

    /// Jumps the playhead to `tick` (held inside what is built): the keyframe at or before the
    /// 60 ticks before it, then those ticks drawn and silent.
    pub fn seek(&mut self, race: &mut Race, tick: u64) -> Moved {
        let Some(built) = self.timeline.prefix() else {
            return Moved::Still;
        };
        let target = tick.min(built).min(self.ticks());
        let from = target.saturating_sub(PRE_ROLL);
        if self.timeline.seek(from).is_err() {
            return Moved::Still;
        }
        race.restore_for_viewing(&self.timeline.probe().sim);
        self.at = self.timeline.tick();
        while self.at < target {
            self.advance(race);
            // The tick being replayed already sounded once, and a seek is silent up to its
            // target.
            drop(race.drain_cues());
            drop(race.drain_announcements());
            drop(race.drain_class_announcements());
        }
        self.owed = 0;
        Moved::Seeked
    }

    /// [`Self::seek`] by `delta` ticks from where the playhead stands.
    pub fn skip(&mut self, race: &mut Race, delta: i64) -> Moved {
        let target = self.at.saturating_add_signed(delta);
        self.seek(race, target)
    }
}
