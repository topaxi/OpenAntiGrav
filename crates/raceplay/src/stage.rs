//! What the tick hands its view work to: the picture's and the recording's half of a [`Race`],
//! borrowed apart from the simulation so a view method can read [`SimRace`](crate::SimRace)
//! while it writes its own state.
//!
//! [`RaceView`] carries the methods that read the simulation and write only the picture (the
//! effects, the camera, the HUD). Each takes the simulation it reads as an argument, never the
//! whole race, so the tick can call it at the statement it always ran at.

use crate::{Race, RaceView};

impl Race {
    /// The picture's half of the race, for a reader outside this crate. Its methods take the
    /// simulation they read: `race.view_state().camera_position(&race.core)`.
    #[must_use]
    pub fn view_state(&self) -> &RaceView {
        &self.view
    }
}
