//! Photo mode's rewind: the race on screen moved through its own past and put back.
//!
//! The decisions are `docs/architecture/adr/0087-a-replay-and-photo-mode-share-one-timeline.md`
//! (decision 5), what was built is `docs/ui/photo-mode.md` ("Accumulated shots and rewind").
//!
//! **The simulation a player is playing is put back to the bit.** Entering takes
//! [`Race::snapshot_sim`] of the live race and sets its recording aside ([`Race::pause_recording`]);
//! leaving restores that snapshot and the recording, so the state hash at every live tick and the
//! race's `Debug` text are what a visit that never rewound leaves. In between, the race on screen
//! is moved the way the replay viewer moves it: the [`Timeline`] probe seeks to a keyframe a
//! pre-roll before the target, that state is put into the race on screen, and the ticks up to the
//! target run through [`Race::tick`], drawn and silent, so the camera spring, the exhaust trails
//! and the particles have settled by the target as they have by any played tick. The ticks are
//! the recorded inputs and events of the race so far.
//!
//! With no recording (a modded race records none) there are no inputs to play, and a seek stops
//! at the whole keyframe at or before the target ([`Timeline::seek`]).
//!
//! **Chosen, not measured:** the pre-roll, the step of the row, and a rewind that is silent
//! (its cues are dropped).

use std::sync::Arc;

use oag_replay::Replay;

use super::*;
use crate::replay::Paused;
use crate::timeline::Keeper;

/// Ticks drawn and silent before a seek's target, so the view has settled there: one second,
/// the viewer's own.
pub const PRE_ROLL: u64 = 60;

/// A race on screen that is being moved through its past.
#[derive(Debug)]
pub struct Rewind {
    /// The live race's simulation, put back by [`Self::end`].
    saved: RaceSim,
    /// The recording the live race was making, put back with it.
    paused: Paused,
    /// The inputs and events that move the race; `None` for a race that records nothing.
    replay: Option<Arc<Replay>>,
    /// The live tick: the furthest forward a seek goes.
    live: u64,
    /// The tick the race on screen stands at.
    at: u64,
    /// Whether the camera is put on the craft again at every tick the race is moved through. A
    /// ship blur holds the camera and turns this off.
    pub follow: bool,
}

impl Rewind {
    /// Starts moving `race`, the live race, through its past. `None` when it cannot be: a race
    /// that has finished is on its end screens and keeps its board.
    pub fn begin(keeper: &mut Keeper, race: &mut Race) -> Option<Self> {
        if race.finished() {
            return None;
        }
        let live = race.sim.world.tick;
        let replay = keeper.visit(race).recording();
        Some(Self {
            saved: race.snapshot_sim(),
            paused: race.pause_recording(),
            replay,
            live,
            at: live,
            follow: true,
        })
    }

    /// The live tick: where leaving puts the race back.
    #[must_use]
    pub fn live(&self) -> u64 {
        self.live
    }

    /// The tick the race on screen stands at.
    #[must_use]
    pub fn at(&self) -> u64 {
        self.at
    }

    /// Whether any tick can be reached, not only a keyframe's: the race has a recording.
    #[must_use]
    pub fn resimulates(&self) -> bool {
        self.replay.is_some()
    }

    /// Moves the race on screen to `tick` (held between the race's start and the live tick) and
    /// returns the tick reached, which with no recording is the keyframe at or before it.
    pub fn seek(&mut self, keeper: &mut Keeper, race: &mut Race, tick: u64) -> u64 {
        let target = tick.min(self.live);
        let from = target.saturating_sub(PRE_ROLL);
        let Some(timeline) = keeper.timeline() else {
            return self.at;
        };
        if timeline.seek(from).is_err() {
            return self.at;
        }
        race.restore_sim(&timeline.probe().sim);
        race.settle_poses();
        self.at = timeline.tick();
        if let Some(replay) = self.replay.clone() {
            while self.at < target {
                if !self.advance(race, &replay) {
                    break;
                }
            }
        }
        if self.follow {
            race.photo_repose();
        }
        self.at
    }

    /// One drawn tick forward, when the race has one and the recording plays it.
    pub fn step(&mut self, race: &mut Race) -> bool {
        match self.replay.clone() {
            Some(replay) if self.at < self.live => self.advance(race, &replay),
            _ => false,
        }
    }

    fn advance(&mut self, race: &mut Race, replay: &Replay) -> bool {
        for event in replay.events_at(self.at) {
            if race.apply_event(event).is_err() {
                return false;
            }
        }
        if self.follow {
            race.photo_repose();
        }
        race.tick(&replay.inputs_at(self.at));
        // The tick being replayed already sounded once, and a rewind is silent.
        drop(race.drain_cues());
        drop(race.drain_announcements());
        drop(race.drain_class_announcements());
        self.at += 1;
        if self.follow {
            race.photo_repose();
        }
        true
    }

    /// Puts the live race back: its simulation to the bit, its recording as it was.
    pub fn end(self, race: &mut Race) {
        race.restore_sim(&self.saved);
        race.settle_poses();
        race.resume_recording(self.paused);
        race.photo_repose();
    }
}
