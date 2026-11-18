//! What the tick hands its view work to: the picture's and the recording's half of a [`Race`],
//! borrowed apart from the simulation so a view method can read [`SimRace`] while it writes its
//! own state.
//!
//! [`RaceView`] carries the methods that read the simulation and write only the picture (the
//! effects, the camera, the HUD); [`Stage`] carries the ones that also touch the recording, the
//! presentation journal or whether this tick is drawn. Each takes the simulation it reads as an
//! argument, never the whole race, so the tick can call it at the statement it always ran at.
//!
//! **Cues stay the simulation's.** A view method that raises a cue (a spark's collision sound,
//! the shield's announcer edge) takes this tick's queue as `cues`, lent out of
//! [`RaceSim`](crate::RaceSim) by [`SimRace::lending_cues`] for the call, so the cue lands in
//! the queue at the place it always did (ADR-0018: cues are an output of the tick).

use oag_gameplay::cue::CueEvent;

use crate::catch_up::Journal;
use crate::replay::ReplayState;
use crate::{Race, RaceView, SimRace};

/// The view-side fields of a [`Race`], borrowed together for one call while the tick holds
/// the simulation. Built with [`stage!`].
pub(crate) struct Stage<'a> {
    pub(crate) view: &'a mut RaceView,
    pub(crate) replay: &'a mut ReplayState,
    pub(crate) journal: &'a mut Journal,
    /// [`Race::tick_sim_only`]'s flag, copied: this tick is a replay whose picture and sound
    /// already happened.
    quiet: bool,
}

/// The [`Stage`] of a [`Race`] expression, borrowing only its view-side fields, so it can be
/// built while the simulation is borrowed: `stage!(self).note_blast(&self.core, ..)`.
macro_rules! stage {
    ($race:expr) => {
        $crate::stage::Stage::new(
            &mut $race.view,
            &mut $race.replay,
            &mut $race.journal,
            $race.quiet,
        )
    };
}
pub(crate) use stage;

impl<'a> Stage<'a> {
    pub(crate) fn new(
        view: &'a mut RaceView,
        replay: &'a mut ReplayState,
        journal: &'a mut Journal,
        quiet: bool,
    ) -> Self {
        Self {
            view,
            replay,
            journal,
            quiet,
        }
    }

    /// Whether this tick draws and sounds: [`Race::draws`] for the tick the stage was built in.
    pub(crate) fn draws(&self) -> bool {
        !self.quiet
    }
}

impl SimRace {
    /// Runs a view call that reads the simulation and may raise cues, with this tick's cue
    /// queue lent to it as `cues` (the simulation's own queue is empty for the call). The call
    /// pushes onto the queue's end, so its order is the one a push from inside the tick gave.
    pub(crate) fn lending_cues<R>(
        &mut self,
        call: impl FnOnce(&SimRace, &mut Vec<CueEvent>) -> R,
    ) -> R {
        let mut cues = std::mem::take(&mut self.sim.cues);
        let result = call(self, &mut cues);
        self.sim.cues = cues;
        result
    }
}

impl Race {
    /// The picture's half of the race, for a reader outside this crate. Its methods take the
    /// simulation they read: `race.view_state().camera_position(&race.core)`.
    #[must_use]
    pub fn view_state(&self) -> &RaceView {
        &self.view
    }
}
