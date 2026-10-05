//! The race side of HD's authored mix: which state the groups chase.
//!
//! The groups themselves, their law and their loader are in
//! [`crate::audio::hd_mix`]. This is the part that needs the race: it hands
//! the mix to `Audio` from the race's banks when a race starts without a front
//! end behind it (`--race`), and says whether the grid or the race is running.

use super::super::Audio;
use super::super::hd_mix::{Live, State};

impl Audio {
    /// Chooses this tick's mix state from the race. A no-op on a title with no
    /// authored mix.
    ///
    /// **Two of the original's states are not modelled**: the fly-over's
    /// `PreRace` (this port's race opens on the grid) and the critical-energy
    /// and player-dead rows, which change only a few groups; `RaceNormal` is
    /// used throughout. Chosen, not measured.
    pub(super) fn hd_race_mix(&mut self, race: &crate::race::Race) {
        if self.hd.live.is_none()
            && let Some(maps) = race.sounds().mix.as_deref()
        {
            self.hd.live = Some(Live::new(maps.clone()));
        }
        let gated = oag_race::RaceState::thrust_gated(race.sim.world.tick);
        self.hd.state = Some(if gated {
            State::Countdown
        } else {
            State::RaceNormal
        });
    }
}
